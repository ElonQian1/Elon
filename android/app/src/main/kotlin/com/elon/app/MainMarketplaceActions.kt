package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.InputType
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
import kotlin.math.roundToInt

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
                    sort = filter.sort,
                    ctx = activity
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
            setPadding(0, 0, 0, dp(32))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        shell.addView(buildSearchBar())
        shell.addView(buildFilterScroller(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            filterChipHeightPx()
        ).apply {
            topMargin = dp(20)
        })
        val results = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        shell.addView(results)
        container.addView(shell)
        resultsContainer = results
        updateFilterChipVisuals()
        return results
    }

    private fun buildSearchBar(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = rect(COLOR_SEARCH_BG, SEARCH_RADIUS_DP)
            setPadding(dp(20), 0, dp(18), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(SEARCH_HEIGHT_DP)
            ).apply {
                marginStart = dp(SEARCH_SIDE_MARGIN_DP)
                marginEnd = dp(SEARCH_SIDE_MARGIN_DP)
                topMargin = dp(16)
            }
            addView(ImageView(activity).apply {
                setImageResource(R.drawable.ic_search_simple)
                setColorFilter(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(24), dp(24)).apply {
                marginEnd = dp(12)
            })
            addView(buildSearchField(), LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            ))
        }
    }

    private fun buildSearchField(): EditText {
        return EditText(activity).apply {
            searchField = this
            background = null
            hint = "搜索应用"
            setText(searchQuery)
            setSingleLine(true)
            inputType = InputType.TYPE_CLASS_TEXT
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            includeFontPadding = false
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, 0, 0, 0)
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_PAGE_TITLE_SP)
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
            clipToPadding = false
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(dp(FILTER_SIDE_PADDING_DP), 0, dp(FILTER_SIDE_PADDING_DP), 0)
                filters.forEach { addView(filterChip(it)) }
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun filterChip(option: MarketplaceFilter): TextView {
        return TextView(activity).apply {
            text = option.label
            includeFontPadding = false
            gravity = Gravity.CENTER
            minWidth = 0
            minHeight = 0
            setPadding(0, 0, 0, 0)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_PAGE_TITLE_SP)
            isClickable = true
            foreground = selectableForeground()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                filterChipHeightPx()
            ).apply {
                marginEnd = dp(FILTER_ITEM_GAP_DP)
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
            chip.setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            chip.paint.isUnderlineText = false
            chip.setTypeface(chip.typeface, Typeface.NORMAL)
            chip.background = if (selected) {
                roundedPx(COLOR_SEGMENT_SELECTED, SEGMENT_HEIGHT_PX / 2)
            } else {
                null
            }
            (chip.layoutParams as? LinearLayout.LayoutParams)?.let { params ->
                params.width = if (selected) filterChipWidthPx() else LinearLayout.LayoutParams.WRAP_CONTENT
                params.height = filterChipHeightPx()
                chip.layoutParams = params
            }
            chip.minWidth = if (selected) filterChipWidthPx() else 0
            chip.setPadding(0, 0, 0, 0)
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
        projects.forEachIndexed { index, project ->
            container.addView(buildProjectCard(project, index))
        }
    }

    private fun buildProjectCard(project: StoreProject, index: Int): LinearLayout {
        val alreadyJoined = isProjectJoined(project)
        val joinBtn = actionButton(projectEntryActionLabel(project, alreadyJoined)).apply {
            setOnClickListener {
                if (alreadyJoined) openJoinedProject(project) else tryJoinProject(project, this)
            }
        }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_BODY, CARD_RADIUS_DP)
            clipToOutline = true
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(16)
                marginEnd = dp(16)
                topMargin = dp(if (index == 0) FIRST_CARD_TOP_MARGIN_DP else CARD_GAP_DP)
            }
            addView(createCardHeader(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_HEADER_HEIGHT_DP)
            ))
            addView(createCardBody(project, joinBtn), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_BODY_HEIGHT_DP)
            ))
        }
    }

    private fun createCardHeader(project: StoreProject): View {
        val identity = identityFor(project)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            background = topRoundedRect(COLOR_CARD_HEADER, CARD_RADIUS_DP)
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = identity.title
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_PAGE_TITLE_SP)
                setTypeface(typeface, Typeface.BOLD)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(statusLabel(joinApprovalLabel(project.joinMode), approvalDotColor(project.joinMode)))
            addView(statusLabel(apkStatusLabel(project), apkDotColor(project)))
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
                    addView(projectThumbnail(project), LinearLayout.LayoutParams(dp(THUMB_SIZE_DP), dp(THUMB_SIZE_DP)).apply {
                        marginEnd = dp(14)
                    })
                    addView(LinearLayout(activity).apply {
                        orientation = LinearLayout.VERTICAL
                        addProjectDetailText("创建者：${project.ownerAccount.ifBlank { "未知" }}")
                        addProjectDetailText("成员：${project.memberCount.coerceAtLeast(0)}")
                    }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    rightMargin = dp(INFO_ROW_RIGHT_MARGIN_DP)
                })
                addView(View(activity).apply {
                    setBackgroundColor(Color.parseColor(COLOR_DIVIDER))
                }, LinearLayout.LayoutParams(dp(DIVIDER_WIDTH_DP), dp(1)).apply {
                    topMargin = dp(10)
                })
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "简介：${project.description?.trim()?.takeIf { it.isNotBlank() } ?: "暂无简介"}"
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
                    setLineSpacing(dp(2).toFloat(), 1.0f)
                }, LinearLayout.LayoutParams(dp(DESC_WIDTH_DP), LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                    topMargin = dp(8)
                })
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(24)
                rightMargin = dp(CARD_MAIN_RIGHT_MARGIN_DP)
                topMargin = dp(CARD_BODY_CONTENT_TOP_DP)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "时间"
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_META_SP)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.END or Gravity.TOP
            ).apply {
                rightMargin = dp(24)
                topMargin = dp(CARD_TIME_TOP_DP)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.END or Gravity.CENTER_VERTICAL
                addView(joinBtn, LinearLayout.LayoutParams(dp(ACTION_BUTTON_WIDTH_DP), dp(ACTION_BUTTON_HEIGHT_DP)).apply {
                    marginEnd = dp(ACTION_BUTTON_GAP_DP)
                })
                addView(actionButton(projectPlazaApkActionLabel(project)).apply {
                    val hasApk = !project.latestApkUrl.isNullOrBlank()
                    val hasInstalledApp = isProjectAppInstalled(activity, project.id, project.name)
                    isEnabled = hasApk || hasInstalledApp
                    alpha = if (isEnabled) 1f else 0.55f
                    setOnClickListener { tryInstallProject(project, this, joinBtn) }
                }, LinearLayout.LayoutParams(dp(ACTION_BUTTON_WIDTH_DP), dp(ACTION_BUTTON_HEIGHT_DP)))
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM
            ).apply {
                leftMargin = dp(24)
                rightMargin = dp(24)
                bottomMargin = dp(ACTION_BUTTON_BOTTOM_DP)
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
            bottomMargin = dp(8)
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
                    text = avatarText(project.displayTitle())
                    setTextColor(Color.parseColor("#253140"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_AVATAR_SP)
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
                marginStart = dp(18)
            }
            addView(TextView(activity).apply {
                includeFontPadding = false
                this.text = text
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_STATUS_SP)
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
            setTextColor(Color.parseColor(COLOR_BUTTON_TEXT))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_LIST_SECONDARY_SP)
            setTypeface(typeface, Typeface.BOLD)
            background = rect(COLOR_BUTTON_BG, 999)
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
                        joinBtn.text = projectEntryActionLabel(project, false)
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
                        joinBtn.text = projectEntryActionLabel(project, false)
                        Toast.makeText(activity, it.message ?: "申请失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryInstallProject(project: StoreProject, installBtn: TextView, joinBtn: TextView?) {
        if (openInstalledProjectApp(activity, project.id, project.name)) {
            return
        }
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

        val shouldJoin = !isProjectJoined(project)
        installBtn.isEnabled = false
        installBtn.text = if (shouldJoin) "加入中..." else "准备安装..."
        thread(name = "install-store-project") {
            val result = runCatching {
                if (shouldJoin) joinStoreProject(http, serverUrl, project.id, token)
                apkUrl
            }
            activity.runOnUiThread {
                installBtn.isEnabled = true
                installBtn.text = projectPlazaApkActionLabel(project)
                result
                    .onSuccess { url ->
                        if (shouldJoin) {
                            joinedIds.add(project.id)
                            joinBtn?.let { markProjectJoined(project, it) }
                        }
                        openProjectApkInstall(activity, url, token, project.id, project.name, http)
                    }
                    .onFailure {
                        Toast.makeText(activity, it.message ?: "安装失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun isProjectJoined(project: StoreProject): Boolean {
        return !project.viewerRole.isNullOrBlank() || joinedIds.contains(project.id)
    }

    private fun markProjectJoined(project: StoreProject, joinBtn: TextView) {
        joinBtn.text = projectEntryActionLabel(project, true)
        joinBtn.isEnabled = true
        joinBtn.setOnClickListener { openJoinedProject(project) }
    }

    private fun centerMessage(text: String, color: String): TextView {
        return TextView(activity).apply {
            this.text = text
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(color))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_PAGE_TITLE_SP)
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
            (!filter.joinedOnly || isProjectJoined(project)) &&
                (!filter.noApprovalOnly || normalizeProjectJoinMode(project.joinMode) != PROJECT_JOIN_MODE_APPROVAL)
        }
    }

    private fun identityFor(project: StoreProject): ProjectCardIdentity {
        val name = project.name.trim()
        val title = project.displayTitle()
        val description = project.description?.trim()?.takeIf { it.isNotBlank() }
        return if (project.hasDisplayAlias()) {
            ProjectCardIdentity(title, description)
        } else if (description != null && looksLikeCodeName(name) && description.length <= 24) {
            ProjectCardIdentity(description, "项目代号：$name")
        } else {
            ProjectCardIdentity(title, description)
        }
    }

    private fun looksLikeCodeName(value: String): Boolean {
        if (value.length !in 3..24) return false
        return value.any { isAsciiLetter(it) } && value.all { isAsciiCodeNameChar(it) }
    }

    private fun isAsciiCodeNameChar(value: Char): Boolean {
        return isAsciiLetter(value) || value in '0'..'9' || value == '_' || value == '-' || value == '.'
    }

    private fun isAsciiLetter(value: Char): Boolean {
        return value in 'A'..'Z' || value in 'a'..'z'
    }

    private fun joinApprovalLabel(joinMode: String): String {
        return if (normalizeProjectJoinMode(joinMode) == PROJECT_JOIN_MODE_APPROVAL) "需审批" else "无需审批"
    }

    private fun approvalDotColor(joinMode: String): String {
        return if (normalizeProjectJoinMode(joinMode) == PROJECT_JOIN_MODE_APPROVAL) COLOR_STATUS_DANGER else COLOR_STATUS_SUCCESS
    }

    private fun apkDotColor(project: StoreProject): String {
        return if (project.latestApkUrl.isNullOrBlank() &&
            !isProjectAppInstalled(activity, project.id, project.name)
        ) COLOR_TEXT_TERTIARY else COLOR_STATUS_SUCCESS
    }

    private fun apkStatusLabel(project: StoreProject): String {
        return when {
            isProjectAppInstalled(activity, project.id, project.name) -> "已安装"
            project.latestApkUrl.isNullOrBlank() -> "暂无APK"
            else -> "可安装"
        }
    }

    private fun projectEntryActionLabel(project: StoreProject, alreadyJoined: Boolean): String {
        return if (alreadyJoined || normalizeProjectJoinMode(project.joinMode) != PROJECT_JOIN_MODE_APPROVAL) {
            "进入空间"
        } else {
            "申请加入"
        }
    }

    private fun projectPlazaApkActionLabel(project: StoreProject): String {
        return if (isProjectAppInstalled(activity, project.id, project.name)) "打开应用" else "下载APK"
    }

    private fun rect(color: String, radiusDp: Int = 0): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun roundedPx(color: String, radiusPx: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            cornerRadius = designPx(radiusPx).toFloat()
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

    private fun filterChipWidthPx(): Int = designPx(SEGMENT_WIDTH_PX)

    private fun filterChipHeightPx(): Int = designPx(SEGMENT_HEIGHT_PX)

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private companion object {
        const val FILTER_ALL = "all"
        const val COLOR_APP_BG = "#000000"
        const val COLOR_CARD_HEADER = "#1F2023"
        const val COLOR_CARD_BODY = "#1A1A1A"
        const val COLOR_SEARCH_BG = "#272727"
        const val COLOR_SEGMENT_SELECTED = "#1A1A1A"
        const val COLOR_TEXT_PRIMARY = "#D9D9D9"
        const val COLOR_TEXT_SECONDARY = "#B8B8B8"
        const val COLOR_TEXT_PLACEHOLDER = "#AFAFAF"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val COLOR_DIVIDER = "#6D6E6F"
        const val COLOR_THUMB_BG = "#FFFFFF"
        const val COLOR_BUTTON_BG = "#FFFFFF"
        const val COLOR_BUTTON_TEXT = "#000000"
        const val COLOR_STATUS_SUCCESS = "#58BE6A"
        const val COLOR_STATUS_DANGER = "#E62129"
        const val FONT_AVATAR_SP = 24f
        const val FONT_PAGE_TITLE_SP = 16f
        const val FONT_STATUS_SP = 15f
        const val FONT_LIST_SECONDARY_SP = 13f
        const val FONT_META_SP = 12f
        const val SEARCH_HEIGHT_DP = 48
        const val SEARCH_RADIUS_DP = 24
        const val SEARCH_SIDE_MARGIN_DP = 20
        const val DESIGN_WIDTH_PX = 1272
        const val SEGMENT_WIDTH_PX = 210
        const val SEGMENT_HEIGHT_PX = 138
        const val FILTER_SIDE_PADDING_DP = 20
        const val FILTER_ITEM_GAP_DP = 14
        const val CARD_RADIUS_DP = 18
        const val CARD_HEADER_HEIGHT_DP = 44
        const val CARD_BODY_HEIGHT_DP = 160
        const val FIRST_CARD_TOP_MARGIN_DP = 18
        const val CARD_GAP_DP = 10
        const val THUMB_SIZE_DP = 40
        const val DIVIDER_WIDTH_DP = 220
        const val DESC_WIDTH_DP = 240
        const val CARD_BODY_CONTENT_TOP_DP = 14
        const val CARD_TIME_TOP_DP = 34
        const val CARD_MAIN_RIGHT_MARGIN_DP = 24
        const val INFO_ROW_RIGHT_MARGIN_DP = 96
        const val ACTION_BUTTON_WIDTH_DP = 68
        const val ACTION_BUTTON_HEIGHT_DP = 32
        const val ACTION_BUTTON_GAP_DP = 10
        const val ACTION_BUTTON_BOTTOM_DP = 10
    }
}
