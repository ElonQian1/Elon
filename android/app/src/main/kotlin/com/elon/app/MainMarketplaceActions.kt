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
    private val openProjectSpace: (StoreProject) -> Unit = {}
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

    private val joinedIds = mutableSetOf<String>()
    private val reactionPrefs by lazy { activity.getSharedPreferences("project_plaza_reactions", 0) }
    private val feedbackSection by lazy {
        ProjectPlazaFeedbackSection(activity, dp, selectableForeground)
    }
    private val membershipActions by lazy {
        ProjectPlazaMembershipActionController(
            activity = activity,
            http = http,
            serverUrl = serverUrl,
            isJoined = ::isProjectJoined,
            onJoined = { project -> joinedIds.add(project.id) },
            onStateChanged = ::rerenderCurrentProjects
        )
    }
    private val featuredSection by lazy {
        ProjectPlazaFeaturedSection(
            activity = activity,
            dp = dp,
            selectableForeground = selectableForeground,
            reactionPrefs = reactionPrefs,
            openProjectSpace = openProjectSpace,
            isProjectJoined = ::isProjectJoined,
            primaryAction = membershipActions::presentation,
            onPrimaryAction = { project -> membershipActions.handle(project, openProjectSpace) }
        )
    }
    private val filterChipViews = LinkedHashMap<String, TextView>()
    private var resultsContainer: LinearLayout? = null
    private var searchField: EditText? = null
    private var searchDebounce: Runnable? = null
    private var searchQuery = ""
    private var activeFilterKey = FILTER_ALL
    private var currentProjects: List<StoreProject> = emptyList()
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
            setPadding(0, 0, 0, 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        shell.addView(buildSearchBar())
        shell.addView(buildFilterScroller(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(48)
        ).apply { topMargin = dp(8) })
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
                marginStart = dp(PLAZA_SIDE_MARGIN_DP)
                marginEnd = dp(PLAZA_SIDE_MARGIN_DP)
                topMargin = dp(16)
            }
            addView(ImageView(activity).apply {
                setImageResource(R.drawable.ic_top_search_custom)
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
            hint = "搜索项目、作者"
            setText(searchQuery)
            setSingleLine(true)
            inputType = InputType.TYPE_CLASS_TEXT
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            includeFontPadding = false
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, 0, 0, 0)
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
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
                dp(FILTER_CHIP_HEIGHT_DP)
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
                rect(COLOR_SEGMENT_SELECTED, FILTER_CHIP_RADIUS_DP)
            } else {
                null
            }
            (chip.layoutParams as? LinearLayout.LayoutParams)?.let { params ->
                params.width = if (selected) dp(FILTER_CHIP_SELECTED_WIDTH_DP) else LinearLayout.LayoutParams.WRAP_CONTENT
                params.height = dp(FILTER_CHIP_HEIGHT_DP)
                chip.layoutParams = params
            }
            chip.minWidth = if (selected) dp(FILTER_CHIP_SELECTED_WIDTH_DP) else 0
            chip.setPadding(0, 0, 0, 0)
        }
    }

    private fun renderLoading() {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(feedbackSection.buildLoading())
    }

    private fun renderError(msg: String) {
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        container.addView(feedbackSection.buildError(msg) { loadProjects(searchQuery) })
    }

    private fun renderProjects(projects: List<StoreProject>) {
        currentProjects = projects
        val container = ensureDiscoveryShell()
        container.removeAllViews()
        if (projects.isEmpty()) {
            val hasActiveCriteria = searchQuery.isNotBlank() || activeFilterKey != FILTER_ALL
            container.addView(feedbackSection.buildEmpty(
                actionLabel = if (hasActiveCriteria) "清除筛选" else "刷新项目",
                onAction = ::resetDiscovery
            ))
            return
        }
        container.addView(featuredSection.build(projects.take(5)), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            featuredSection.heightPx()
        ).apply { topMargin = dp(4) })
        container.addView(buildResultsHeading(projects), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(PLAZA_SIDE_MARGIN_DP)
            marginEnd = dp(PLAZA_SIDE_MARGIN_DP)
            topMargin = dp(14)
        })
        projects.forEachIndexed { index, project ->
            container.addView(buildProjectListRow(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(PLAZA_SIDE_MARGIN_DP)
                marginEnd = dp(PLAZA_SIDE_MARGIN_DP)
                topMargin = dp(if (index == 0) LIST_FIRST_ROW_TOP_DP else LIST_ROW_GAP_DP)
            })
        }
    }

    private fun buildResultsHeading(projects: List<StoreProject>) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, 0, 0, dp(4))
        addView(TextView(activity).apply {
            text = "全部"
            includeFontPadding = false
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
        }, LinearLayout.LayoutParams(0, dp(38), 1f).apply { gravity = Gravity.CENTER_VERTICAL })
        addView(TextView(activity).apply {
            val installableCount = projects.count { !it.latestApkUrl.isNullOrBlank() }
            text = "${projects.size} 个项目 · $installableCount 个可安装"
            includeFontPadding = false
            gravity = Gravity.CENTER_VERTICAL
            setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, dp(38)))
    }

    private fun buildProjectListRow(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(LIST_ROW_MIN_HEIGHT_DP)
        setPadding(0, dp(14), 0, dp(14))
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { openProjectSpace(project) }
        addView(projectPlazaProjectCover(
            activity = activity,
            project = project,
            sizePx = dp(LIST_COVER_SIZE_DP),
            radiusPx = dp(LIST_COVER_RADIUS_DP).toFloat(),
            fallbackTextSp = 22f
        ), LinearLayout.LayoutParams(dp(LIST_COVER_SIZE_DP), dp(LIST_COVER_SIZE_DP)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                text = project.displayTitle(); maxLines = 1; ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY)); setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            })
            addView(TextView(activity).apply {
                text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
                maxLines = 1; ellipsize = TextUtils.TruncateAt.END; includeFontPadding = false
                setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY)); setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(5) })
            addView(buildProjectListMeta(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(7) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(16) })
        addView(FrameLayout(activity).apply {
            contentDescription = "进入${project.displayTitle()}"
            addView(ImageView(activity).apply {
                setImageResource(R.drawable.project_view_chevron)
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = null
            }, FrameLayout.LayoutParams(dp(LIST_CHEVRON_DP), dp(LIST_CHEVRON_DP), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(LIST_CHEVRON_END_INSET_DP)
            })
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun buildProjectListMeta(project: StoreProject) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        addView(TextView(activity).apply {
            val owner = project.ownerAccount.trim().takeIf { it.isNotBlank() && it != "?" } ?: "未知"
            text = "$owner · ${project.memberCount.coerceAtLeast(0)} 人"
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        val build = projectPlazaBuildStatus(project.lastTaskStatus)
        addView(View(activity).apply {
            background = rect(toneColor(build.tone), 3)
            contentDescription = null
        }, LinearLayout.LayoutParams(dp(6), dp(6)).apply { marginStart = dp(8) })
        addView(TextView(activity).apply {
            text = build.label
            includeFontPadding = false
            setTextColor(Color.parseColor(toneColor(build.tone)))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { marginStart = dp(6) })
    }

    private fun isProjectJoined(project: StoreProject): Boolean {
        return !project.viewerRole.isNullOrBlank() || joinedIds.contains(project.id)
    }

    private fun rerenderCurrentProjects() {
        if (currentProjects.isNotEmpty()) renderProjects(currentProjects)
    }

    private fun resetDiscovery() {
        searchDebounce?.let { activity.window.decorView.removeCallbacks(it) }
        searchDebounce = null
        searchQuery = ""
        activeFilterKey = FILTER_ALL
        searchField?.setText("")
        updateFilterChipVisuals()
        loadProjects()
    }

    private fun toneColor(tone: ProjectPlazaTone): String = when (tone) {
        ProjectPlazaTone.SUCCESS -> COLOR_STATUS_SUCCESS
        ProjectPlazaTone.DANGER -> COLOR_STATUS_DANGER
        ProjectPlazaTone.NEUTRAL -> COLOR_TEXT_TERTIARY
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

    private fun rect(color: String, radiusDp: Int = 0): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private companion object {
        const val FILTER_ALL = "all"
        const val COLOR_SEARCH_BG = "#272727"
        const val COLOR_SEGMENT_SELECTED = "#1A1A1A"
        const val COLOR_TEXT_PRIMARY = "#D9D9D9"
        const val COLOR_TEXT_PLACEHOLDER = "#AFAFAF"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val FONT_PAGE_TITLE_SP = 16f
        const val COLOR_STATUS_SUCCESS = "#58BE6A"
        const val COLOR_STATUS_DANGER = "#E62129"
        const val SEARCH_HEIGHT_DP = 56
        const val SEARCH_RADIUS_DP = 28
        const val PLAZA_SIDE_MARGIN_DP = 20
        const val FILTER_CHIP_HEIGHT_DP = 48
        const val FILTER_CHIP_SELECTED_WIDTH_DP = 70
        const val FILTER_CHIP_RADIUS_DP = 24
        const val FILTER_SIDE_PADDING_DP = 16
        const val FILTER_ITEM_GAP_DP = 14
        const val LIST_FIRST_ROW_TOP_DP = 2
        const val LIST_ROW_GAP_DP = 4
        const val LIST_ROW_MIN_HEIGHT_DP = 112
        const val LIST_COVER_SIZE_DP = 60
        const val LIST_COVER_RADIUS_DP = 12
        const val LIST_CHEVRON_DP = 16
        const val LIST_CHEVRON_END_INSET_DP = 4
    }
}
