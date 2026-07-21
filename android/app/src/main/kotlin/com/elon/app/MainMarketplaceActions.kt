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
        container.addView(sectionTitle("推荐"), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(CARD_SIDE_MARGIN_DP)
            marginEnd = dp(CARD_SIDE_MARGIN_DP)
            topMargin = dp(28)
        })
        container.addView(buildFeaturedStrip(projects.take(5)), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(392)
        ).apply { topMargin = dp(18) })
        container.addView(sectionTitle("全部项目"), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(CARD_SIDE_MARGIN_DP)
            marginEnd = dp(CARD_SIDE_MARGIN_DP)
            topMargin = dp(28)
            bottomMargin = dp(10)
        })
        projects.forEachIndexed { index, project ->
            container.addView(buildProjectListRow(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(CARD_SIDE_MARGIN_DP)
                marginEnd = dp(CARD_SIDE_MARGIN_DP)
                topMargin = dp(if (index == 0) 0 else 12)
            })
        }
    }

    private fun sectionTitle(label: String) = TextView(activity).apply {
        text = label
        includeFontPadding = false
        setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
    }

    private fun buildFeaturedStrip(projects: List<StoreProject>) = HorizontalScrollView(activity).apply {
        isHorizontalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        clipToPadding = false
        setPadding(dp(CARD_SIDE_MARGIN_DP), 0, dp(28), 0)
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEach { project ->
                addView(buildFeaturedCard(project), LinearLayout.LayoutParams(dp(306), dp(382)).apply {
                    marginEnd = dp(16)
                })
            }
        })
    }

    private fun buildFeaturedCard(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(24), dp(24), dp(24), dp(20))
        background = rect("#181818", 28)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { openProjectSpace(project) }
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(projectThumbnail(project), LinearLayout.LayoutParams(dp(50), dp(50)))
            addView(LinearLayout(activity).apply {
                gravity = Gravity.END or Gravity.CENTER_VERTICAL
                addView(reactionButton(project, "favorite", "☆", "★", "收藏"))
                addView(reactionButton(project, "liked", "♡", "♥", "点赞"), LinearLayout.LayoutParams(dp(48), dp(48)).apply { marginStart = dp(2) })
            }, LinearLayout.LayoutParams(0, dp(50), 1f))
        })
        addView(TextView(activity).apply {
            text = project.displayTitle()
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(18) })
        addView(TextView(activity).apply {
            text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(48)).apply { topMargin = dp(8) })
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            repeat(2) { index ->
                addView(FrameLayout(activity).apply {
                    background = rect(if (index == 0) "#747474" else "#656565", 10)
                    addView(ImageView(activity).apply {
                        setImageResource(R.drawable.ic_attach_photos)
                        setColorFilter(Color.parseColor("#D9D9D9"))
                    }, FrameLayout.LayoutParams(dp(28), dp(28), Gravity.CENTER))
                }, LinearLayout.LayoutParams(0, dp(112), 1f).apply { if (index == 1) marginStart = dp(10) })
            }
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(112)).apply { topMargin = dp(12) })
        addView(TextView(activity).apply {
            text = "›"
            gravity = Gravity.CENTER
            background = rect("#D9D9D9", 25)
            setTextColor(Color.parseColor("#454545"))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 32f)
            contentDescription = "进入${project.displayTitle()}"
        }, LinearLayout.LayoutParams(dp(50), dp(50)).apply { gravity = Gravity.END; topMargin = dp(12) })
    }

    private fun reactionButton(project: StoreProject, key: String, off: String, on: String, label: String) = TextView(activity).apply {
        fun render() {
            val selected = reactionPrefs.getBoolean("${project.id}:$key", false)
            text = if (selected) on else off
            contentDescription = if (selected) "取消$label" else label
        }
        gravity = Gravity.CENTER
        setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
        setTextSize(TypedValue.COMPLEX_UNIT_SP, 28f)
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            reactionPrefs.edit().putBoolean("${project.id}:$key", !reactionPrefs.getBoolean("${project.id}:$key", false)).apply()
            render()
        }
        render()
    }.also { it.layoutParams = LinearLayout.LayoutParams(dp(48), dp(48)) }

    private fun buildProjectListRow(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(12), dp(10), dp(4), dp(10))
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener { openProjectSpace(project) }
        addView(projectThumbnail(project), LinearLayout.LayoutParams(dp(58), dp(58)))
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
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply { marginStart = dp(16) })
        addView(TextView(activity).apply {
            text = "›"; gravity = Gravity.CENTER; contentDescription = "进入${project.displayTitle()}"
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY)); setTextSize(TypedValue.COMPLEX_UNIT_SP, 32f)
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun projectThumbnail(project: StoreProject): View {
        return FrameLayout(activity).apply {
            background = rect(COLOR_THUMB_BG, THUMB_RADIUS_DP)
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
        const val DESIGN_WIDTH_PX = 1272
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
