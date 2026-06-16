package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.util.TypedValue
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

internal class MainMarketplaceActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val getListContainer: () -> LinearLayout,
    private val openJoinedProject: (StoreProject) -> Unit = {}
) {
    private data class MarketplaceFilter(
        val key: String,
        val label: String,
        val joinMode: String? = null,
        val hasApk: Boolean? = null,
        val sort: String? = null,
        val joinedOnly: Boolean = false,
        val noApprovalOnly: Boolean = false
    )

    private data class ProjectCardIdentity(
        val title: String,
        val subtitle: String?
    )

    private val joinedIds = mutableSetOf<String>()
    private val filterChipViews = LinkedHashMap<String, TextView>()
    private var resultsContainer: LinearLayout? = null
    private var searchField: EditText? = null
    private var searchDebounce: Runnable? = null
    private var searchQuery = ""
    private var activeFilterKey = FILTER_ALL

    @Volatile
    private var loadSerial = 0

    private val filters = listOf(
        MarketplaceFilter(FILTER_ALL, "全部"),
        MarketplaceFilter("installable", "可安装", hasApk = true),
        MarketplaceFilter("no_approval", "无审批", noApprovalOnly = true),
        MarketplaceFilter("joined", "已加入", joinedOnly = true),
        MarketplaceFilter("popular", "最热门", sort = "members")
    )

    fun loadProjects(search: String? = null) {
        if (search != null) searchQuery = search.trim()
        val serial = ++loadSerial
        renderLoading()
        thread(name = "project-plaza-list") {
            val filter = activeFilter()
            val storeResult = runCatching {
                fetchAllStoreProjects(
                    http = http,
                    serverUrl = serverUrl,
                    search = searchQuery.ifBlank { null },
                    joinMode = filter.joinMode,
                    hasApk = filter.hasApk,
                    sort = filter.sort
                )
            }
            val alreadyJoined = runCatching {
                if (!AuthManager.isLoggedIn(activity)) emptySet()
                else fetchJoinedProjectIds(http, serverUrl, activity)
            }.getOrDefault(emptySet())

            activity.runOnUiThread {
                if (serial != loadSerial) return@runOnUiThread
                joinedIds.clear()
                joinedIds.addAll(alreadyJoined)
                storeResult
                    .onSuccess { renderProjects(applyClientFilter(it, activeFilter())) }
                    .onFailure { renderError(it.message ?: "加载失败") }
            }
        }
    }

    private fun ensureDiscoveryShell(): LinearLayout {
        val container = getListContainer()
        val currentResults = resultsContainer
        if (currentResults != null && currentResults.parent != null) {
            updateFilterChipVisuals()
            return currentResults
        }
        container.removeAllViews()
        filterChipViews.clear()
        val shell = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, dp(28))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        shell.addView(buildSearchPanel())
        val results = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(RESULTS_TRAY_OVERLAP_DP), 0, 0)
            background = topRoundedRect(COLOR_APP_BG, RESULTS_TRAY_RADIUS_DP)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = -dp(RESULTS_TRAY_OVERLAP_DP)
            }
        }
        shell.addView(results)
        container.addView(shell)
        resultsContainer = results
        updateFilterChipVisuals()
        return results
    }

    private fun buildSearchPanel(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_HEADER, 14)
            setPadding(dp(22), dp(20), dp(22), dp(16))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(12)
            }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "搜索"
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
            })
            addView(buildFilterScroller(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(16)
            })
            addView(buildSearchField(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(1)
            ).apply {
                topMargin = dp(1)
            })
        }
    }

    private fun buildSearchField(): EditText {
        return EditText(activity).apply {
            searchField = this
            visibility = View.GONE
            setText(searchQuery)
            setSingleLine(true)
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
                override fun afterTextChanged(s: Editable?) {
                    val next = s?.toString()?.trim().orEmpty()
                    if (next == searchQuery) return
                    searchQuery = next
                    searchDebounce?.let { activity.window.decorView.removeCallbacks(it) }
                    searchDebounce = Runnable { loadProjects(searchQuery) }.also {
                        activity.window.decorView.postDelayed(it, 320)
                    }
                }
            })
        }
    }

    private fun buildFilterScroller(): HorizontalScrollView {
        return HorizontalScrollView(activity).apply {
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                filters.forEach { addView(filterChip(it)) }
            })
        }
    }

    private fun filterChip(option: MarketplaceFilter): TextView {
        return TextView(activity).apply {
            text = option.label
            includeFontPadding = false
            gravity = Gravity.CENTER
            setPadding(0, dp(2), 0, dp(5))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            isClickable = true
            foreground = selectableForeground()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginEnd = dp(34)
            }
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
            chip.setTextColor(Color.parseColor(if (selected) COLOR_TEXT_PRIMARY else COLOR_TEXT_SECONDARY))
            chip.paint.isUnderlineText = selected
            chip.setTypeface(chip.typeface, if (selected) Typeface.BOLD else Typeface.NORMAL)
        }
    }

    private fun renderLoading() {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(centerMessage("加载中...", COLOR_TEXT_TERTIARY))
    }

    private fun renderError(msg: String) {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(centerMessage(msg, "#FF7A7A"))
    }

    private fun renderProjects(projects: List<StoreProject>) {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        if (projects.isEmpty()) {
            container.addView(centerMessage("暂无匹配项目", COLOR_TEXT_SECONDARY))
            return
        }
        projects.forEach { project ->
            container.addView(buildProjectCard(project))
        }
    }

    private fun buildProjectCard(project: StoreProject): LinearLayout {
        val alreadyJoined = joinedIds.contains(project.id)
        val joinBtn = actionButton(projectJoinActionLabel(project.joinMode, alreadyJoined)).apply {
            setOnClickListener {
                if (alreadyJoined) openJoinedProject(project) else tryJoinProject(project, this)
            }
        }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_BODY, 12)
            clipToOutline = true
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(16)
                marginEnd = dp(16)
                topMargin = dp(8)
            }
            addView(createCardHeader(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(54)
            ))
            addView(createCardBody(project, joinBtn), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(184)
            ))
        }
    }

    private fun createCardHeader(project: StoreProject): View {
        val identity = identityFor(project)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            background = topRoundedRect(COLOR_CARD_HEADER, 12)
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = identity.title
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
                setTypeface(typeface, Typeface.BOLD)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(statusLabel(joinApprovalLabel(project.joinMode), approvalDotColor(project.joinMode)))
            addView(statusLabel(if (project.latestApkUrl.isNullOrBlank()) "暂无APK" else "可安装", apkDotColor(project)))
        }
    }

    private fun createCardBody(project: StoreProject, joinBtn: TextView): View {
        return FrameLayout(activity).apply {
            background = rect(COLOR_CARD_BODY, 0)

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    addView(projectThumbnail(project), LinearLayout.LayoutParams(dp(40), dp(40)).apply {
                        marginEnd = dp(14)
                    })
                    addView(LinearLayout(activity).apply {
                        orientation = LinearLayout.VERTICAL
                        addProjectDetailText("创建者：${project.ownerAccount.ifBlank { "未知" }}")
                        addProjectDetailText("成员：${project.memberCount.coerceAtLeast(0)}")
                    }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                })
                addView(View(activity).apply {
                    setBackgroundColor(Color.parseColor(COLOR_DIVIDER))
                    alpha = 0.74f
                }, LinearLayout.LayoutParams(dp(196), dp(1)).apply {
                    topMargin = dp(12)
                })
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "简介：${project.description?.trim()?.takeIf { it.isNotBlank() } ?: "暂无简介"}"
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
                    setLineSpacing(dp(2).toFloat(), 1.0f)
                }, LinearLayout.LayoutParams(dp(206), LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                    topMargin = dp(11)
                })
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(24)
                rightMargin = dp(132)
                topMargin = dp(34)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "时间"
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.END or Gravity.TOP
            ).apply {
                rightMargin = dp(24)
                topMargin = dp(42)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.END or Gravity.CENTER_VERTICAL
                addView(joinBtn, LinearLayout.LayoutParams(dp(92), dp(34)).apply {
                    marginEnd = dp(10)
                })
                addView(actionButton("下载APK").apply {
                    isEnabled = !project.latestApkUrl.isNullOrBlank()
                    alpha = if (isEnabled) 1f else 0.55f
                    setOnClickListener { tryInstallProject(project, this, joinBtn) }
                }, LinearLayout.LayoutParams(dp(92), dp(34)))
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM
            ).apply {
                leftMargin = dp(24)
                rightMargin = dp(24)
                bottomMargin = dp(20)
            })
        }
    }

    private fun LinearLayout.addProjectDetailText(value: String) {
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = value
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            bottomMargin = dp(9)
        })
    }

    private fun projectThumbnail(project: StoreProject): View {
        return FrameLayout(activity).apply {
            background = rect(COLOR_THUMB_BG, 6)
            clipToOutline = true
            val iconBitmap = UserProfileStore.decodeAvatar(project.iconDataUrl)
            if (iconBitmap != null) {
                addView(ImageView(activity).apply {
                    setImageBitmap(iconBitmap)
                    scaleType = ImageView.ScaleType.CENTER_CROP
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            } else {
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    text = avatarText(project.name.ifBlank { "项目" })
                    setTextColor(Color.parseColor("#253140"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
                    setTypeface(typeface, Typeface.BOLD)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun statusLabel(text: String, dotColor: String): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(24)
            }
            addView(TextView(activity).apply {
                includeFontPadding = false
                this.text = text
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            })
            addView(View(activity).apply {
                background = rect(dotColor, 999)
            }, LinearLayout.LayoutParams(dp(6), dp(6)).apply {
                marginStart = dp(6)
            })
        }
    }

    private fun actionButton(text: String): TextView {
        return TextView(activity).apply {
            this.text = text
            includeFontPadding = false
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#101010"))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            setTypeface(typeface, Typeface.BOLD)
            background = rect("#C8C8C8", 999)
            isClickable = true
            foreground = selectableForeground()
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
        }
    }

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
        thread(name = "join-project") {
            val result = runCatching { joinStoreProject(http, serverUrl, project.id, token) }
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
        thread(name = "request-join-project") {
            val result = runCatching { requestJoinStoreProject(http, serverUrl, project.id, token) }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        joinBtn.text = "已申请"
                        Toast.makeText(activity, "申请已提交，等待项目管理员审核", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        joinBtn.isEnabled = true
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
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (!AuthManager.isLoggedIn(activity) || token.isBlank()) {
            Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
            return
        }

        val shouldJoin = !joinedIds.contains(project.id)
        installBtn.isEnabled = false
        installBtn.text = if (shouldJoin) "加入中..." else "准备安装..."
        thread(name = "install-store-project") {
            val result = runCatching {
                if (shouldJoin) joinStoreProject(http, serverUrl, project.id, token)
                apkUrl
            }
            activity.runOnUiThread {
                installBtn.isEnabled = true
                installBtn.text = "下载APK"
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
        joinBtn.text = "进入空间"
        joinBtn.isEnabled = true
        joinBtn.setOnClickListener { openJoinedProject(project) }
    }

    private fun centerMessage(text: String, color: String): TextView {
        return TextView(activity).apply {
            this.text = text
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(color))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
            setPadding(dp(20), dp(58), dp(20), dp(58))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun activeFilter(): MarketplaceFilter {
        return filters.firstOrNull { it.key == activeFilterKey } ?: filters.first()
    }

    private fun applyClientFilter(projects: List<StoreProject>, filter: MarketplaceFilter): List<StoreProject> {
        return projects.filter { project ->
            (!filter.joinedOnly || joinedIds.contains(project.id)) &&
                (!filter.noApprovalOnly || normalizeProjectJoinMode(project.joinMode) != PROJECT_JOIN_MODE_APPROVAL)
        }
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

    private fun joinApprovalLabel(joinMode: String): String {
        return if (normalizeProjectJoinMode(joinMode) == PROJECT_JOIN_MODE_APPROVAL) "需审批" else "无需审批"
    }

    private fun approvalDotColor(joinMode: String): String {
        return if (normalizeProjectJoinMode(joinMode) == PROJECT_JOIN_MODE_APPROVAL) "#F04B4F" else "#58BE6A"
    }

    private fun apkDotColor(project: StoreProject): String {
        return if (project.latestApkUrl.isNullOrBlank()) "#777777" else "#58BE6A"
    }

    private fun rect(color: String, radiusDp: Int = 0): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun topRoundedRect(color: String, radiusDp: Int): GradientDrawable {
        val radius = dp(radiusDp).toFloat()
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            cornerRadii = floatArrayOf(radius, radius, radius, radius, 0f, 0f, 0f, 0f)
        }
    }

    private companion object {
        const val FILTER_ALL = "all"
        const val COLOR_APP_BG = "#101010"
        const val COLOR_CARD_HEADER = "#202024"
        const val COLOR_CARD_BODY = "#2A2A2A"
        const val COLOR_TEXT_PRIMARY = "#D6D6D6"
        const val COLOR_TEXT_SECONDARY = "#A8A8A8"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val COLOR_DIVIDER = "#A8A8A8"
        const val COLOR_THUMB_BG = "#D2D2D2"
        const val RESULTS_TRAY_RADIUS_DP = 18
        const val RESULTS_TRAY_OVERLAP_DP = 16
    }
}
