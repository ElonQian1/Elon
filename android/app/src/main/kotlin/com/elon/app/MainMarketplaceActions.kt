package com.elon.app

import android.content.Intent
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
        projects.chunked(2).forEachIndexed { rowIndex, rowProjects ->
            container.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                isBaselineAligned = false
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginStart = dp(CARD_SIDE_MARGIN_DP)
                    marginEnd = dp(CARD_SIDE_MARGIN_DP)
                    topMargin = dp(if (rowIndex == 0) FIRST_CARD_TOP_MARGIN_DP else CARD_GAP_DP)
                }
                rowProjects.forEachIndexed { columnIndex, project ->
                    addView(buildProjectCard(project), LinearLayout.LayoutParams(
                        0,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        1f
                    ).apply {
                        if (columnIndex == 0) marginEnd = dp(CARD_GRID_GAP_DP)
                    })
                }
                if (rowProjects.size == 1) {
                    addView(View(activity), LinearLayout.LayoutParams(0, 1, 1f))
                }
            })
        }
    }

    private fun buildProjectCard(project: StoreProject): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_BODY, CARD_RADIUS_DP)
            clipToOutline = true
            addView(createCardContent(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_CONTENT_HEIGHT_DP)
            ))
            addView(createCardActionBar(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_ACTION_BAR_HEIGHT_DP)
            ))
        }
    }

    private fun createCardContent(project: StoreProject): View {
        val identity = identityFor(project)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_HEADER, 0)
            setPadding(dp(CARD_CONTENT_SIDE_PADDING_DP), dp(16), dp(CARD_CONTENT_SIDE_PADDING_DP), 0)
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = identity.title
                gravity = Gravity.CENTER
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_CARD_TITLE_SP)
                setTypeface(typeface, Typeface.BOLD)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

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
                topMargin = dp(22)
            })

            addView(View(activity).apply {
                setBackgroundColor(Color.parseColor(COLOR_DIVIDER))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(1)
            ).apply {
                topMargin = dp(12)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "简介：${identity.subtitle ?: project.description?.trim()?.takeIf { it.isNotBlank() } ?: "暂无简介"}"
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, FONT_LIST_SECONDARY_SP)
                setLineSpacing(dp(2).toFloat(), 1.0f)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(10)
            })
        }
    }

    private fun createCardActionBar(project: StoreProject): View {
        val alreadyJoined = isProjectJoined(project)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.END or Gravity.CENTER_VERTICAL
            background = rect(COLOR_CARD_BODY, 0)
            setPadding(0, 0, dp(10), 0)
            val entryAction = iconActionButton(
                iconRes = R.drawable.ic_plaza_enter_space,
                description = projectEntryActionLabel(project, alreadyJoined)
            ) { tryEnterProject(project, it) }
            addView(entryAction, iconActionParams())
            addView(iconActionButton(
                iconRes = R.drawable.ic_plaza_share_project,
                description = "分享项目"
            ) { shareProject(project) }, iconActionParams())
            addView(iconActionButton(
                iconRes = R.drawable.ic_plaza_download_apk,
                description = "下载APK",
                enabled = !project.latestApkUrl.isNullOrBlank()
            ) { tryInstallProjectFromIcon(project, it, entryAction) }, iconActionParams())
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

    private fun iconActionButton(
        iconRes: Int,
        description: String,
        enabled: Boolean = true,
        onClick: (View) -> Unit
    ): FrameLayout {
        return FrameLayout(activity).apply {
            contentDescription = description
            isEnabled = enabled
            isClickable = enabled
            alpha = if (enabled) 1f else DISABLED_ACTION_ALPHA
            foreground = selectableForeground()
            setOnClickListener { if (isEnabled) onClick(this) }
            addView(ImageView(activity).apply {
                setImageResource(iconRes)
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = null
            }, FrameLayout.LayoutParams(dp(ACTION_ICON_SIZE_DP), dp(ACTION_ICON_SIZE_DP), Gravity.CENTER))
        }
    }

    private fun iconActionParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(dp(ACTION_ICON_TOUCH_DP), dp(ACTION_ICON_TOUCH_DP)).apply {
            marginStart = dp(ACTION_ICON_GAP_DP)
        }
    }

    private fun setIconButtonBusy(button: View, busy: Boolean) {
        button.isEnabled = !busy
        button.isClickable = !busy
        button.alpha = if (busy) DISABLED_ACTION_ALPHA else 1f
    }

    private fun tryEnterProject(project: StoreProject, actionBtn: View) {
        if (isProjectJoined(project)) {
            openJoinedProject(project)
            return
        }
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity) ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        if (normalizeProjectJoinMode(project.joinMode) == PROJECT_JOIN_MODE_APPROVAL) {
            tryRequestJoinProjectFromIcon(project, actionBtn, token)
            return
        }

        setIconButtonBusy(actionBtn, true)
        thread(name = "join-project") {
            val result = runCatching { joinStoreProject(http, serverUrl, project.id, token) }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        joinedIds.add(project.id)
                        markProjectJoined(project, actionBtn)
                        Toast.makeText(activity, projectJoinSuccessToast(project.joinMode), Toast.LENGTH_SHORT).show()
                        openJoinedProject(project)
                    }
                    .onFailure {
                        setIconButtonBusy(actionBtn, false)
                        Toast.makeText(activity, it.message ?: "加入失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryRequestJoinProjectFromIcon(project: StoreProject, actionBtn: View, token: String) {
        setIconButtonBusy(actionBtn, true)
        thread(name = "request-join-project") {
            val result = runCatching { requestJoinStoreProject(http, serverUrl, project.id, token) }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        actionBtn.alpha = DISABLED_ACTION_ALPHA
                        Toast.makeText(activity, "申请已提交，等待项目管理员审核", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        setIconButtonBusy(actionBtn, false)
                        Toast.makeText(activity, it.message ?: "申请失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryInstallProjectFromIcon(project: StoreProject, installBtn: View, joinBtn: View?) {
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
        setIconButtonBusy(installBtn, true)
        thread(name = "install-store-project") {
            val result = runCatching {
                if (shouldJoin) joinStoreProject(http, serverUrl, project.id, token)
                apkUrl
            }
            activity.runOnUiThread {
                setIconButtonBusy(installBtn, false)
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

    private fun shareProject(project: StoreProject) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, projectShareText(project))
        }
        activity.startActivity(Intent.createChooser(intent, "分享项目"))
    }

    private fun projectShareText(project: StoreProject): String {
        val identity = identityFor(project)
        val lines = mutableListOf(
            "一龙项目：${identity.title}",
            "创建者：${project.ownerAccount.ifBlank { "未知" }}",
            "成员：${project.memberCount.coerceAtLeast(0)}",
            "加入方式：${projectJoinModeSummary(project.joinMode)}"
        )
        val description = identity.subtitle ?: project.description?.trim()?.takeIf { it.isNotBlank() }
        if (!description.isNullOrBlank()) lines += "简介：$description"
        project.latestApkUrl?.trim()?.takeIf { it.isNotBlank() }?.let { lines += "APK：$it" }
        return lines.joinToString("\n")
    }

    private fun isProjectJoined(project: StoreProject): Boolean {
        return !project.viewerRole.isNullOrBlank() || joinedIds.contains(project.id)
    }

    private fun markProjectJoined(project: StoreProject, joinBtn: View) {
        joinBtn.contentDescription = projectEntryActionLabel(project, true)
        setIconButtonBusy(joinBtn, false)
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

    private fun projectEntryActionLabel(project: StoreProject, alreadyJoined: Boolean): String {
        return if (alreadyJoined || normalizeProjectJoinMode(project.joinMode) != PROJECT_JOIN_MODE_APPROVAL) {
            "进入空间"
        } else {
            "申请加入"
        }
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

    private fun filterChipWidthPx(): Int = designPx(SEGMENT_WIDTH_PX)

    private fun filterChipHeightPx(): Int = designPx(SEGMENT_HEIGHT_PX)

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private companion object {
        const val FILTER_ALL = "all"
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
        const val FONT_AVATAR_SP = 24f
        const val FONT_PAGE_TITLE_SP = 16f
        const val FONT_LIST_SECONDARY_SP = 13f
        const val FONT_CARD_TITLE_SP = 18f
        const val SEARCH_HEIGHT_DP = 48
        const val SEARCH_RADIUS_DP = 24
        const val SEARCH_SIDE_MARGIN_DP = 20
        const val DESIGN_WIDTH_PX = 1272
        const val SEGMENT_WIDTH_PX = 210
        const val SEGMENT_HEIGHT_PX = 138
        const val FILTER_SIDE_PADDING_DP = 20
        const val FILTER_ITEM_GAP_DP = 14
        const val CARD_RADIUS_DP = 18
        const val CARD_SIDE_MARGIN_DP = 10
        const val CARD_CONTENT_HEIGHT_DP = 184
        const val CARD_ACTION_BAR_HEIGHT_DP = 56
        const val CARD_GRID_GAP_DP = 10
        const val CARD_CONTENT_SIDE_PADDING_DP = 18
        const val FIRST_CARD_TOP_MARGIN_DP = 18
        const val CARD_GAP_DP = 14
        const val THUMB_SIZE_DP = 40
        const val ACTION_ICON_TOUCH_DP = 48
        const val ACTION_ICON_SIZE_DP = 34
        const val ACTION_ICON_GAP_DP = 0
        const val DISABLED_ACTION_ALPHA = 0.45f
    }
}
