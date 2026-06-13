package com.elon.app

import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

// StoreProject 和 fetchStoreProjects / joinStoreProject 均来自 MainStoreApi.kt

internal class MainMarketplaceActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val getListContainer: () -> LinearLayout,
    private val openJoinedProject: (StoreProject) -> Unit = {}
) {

    private val joinedIds = mutableSetOf<String>()
    private val avatarCache = HashMap<String, android.graphics.Bitmap>()
    private val filterChipViews = LinkedHashMap<String, TextView>()
    private var resultsContainer: LinearLayout? = null
    private var searchField: EditText? = null
    private var searchDebounce: Runnable? = null
    private var searchQuery = ""
    private var activeFilterKey = FILTER_ALL
    @Volatile
    private var loadSerial = 0

    private data class ProjectCardIdentity(
        val title: String,
        val subtitle: String?
    )

    private data class MarketplaceFilter(
        val key: String,
        val label: String,
        val joinMode: String? = null,
        val hasApk: Boolean? = null,
        val sort: String? = null,
        val joinedOnly: Boolean = false
    )

    private val filters = listOf(
        MarketplaceFilter(FILTER_ALL, "全部"),
        MarketplaceFilter("installable", "可安装 APK", hasApk = true),
        MarketplaceFilter("open", "可直接加入", joinMode = PROJECT_JOIN_MODE_OPEN),
        MarketplaceFilter("readonly", "只读体验", joinMode = PROJECT_JOIN_MODE_READONLY),
        MarketplaceFilter("approval", "需要申请", joinMode = PROJECT_JOIN_MODE_APPROVAL),
        MarketplaceFilter("joined", "我已加入", joinedOnly = true),
        MarketplaceFilter("popular", "成员最多", sort = "members")
    )

    private companion object {
        const val FILTER_ALL = "all"
        const val STORE_PAGE_LIMIT = 50
    }

    // 根据字符串生成固定色相的深色渐变（用作卡片顶部识别色带）
    private val BANNER_PALETTES = arrayOf(
        intArrayOf(0xFF3B4F8A.toInt(), 0xFF2A3A73.toInt()),  // 深蓝紫
        intArrayOf(0xFF5A3070.toInt(), 0xFF3E1F5A.toInt()),  // 深紫
        intArrayOf(0xFF2D6A4A.toInt(), 0xFF1B4A33.toInt()),  // 深绿
        intArrayOf(0xFF7A3535.toInt(), 0xFF5A2020.toInt()),  // 深红
        intArrayOf(0xFF5A4A1A.toInt(), 0xFF3A3010.toInt()),  // 深金
        intArrayOf(0xFF1A5A6A.toInt(), 0xFF0F3A4A.toInt()),  // 深青
        intArrayOf(0xFF6A3A1A.toInt(), 0xFF4A260F.toInt()),  // 深橙
        intArrayOf(0xFF2A4A6A.toInt(), 0xFF1A3050.toInt()),  // 深天蓝
    )

    private fun paletteFor(key: String): IntArray {
        val hash = key.fold(0) { acc, c -> acc * 31 + c.code }
        return BANNER_PALETTES[Math.abs(hash) % BANNER_PALETTES.size]
    }

    private fun identityFor(project: StoreProject): ProjectCardIdentity {
        val name = project.name.trim().ifBlank { "未命名项目" }
        val description = project.description?.trim()?.takeIf { it.isNotBlank() }
        return if (description != null && looksLikeCodeName(name) && description.length <= 24) {
            ProjectCardIdentity(description, "项目代号：$name")
        } else {
            ProjectCardIdentity(name, description)
        }
    }

    private fun looksLikeCodeName(value: String): Boolean {
        if (value.length !in 3..24) return false
        return value.any { it.isLetter() } && value.all { it.isLetterOrDigit() || it == '_' || it == '-' || it == '.' }
    }

    private fun roundedRect(
        color: String,
        radiusDp: Int = 8,
        strokeColor: String? = null
    ): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(radiusDp).toFloat()
            setColor(Color.parseColor(color))
            if (strokeColor != null) setStroke(dp(1), Color.parseColor(strokeColor))
        }
    }

    private fun pill(text: String, textColor: String, bgColor: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 12f
            setTextColor(Color.parseColor(textColor))
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            gravity = Gravity.CENTER
            setPadding(dp(10), dp(5), dp(10), dp(5))
            background = roundedRect(bgColor, 999)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { rightMargin = dp(8) }
        }
    }

    private fun actionButton(text: String, bgColor: String, textColor: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 16f
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(textColor))
            background = roundedRect(bgColor, 8)
            isEnabled = true
            isClickable = true
            foreground = selectableForeground()
        }
    }

    private fun activeFilter(): MarketplaceFilter {
        return filters.firstOrNull { it.key == activeFilterKey } ?: filters.first()
    }

    private fun isFilterActive(): Boolean {
        return activeFilterKey != FILTER_ALL || searchQuery.isNotBlank()
    }

    private fun ensureDiscoveryShell(): LinearLayout {
        val container = getListContainer()
        val currentResults = resultsContainer
        if (currentResults != null && currentResults.parent === container) {
            updateFilterChipVisuals()
            return currentResults
        }
        container.removeAllViews()
        filterChipViews.clear()
        val shell = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(12), dp(12), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        shell.addView(buildSearchBox())
        shell.addView(buildFilterScroller())
        container.addView(shell)
        updateFilterChipVisuals()
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            container.addView(this)
            resultsContainer = this
        }
    }

    private fun buildSearchBox(): LinearLayout {
        val input = EditText(activity).apply {
            setText(searchQuery)
            setSingleLine(true)
            textSize = 15f
            hint = "搜索项目、功能或创建者"
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#777777"))
            background = null
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            setPadding(0, 0, dp(8), 0)
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
                override fun afterTextChanged(s: Editable?) {
                    val next = s?.toString()?.trim().orEmpty()
                    if (next == searchQuery) return
                    searchQuery = next
                    searchDebounce?.let { activity.window.decorView.removeCallbacks(it) }
                    searchDebounce = Runnable { loadProjects(searchQuery) }.also {
                        activity.window.decorView.postDelayed(it, 360)
                    }
                }
            })
            setOnEditorActionListener { _, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_SEARCH) {
                    searchDebounce?.let { activity.window.decorView.removeCallbacks(it) }
                    loadProjects(searchQuery)
                    true
                } else {
                    false
                }
            }
        }
        searchField = input
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = roundedRect("#222222", 8, "#2E2E2E")
            setPadding(dp(12), 0, dp(8), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(46)
            )
            addView(TextView(activity).apply {
                text = "搜索"
                textSize = 13f
                setTextColor(Color.parseColor("#8DDC9B"))
                gravity = Gravity.CENTER
            }, LinearLayout.LayoutParams(dp(44), LinearLayout.LayoutParams.MATCH_PARENT))
            addView(input, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(TextView(activity).apply {
                text = "清除"
                textSize = 13f
                setTextColor(Color.parseColor("#A8A8A8"))
                gravity = Gravity.CENTER
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { clearDiscoveryFilters() }
            }, LinearLayout.LayoutParams(dp(46), LinearLayout.LayoutParams.MATCH_PARENT))
        }
    }

    private fun buildFilterScroller(): HorizontalScrollView {
        return HorizontalScrollView(activity).apply {
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(10) }
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                setPadding(0, 0, dp(4), 0)
                filters.forEach { option ->
                    addView(filterChip(option))
                }
            })
        }
    }

    private fun filterChip(option: MarketplaceFilter): TextView {
        return TextView(activity).apply {
            text = option.label
            textSize = 13f
            gravity = Gravity.CENTER
            setPadding(dp(12), dp(7), dp(12), dp(7))
            isClickable = true
            foreground = selectableForeground()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { rightMargin = dp(8) }
            setOnClickListener {
                if (activeFilterKey == option.key) return@setOnClickListener
                activeFilterKey = option.key
                updateFilterChipVisuals()
                loadProjects(searchQuery)
            }
            filterChipViews[option.key] = this
        }
    }

    private fun updateFilterChipVisuals() {
        filterChipViews.forEach { (key, chip) ->
            val selected = key == activeFilterKey
            chip.setTextColor(Color.parseColor(if (selected) "#D6D6D6" else "#A8A8A8"))
            chip.background = roundedRect(
                if (selected) "#16251A" else "#222222",
                999,
                if (selected) "#58BE6A" else "#2A2A2A"
            )
        }
    }

    private fun clearDiscoveryFilters() {
        val hadState = isFilterActive()
        searchQuery = ""
        activeFilterKey = FILTER_ALL
        searchField?.setText("")
        updateFilterChipVisuals()
        if (hadState) loadProjects("")
    }

    // ─── 加载公开项目列表 ─────────────────────────────────────────────────────

    fun loadProjects(search: String? = null) {
        if (search != null) searchQuery = search.trim()
        val serial = ++loadSerial
        renderLoading()
        thread {
            val filter = activeFilter()
            val storeResult = runCatching {
                fetchStoreProjects(
                    http = http,
                    serverUrl = serverUrl,
                    search = searchQuery.ifBlank { null },
                    limit = STORE_PAGE_LIMIT,
                    joinMode = filter.joinMode,
                    hasApk = filter.hasApk,
                    sort = filter.sort
                )
            }
            val alreadyJoined: Set<String> = runCatching {
                if (!AuthManager.isLoggedIn(activity)) emptySet<String>()
                else fetchJoinedProjectIds(http, serverUrl, activity)
            }.getOrDefault(emptySet<String>())

            activity.runOnUiThread {
                if (serial != loadSerial) return@runOnUiThread
                joinedIds.clear()
                joinedIds.addAll(alreadyJoined)
                storeResult
                    .onSuccess { projects ->
                        renderProjects(if (filter.joinedOnly) projects.filter { joinedIds.contains(it.id) } else projects)
                    }
                    .onFailure { renderError(it.message ?: "加载失败") }
            }
        }
    }

    // ─── 加入项目 ─────────────────────────────────────────────────────────────

    private fun tryJoinProject(project: StoreProject, joinBtn: TextView) {
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity) ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        if (normalizeProjectJoinMode(project.joinMode) == PROJECT_JOIN_MODE_APPROVAL) {
            tryRequestJoinProject(project, joinBtn, token)
            return
        }
        joinBtn.isEnabled = false
        joinBtn.text = "加入中..."
        thread {
            val result = runCatching {
                joinStoreProject(http, serverUrl, project.id, token)
            }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        joinedIds.add(project.id)
                        markProjectJoined(project, joinBtn)
                        Toast.makeText(activity, projectJoinSuccessToast(project.joinMode), Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        joinBtn.isEnabled = true
                        joinBtn.text = projectJoinActionLabel(project.joinMode)
                        Toast.makeText(activity, it.message ?: "加入失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryRequestJoinProject(project: StoreProject, joinBtn: TextView, token: String) {
        joinBtn.isEnabled = false
        joinBtn.text = "申请中..."
        thread {
            val result = runCatching {
                requestJoinStoreProject(http, serverUrl, project.id, token)
            }
            activity.runOnUiThread {
                joinBtn.isEnabled = true
                joinBtn.text = "已申请"
                result
                    .onSuccess {
                        joinBtn.isEnabled = false
                        Toast.makeText(activity, "申请已提交，等待项目管理员审核", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        joinBtn.text = projectJoinActionLabel(project.joinMode)
                        Toast.makeText(activity, it.message ?: "申请失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryInstallProject(project: StoreProject, installBtn: TextView, joinBtn: TextView?) {
        if (!isAndroidApkInstallSupported()) {
            Toast.makeText(activity, "当前设备不是 Android，无法直接安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        val apkUrl = project.latestApkUrl?.trim().orEmpty()
        if (apkUrl.isBlank()) {
            Toast.makeText(activity, "这个项目还没有可安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (token.isBlank()) {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }

        val shouldJoin = !joinedIds.contains(project.id)
        installBtn.isEnabled = false
        installBtn.text = if (shouldJoin) "加入中..." else "准备安装..."
        thread {
            val result = runCatching {
                if (shouldJoin) joinStoreProject(http, serverUrl, project.id, token)
                apkUrl
            }
            activity.runOnUiThread {
                installBtn.isEnabled = true
                installBtn.text = "直接安装"
                result
                    .onSuccess { url ->
                        if (shouldJoin) {
                            joinedIds.add(project.id)
                            joinBtn?.let { markProjectJoined(project, it) }
                        }
                        openProjectApkInstall(activity, url, token)
                    }
                    .onFailure {
                        Toast.makeText(activity, it.message ?: "安装失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun markProjectJoined(project: StoreProject, joinBtn: TextView) {
        joinBtn.text = "进入项目"
        joinBtn.isEnabled = true
        joinBtn.setTextColor(Color.parseColor("#101010"))
        (joinBtn.background as? GradientDrawable)?.setColor(Color.parseColor("#C8C8C8"))
        joinBtn.setOnClickListener { openJoinedProject(project) }
    }

    // ─── 渲染 ─────────────────────────────────────────────────────────────────

    private fun renderLoading() {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = "加载中..."
            textSize = 14f
            setTextColor(Color.parseColor("#777777"))
            gravity = Gravity.CENTER
            setPadding(0, dp(60), 0, dp(60))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
    }

    private fun renderError(msg: String) {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = msg
            textSize = 14f
            setTextColor(Color.parseColor("#FF7A7A"))
            gravity = Gravity.CENTER
            setPadding(dp(20), dp(60), dp(20), dp(60))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
    }

    private fun renderProjects(projects: List<StoreProject>) {
        val container = ensureDiscoveryShell()
        container.removeAllViews()

        if (projects.isEmpty()) {
            container.addView(emptyResultView())
            return
        }

        // 顶部标题栏
        container.addView(TextView(activity).apply {
            text = resultSummary(projects.size)
            textSize = 12f
            setTextColor(Color.parseColor("#777777"))
            setPadding(dp(16), dp(16), dp(16), dp(8))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })

        for (project in projects) {
            container.addView(buildProjectCard(project))
        }
    }

    private fun resultSummary(count: Int): String {
        val filter = activeFilter()
        val parts = mutableListOf("公开项目广场", "${count} 个项目")
        if (filter.key != FILTER_ALL) parts.add(filter.label)
        if (searchQuery.isNotBlank()) parts.add("“$searchQuery”")
        return parts.joinToString(" · ")
    }

    private fun emptyResultView(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(dp(24), dp(54), dp(24), dp(54))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            addView(TextView(activity).apply {
                text = if (isFilterActive()) "没有找到匹配的项目" else "暂无公开项目"
                textSize = 15f
                setTextColor(Color.parseColor("#A8A8A8"))
                gravity = Gravity.CENTER
            })
            if (isFilterActive()) {
                addView(TextView(activity).apply {
                    text = "清除筛选"
                    textSize = 14f
                    setTextColor(Color.parseColor("#D6D6D6"))
                    gravity = Gravity.CENTER
                    background = roundedRect("#2A2A2A", 8)
                    isClickable = true
                    foreground = selectableForeground()
                    setOnClickListener { clearDiscoveryFilters() }
                    layoutParams = LinearLayout.LayoutParams(dp(112), dp(40)).apply {
                        topMargin = dp(16)
                    }
                })
            }
        }
    }

    // ─── Discord 风格卡片 ─────────────────────────────────────────────────────

    private fun buildProjectCard(project: StoreProject): LinearLayout {
        val alreadyJoined = joinedIds.contains(project.id)
        val palette = paletteFor(project.id)
        val identity = identityFor(project)

        // 外层卡片容器（圆角 + 深色背景）
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = roundedRect("#222222", 8, "#2E2E2E")
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.setMargins(dp(12), dp(12), dp(12), dp(6))
            layoutParams = lp
            clipToOutline = true
        }

        // 顶部识别色带只负责区分卡片，不再抢占项目信息的主视觉。
        card.addView(FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(6)
            )
            background = GradientDrawable(
                GradientDrawable.Orientation.LEFT_RIGHT,
                palette
            )
        })

        // ── 卡片内容区 ────────────────────────────────────────────────────────
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(14), dp(16), dp(16))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        val headerRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        // 圆形头像容器：文字头像 + 真实头像图层叠加
        val avatarSize = dp(54)
        val avatarFrame = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(avatarSize, avatarSize).apply {
                rightMargin = dp(12)
            }
        }
        val avatarText = TextView(activity).apply {
            text = identity.title.firstOrNull()?.uppercaseChar()?.toString() ?: "P"
            textSize = 21f
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            setTextColor(Color.parseColor("#D6D6D6"))
            gravity = Gravity.CENTER
            background = GradientDrawable(
                GradientDrawable.Orientation.TL_BR,
                palette
            ).apply {
                shape = GradientDrawable.OVAL
                setStroke(dp(1), Color.parseColor("#2A2A2A"))
            }
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
        }
        avatarFrame.addView(avatarText)
        val avatarImg = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
            scaleType = ImageView.ScaleType.CENTER_CROP
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.TRANSPARENT)
            }
            clipToOutline = true
            visibility = android.view.View.GONE
        }
        avatarFrame.addView(avatarImg)
        val projectIcon = UserProfileStore.decodeAvatar(project.iconDataUrl)
        if (projectIcon != null) {
            avatarImg.setImageBitmap(projectIcon)
            avatarImg.visibility = android.view.View.VISIBLE
        } else if (project.ownerUserId.isNotBlank()) {
            loadAvatarAsync(project.ownerUserId, avatarImg)
        }
        headerRow.addView(avatarFrame)

        val titleColumn = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        titleColumn.addView(TextView(activity).apply {
            text = identity.title
            textSize = 21f
            setTextColor(Color.parseColor("#D6D6D6"))
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            maxLines = 2
            setLineSpacing(dp(1).toFloat(), 1.0f)
            ellipsize = android.text.TextUtils.TruncateAt.END
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
        identity.subtitle?.let { subtitle ->
            titleColumn.addView(TextView(activity).apply {
                text = subtitle
                textSize = 13f
                setTextColor(Color.parseColor("#A8A8A8"))
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(4) }
            })
        }
        headerRow.addView(titleColumn)
        body.addView(headerRow)

        // 在线人数 & 作者行
        val pillRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(12) }
        }
        pillRow.addView(pill("\u25CF  ${project.memberCount} 位成员", "#58BE6A", "#13251A"))
        pillRow.addView(pill(projectJoinModeSummary(project.joinMode), "#8DDC9B", "#16251A"))
        if (!project.latestApkUrl.isNullOrBlank()) {
            pillRow.addView(pill("可安装 APK", "#D6D6D6", "#2A2A2A"))
        }
        body.addView(pillRow)

        // 作者
        val owner = project.ownerAccount.takeIf { it != "?" && it.isNotBlank() }
        if (owner != null) {
            body.addView(TextView(activity).apply {
                text = "创建者：$owner"
                textSize = 12f
                setTextColor(Color.parseColor("#777777"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(8) }
            })
        }

        // ── 加入按钮（全宽，银灰主操作）─────────────────────────────────────
        val joinBtn = actionButton(
            projectJoinActionLabel(project.joinMode, alreadyJoined),
            "#C8C8C8",
            "#101010"
        ).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(46)
            ).apply { topMargin = dp(16) }
        }

        if (alreadyJoined) {
            joinBtn.setOnClickListener { openJoinedProject(project) }
        } else {
            joinBtn.setOnClickListener { tryJoinProject(project, joinBtn) }
        }
        if (isAndroidApkInstallSupported() && !project.latestApkUrl.isNullOrBlank()) {
            val actionRow = LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(46)
                ).apply { topMargin = dp(16) }
            }
            joinBtn.layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            ).apply { rightMargin = dp(10) }

            val installBtn = actionButton("直接安装", "#C8C8C8", "#101010").apply {
                setOnClickListener { tryInstallProject(project, this, joinBtn) }
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            }

            actionRow.addView(joinBtn)
            actionRow.addView(installBtn)
            body.addView(actionRow)
        } else {
            body.addView(joinBtn)
        }

        card.addView(body)
        return card
    }

    // ─── 异步加载头像 ─────────────────────────────────────────────────────────

    private fun loadAvatarAsync(ownerUserId: String, imageView: ImageView) {
        val cached = avatarCache[ownerUserId]
        if (cached != null) {
            imageView.setImageBitmap(cached)
            imageView.visibility = android.view.View.VISIBLE
            return
        }
        thread(name = "avatar-$ownerUserId") {
            val result = runCatching {
                val req = okhttp3.Request.Builder()
                    .url("$serverUrl/api/users/$ownerUserId/avatar")
                    .get()
                    .build()
                val resp = http.newCall(req).execute()
                if (!resp.isSuccessful) return@runCatching null
                resp.body?.byteStream()?.let { BitmapFactory.decodeStream(it) }
            }
            val bitmap = result.getOrNull()
            if (bitmap != null) {
                avatarCache[ownerUserId] = bitmap
                activity.runOnUiThread {
                    imageView.setImageBitmap(bitmap)
                    imageView.visibility = android.view.View.VISIBLE
                }
            }
        }
    }
}
