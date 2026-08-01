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
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import java.util.Locale
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

    private data class ProjectCardIdentity(
        val title: String,
        val subtitle: String?
    )

    private val joinedIds = mutableSetOf<String>()
    private val reactionPrefs by lazy { activity.getSharedPreferences("project_plaza_reactions", 0) }
    private val filterChipViews = LinkedHashMap<String, TextView>()
    private var resultsContainer: LinearLayout? = null
    private var searchPanel: View? = null
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
        shell.addView(buildDiscoveryHeader())
        searchPanel = buildSearchBar().also { it.visibility = View.GONE }
        shell.addView(searchPanel)
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

    private fun buildDiscoveryHeader(): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(PLAZA_SIDE_MARGIN_DP), 0, dp(PLAZA_SEARCH_END_MARGIN_DP), 0)
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(48)
        )
        addView(sectionTitle("推荐"), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        addView(FrameLayout(activity).apply {
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "搜索项目"
            setOnClickListener {
                val panel = searchPanel ?: return@setOnClickListener
                val showing = panel.visibility == View.VISIBLE
                panel.visibility = if (showing) View.GONE else View.VISIBLE
                if (!showing) searchField?.requestFocus()
            }
            addView(ImageView(activity).apply {
                setImageResource(R.drawable.ic_top_search_custom)
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = null
            }, FrameLayout.LayoutParams(designPx(TOP_SEARCH_ICON_PX), designPx(TOP_SEARCH_ICON_PX), Gravity.CENTER))
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
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
                bottomMargin = dp(12)
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
        container.addView(buildFeaturedStrip(projects.take(5)), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            featuredCardHeightPx()
        ).apply { topMargin = dp(4) })
        container.addView(sectionTitle("全部"), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(PLAZA_SIDE_MARGIN_DP)
            marginEnd = dp(PLAZA_SIDE_MARGIN_DP)
            topMargin = dp(14)
            bottomMargin = dp(4)
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

    private fun sectionTitle(label: String) = TextView(activity).apply {
        text = label
        includeFontPadding = false
        setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
    }

    private fun buildFeaturedStrip(projects: List<StoreProject>) = ProjectPlazaCarousel(activity).apply {
        configureContentInsets(dp(PLAZA_SIDE_MARGIN_DP), dp(PLAZA_TRAILING_PADDING_DP))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEachIndexed { index, project ->
                val card = buildFeaturedCard(project).apply {
                    val initialScale = if (index == 0) 1f else PROJECT_PLAZA_CARD_MIN_SCALE
                    scaleX = initialScale
                    scaleY = initialScale
                }
                addView(card, LinearLayout.LayoutParams(featuredCardWidthPx(), featuredCardHeightPx()).apply {
                    marginEnd = dp(FEATURED_CARD_GAP_DP)
                })
            }
        })
        post { refreshCardScales() }
    }

    private fun buildFeaturedCard(project: StoreProject) = FrameLayout(activity).apply {
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "查看${project.displayTitle()}"
        setOnClickListener { openProjectSpace(project) }
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.project_plaza_ui1_card)
            scaleType = ImageView.ScaleType.FIT_XY
            contentDescription = null
        }, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(19), dp(21), dp(19), dp(14))
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                addView(ImageView(activity).apply {
                    setImageResource(R.drawable.project_plaza_ui3_avatar)
                    scaleType = ImageView.ScaleType.FIT_XY
                    contentDescription = "${project.displayTitle()}头像"
                }, LinearLayout.LayoutParams(dp(36), dp(36)))
                addView(LinearLayout(activity).apply {
                    gravity = Gravity.END or Gravity.CENTER_VERTICAL
                    addView(reactionImageButton(project, "favorite", R.drawable.project_plaza_ui5_star, "收藏"))
                    addView(reactionImageButton(project, "liked", R.drawable.project_plaza_ui4_heart, "点赞"), LinearLayout.LayoutParams(dp(30), dp(36)).apply { marginStart = dp(1) })
                }, LinearLayout.LayoutParams(0, dp(36), 1f))
            })
            addView(TextView(activity).apply {
                text = project.displayTitle()
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(11) })
            addView(TextView(activity).apply {
                text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
                setLineSpacing(0f, 1.05f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(28)).apply { topMargin = dp(6) })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                repeat(2) { index ->
                    addView(mediaPlaceholder(), LinearLayout.LayoutParams(dp(60), dp(85)).apply {
                        if (index == 1) marginStart = dp(7)
                    })
                }
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(85)).apply { topMargin = dp(9) })
        }, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.project_view_chevron)
            scaleType = ImageView.ScaleType.FIT_CENTER
            setColorFilter(Color.parseColor("#454545"))
            background = rect("#D9D9D9", 18)
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "进入${project.displayTitle()}"
        }, FrameLayout.LayoutParams(dp(27), dp(27), Gravity.END or Gravity.BOTTOM).apply {
            marginEnd = dp(20)
            bottomMargin = dp(17)
        })
    }

    private fun mediaPlaceholder() = FrameLayout(activity).apply {
        background = rect("#676767", 10)
        addView(ImageView(activity).apply {
            setImageResource(R.drawable.ic_attach_photos)
            setColorFilter(Color.parseColor("#D9D9D9"))
            contentDescription = null
        }, FrameLayout.LayoutParams(dp(24), dp(24), Gravity.CENTER))
    }

    private fun reactionImageButton(
        project: StoreProject,
        key: String,
        drawableRes: Int,
        label: String
    ) = FrameLayout(activity).apply {
        val icon = ImageView(activity).apply {
            setImageResource(drawableRes)
            scaleType = ImageView.ScaleType.FIT_CENTER
            contentDescription = null
        }
        fun render() {
            val selected = reactionPrefs.getBoolean("${project.id}:$key", false)
            icon.alpha = if (selected) 1f else 0.72f
            contentDescription = if (selected) "取消$label" else label
        }
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            reactionPrefs.edit()
                .putBoolean("${project.id}:$key", !reactionPrefs.getBoolean("${project.id}:$key", false))
                .apply()
            render()
        }
        val iconWidth = if (drawableRes == R.drawable.project_plaza_ui5_star) designPx(65) else designPx(59)
        val iconHeight = if (drawableRes == R.drawable.project_plaza_ui5_star) designPx(59) else designPx(55)
        addView(icon, FrameLayout.LayoutParams(iconWidth, iconHeight, Gravity.CENTER))
        render()
    }.also { it.layoutParams = LinearLayout.LayoutParams(dp(30), dp(36)) }

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

    private fun featuredCardWidthPx(): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: dp(360)
        return (width * FEATURED_CARD_WIDTH_FRACTION).roundToInt()
    }

    private fun featuredCardHeightPx(): Int =
        (featuredCardWidthPx() * FEATURED_CARD_HEIGHT_RATIO).roundToInt()

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

    private fun iconActionParams(first: Boolean = false): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(dp(ACTION_ICON_TOUCH_DP), dp(ACTION_ICON_TOUCH_DP)).apply {
            if (!first) marginStart = dp(ACTION_ICON_GAP_DP)
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

    private fun memberColumnThumbStartPx(): Int {
        val screenWidth = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: dp(360)
        val cardWidth = screenWidth - dp(CARD_SIDE_MARGIN_DP * 2)
        val separatorWidth = dp(1) * 3
        val firstStatCenter = (cardWidth - separatorWidth).coerceAtLeast(0) / 8f
        return (firstStatCenter - dp(THUMB_SIZE_DP) / 2f).roundToInt().coerceAtLeast(0)
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
        const val COLOR_THUMB_BG = "#D9D9D9"
        const val FONT_AVATAR_SP = 17f
        const val FONT_PAGE_TITLE_SP = 16f
        const val FONT_CARD_TITLE_SP = 16f
        const val FONT_CARD_META_SP = 13f
        const val FONT_CARD_DESC_SP = 13f
        const val FONT_STAT_VALUE_SP = 16f
        const val FONT_STAT_LABEL_SP = 13f
        const val SEARCH_HEIGHT_DP = 48
        const val SEARCH_RADIUS_DP = 24
        const val SEARCH_SIDE_MARGIN_DP = 16
        const val PLAZA_SIDE_MARGIN_DP = 20
        const val PLAZA_SEARCH_END_MARGIN_DP = 12
        const val PLAZA_TRAILING_PADDING_DP = 98
        const val FEATURED_CARD_GAP_DP = 9
        const val FEATURED_CARD_WIDTH_FRACTION = 0.6564706f
        const val FEATURED_CARD_HEIGHT_RATIO = 1.1266428f
        const val DESIGN_WIDTH_PX = 1275
        const val TOP_SEARCH_ICON_PX = 97
        const val LIST_FIRST_ROW_TOP_PX = 28
        const val LIST_ROW_GAP_PX = 64
        const val LIST_TEXT_GAP_PX = 48
        const val LIST_CHEVRON_PX = 43
        const val LIST_CHEVRON_END_INSET_PX = 12
        const val SEGMENT_WIDTH_PX = 210
        const val SEGMENT_HEIGHT_PX = 138
        const val FILTER_SIDE_PADDING_DP = 16
        const val FILTER_ITEM_GAP_DP = 14
        const val CARD_SIDE_MARGIN_DP = 16
        const val FIRST_CARD_TOP_MARGIN_DP = 36
        const val CARD_GAP_DP = 34
        const val CARD_THUMB_TEXT_GAP_DP = 10
        const val CARD_TITLE_ACTION_GAP_DP = 4
        const val HEADER_META_TOP_MARGIN_DP = 4
        const val STATS_TOP_MARGIN_DP = 12
        const val STATS_ROW_HEIGHT_DP = 58
        const val STAT_TOP_SLOT_HEIGHT_DP = 28
        const val STAT_SEPARATOR_HEIGHT_DP = 34
        const val DESC_TOP_MARGIN_DP = 14
        const val CARD_DIVIDER_TOP_MARGIN_DP = 16
        const val THUMB_SIZE_DP = 44
        const val THUMB_RADIUS_DP = 5
        const val MEMBER_STAT_ICON_DP = 28
        const val ACTION_ICON_TOUCH_DP = 40
        const val ACTION_ICON_SIZE_DP = 30
        const val ACTION_ICON_GAP_DP = 2
        const val DESC_COLLAPSED_LINES = 2
        const val DISABLED_ACTION_ALPHA = 0.45f
    }
}
