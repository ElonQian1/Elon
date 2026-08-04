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
import kotlin.math.roundToInt

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
    private val featuredSection by lazy {
        ProjectPlazaFeaturedSection(
            activity = activity,
            dp = dp,
            selectableForeground = selectableForeground,
            reactionPrefs = reactionPrefs,
            openProjectSpace = openProjectSpace,
            isProjectJoined = ::isProjectJoined
        )
    }
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
                topMargin = designPx(if (index == 0) LIST_FIRST_ROW_TOP_PX else LIST_ROW_GAP_PX)
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
        setPadding(dp(9), 0, 0, 0)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { openProjectSpace(project) }
        addView(projectThumbnail(project), LinearLayout.LayoutParams(designPx(159), designPx(155)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                text = project.displayTitle(); maxLines = 1; ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY)); setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            })
            addView(TextView(activity).apply {
                text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
                maxLines = 1; ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY)); setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(5) })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = designPx(LIST_TEXT_GAP_PX) })
        addView(FrameLayout(activity).apply {
            contentDescription = "进入${project.displayTitle()}"
            addView(ImageView(activity).apply {
                setImageResource(R.drawable.project_view_chevron)
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = null
            }, FrameLayout.LayoutParams(designPx(LIST_CHEVRON_PX), designPx(LIST_CHEVRON_PX), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = designPx(LIST_CHEVRON_END_INSET_PX)
            })
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun projectThumbnail(project: StoreProject): View {
        return ImageView(activity).apply {
            setImageResource(R.drawable.project_plaza_ui2_thumbnail)
            scaleType = ImageView.ScaleType.FIT_XY
            contentDescription = "${project.displayTitle()}缩略图"
        }
    }

    private fun isProjectJoined(project: StoreProject): Boolean {
        return !project.viewerRole.isNullOrBlank() || joinedIds.contains(project.id)
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

    private fun rect(color: String, radiusDp: Int = 0): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private companion object {
        const val FILTER_ALL = "all"
        const val COLOR_SEARCH_BG = "#272727"
        const val COLOR_SEGMENT_SELECTED = "#1A1A1A"
        const val COLOR_TEXT_PRIMARY = "#D9D9D9"
        const val COLOR_TEXT_SECONDARY = "#B8B8B8"
        const val COLOR_TEXT_PLACEHOLDER = "#AFAFAF"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val FONT_PAGE_TITLE_SP = 16f
        const val SEARCH_HEIGHT_DP = 48
        const val SEARCH_RADIUS_DP = 24
        const val PLAZA_SIDE_MARGIN_DP = 20
        const val FILTER_CHIP_HEIGHT_DP = 48
        const val FILTER_CHIP_SELECTED_WIDTH_DP = 70
        const val FILTER_CHIP_RADIUS_DP = 24
        const val FILTER_SIDE_PADDING_DP = 16
        const val FILTER_ITEM_GAP_DP = 14
        const val DESIGN_WIDTH_PX = 1275
        const val LIST_FIRST_ROW_TOP_PX = 28
        const val LIST_ROW_GAP_PX = 64
        const val LIST_TEXT_GAP_PX = 48
        const val LIST_CHEVRON_PX = 43
        const val LIST_CHEVRON_END_INSET_PX = 12
    }
}
