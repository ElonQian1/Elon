package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.animation.DecelerateInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class ProjectManagementHomeView(
    private val activity: AppCompatActivity,
    private val container: LinearLayout,
    private val projects: () -> List<AppProject>,
    private val plazaProjects: () -> List<StoreProject>,
    private val personalProjectsExpanded: () -> Boolean,
    private val jointProjectsExpanded: () -> Boolean,
    private val setPersonalProjectsExpanded: (Boolean) -> Unit,
    private val setJointProjectsExpanded: (Boolean) -> Unit,
    private val formatTime: (Long) -> String,
    private val openProject: (Int) -> Unit,
    private val openProjectConversations: (Int) -> Unit,
    private val isProjectWorking: (AppProject) -> Boolean,
    private val showProjectActions: (Int, View?) -> Unit,
    private val showCreateProjectDialog: () -> Unit,
    private val showProjectPlaza: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private data class IndexedProject(
        val index: Int,
        val project: AppProject
    )

    fun render() {
        container.removeAllViews()
        container.setBackgroundColor(Color.parseColor("#101010"))
        container.addView(createPlazaBanner())

        val indexed = projects().mapIndexed { index, project -> IndexedProject(index, project) }
        val personal = indexed
            .filter { !it.project.isJointDevelopmentProject() }
            .sortedWith(compareByDescending<IndexedProject> { it.project.isSystemArchiveProject() }
                .thenByDescending { it.project.updatedAt })
        val joint = indexed
            .filter { it.project.isJointDevelopmentProject() }
            .sortedByDescending { it.project.updatedAt }

        addSection(
            title = "个人项目",
            items = personal,
            topMargin = 4,
            emptyAction = showCreateProjectDialog,
            expanded = personalProjectsExpanded(),
            setExpanded = setPersonalProjectsExpanded
        )
        addSection(
            title = "联合项目",
            items = joint,
            topMargin = 4,
            emptyAction = null,
            expanded = jointProjectsExpanded(),
            setExpanded = setJointProjectsExpanded
        )
        container.addView(bottomSpacer())
    }

    private fun createPlazaBanner(): View {
        val contentWidth = activity.resources.displayMetrics.widthPixels - dp(16)
        val bannerHeight = (contentWidth * 0.36f).toInt().coerceIn(dp(124), dp(158))
        val bannerProjects = plazaProjects()
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                bannerHeight
            ).apply {
                marginStart = dp(8)
                marginEnd = dp(8)
                topMargin = dp(12)
            }
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showProjectPlaza() }

            addView(ProjectPlazaPatternView(activity, bannerProjects), FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "项目广场"
                setPadding(dp(14), dp(9), dp(14), dp(9))
                background = rect("#990F1217", 6)
                setTextColor(Color.parseColor("#F2F5FA"))
                alpha = 0.92f
                setTextSize(TypedValue.COMPLEX_UNIT_SP, SECTION_TITLE_TEXT_SP)
                setShadowLayer(dp(2).toFloat(), 0f, dp(1).toFloat(), Color.parseColor("#AA000000"))
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.START or Gravity.TOP
                leftMargin = dp(12)
                topMargin = dp(14)
            })

        }
    }

    private fun addSection(
        title: String,
        items: List<IndexedProject>,
        topMargin: Int,
        emptyAction: (() -> Unit)?,
        expanded: Boolean,
        setExpanded: (Boolean) -> Unit
    ) {
        val canExpand = items.size > COLLAPSED_PROJECT_LIMIT
        val initiallyExpanded = expanded && canExpand
        val gridContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            clipChildren = true
        }
        lateinit var header: LinearLayout
        header = createSectionHeader(title, canExpand, initiallyExpanded) { arrow ->
            val targetExpanded = !arrow.isSelected
            setExpanded(targetExpanded)
            header.contentDescription = "${title}${if (targetExpanded) "收起" else "展开"}"
            header.isEnabled = false
            animateSectionGrid(
                gridContainer = gridContainer,
                allItems = items,
                emptyAction = emptyAction,
                expanded = targetExpanded,
                arrow = arrow
            ) {
                header.isEnabled = true
            }
        }
        container.addView(header, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(42)
        ).apply {
            marginStart = dp(8)
            marginEnd = dp(8)
            this.topMargin = dp(topMargin)
        })
        container.addView(gridContainer, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        addProjectGrid(gridContainer, sectionItems(items, initiallyExpanded), emptyAction)
    }

    private fun createSectionHeader(
        title: String,
        canExpand: Boolean,
        expanded: Boolean,
        onToggle: (TextView) -> Unit
    ): LinearLayout {
        val arrow = TextView(activity).apply {
            includeFontPadding = false
            text = "›"
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A6AFBD"))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 21f)
            isSelected = expanded
            rotation = if (expanded) 90f else 0f
            visibility = if (canExpand) View.VISIBLE else View.INVISIBLE
        }
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(6), 0, 0, 0)
            if (canExpand) {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = "${title}${if (expanded) "收起" else "展开"}"
                setOnClickListener { onToggle(arrow) }
            }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, SECTION_TITLE_TEXT_SP)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(arrow, LinearLayout.LayoutParams(dp(24), LinearLayout.LayoutParams.MATCH_PARENT))
        }
    }

    private fun sectionItems(items: List<IndexedProject>, expanded: Boolean): List<IndexedProject> {
        if (expanded || items.size <= COLLAPSED_PROJECT_LIMIT) return items
        return items.take(COLLAPSED_PROJECT_LIMIT)
    }

    private fun addProjectGrid(
        target: LinearLayout,
        items: List<IndexedProject>,
        emptyAction: (() -> Unit)?
    ) {
        val cells = when {
            items.isEmpty() -> listOf<IndexedProject?>(null, null)
            items.size % 2 == 0 -> items
            else -> items + listOf(null)
        }
        cells.chunked(2).forEachIndexed { rowIndex, rowItems ->
            target.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.TOP
                rowItems.forEachIndexed { cellIndex, indexed ->
                    val card = indexed?.let { createProjectCard(it) } ?: createEmptyProjectSlot(emptyAction)
                    addView(card, LinearLayout.LayoutParams(
                        0,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        1f
                    ).apply {
                        if (cellIndex == 0) marginEnd = dp(4)
                        else marginStart = dp(4)
                    })
                }
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(8)
                marginEnd = dp(8)
                topMargin = if (rowIndex == 0) dp(6) else dp(14)
            })
        }
    }

    private fun animateSectionGrid(
        gridContainer: LinearLayout,
        allItems: List<IndexedProject>,
        emptyAction: (() -> Unit)?,
        expanded: Boolean,
        arrow: TextView,
        onFinished: () -> Unit
    ) {
        val startHeight = gridContainer.height.takeIf { it > 0 }
            ?: measureProjectGridHeight(sectionItems(allItems, !expanded), emptyAction)
        val targetItems = sectionItems(allItems, expanded)
        val targetHeight = measureProjectGridHeight(targetItems, emptyAction)
        gridContainer.animate().cancel()
        arrow.animate().cancel()
        arrow.isSelected = expanded
        arrow.animate()
            .rotation(if (expanded) 90f else 0f)
            .setDuration(180L)
            .setInterpolator(DecelerateInterpolator())
            .start()

        if (expanded) {
            gridContainer.removeAllViews()
            addProjectGrid(gridContainer, targetItems, emptyAction)
        }
        gridContainer.layoutParams = gridContainer.layoutParams.apply { height = startHeight }
        gridContainer.requestLayout()

        ValueAnimator.ofInt(startHeight, targetHeight).apply {
            duration = SECTION_ANIMATION_MS
            interpolator = DecelerateInterpolator()
            addUpdateListener { animator ->
                gridContainer.layoutParams = gridContainer.layoutParams.apply {
                    height = animator.animatedValue as Int
                }
                gridContainer.requestLayout()
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    if (!expanded) {
                        gridContainer.removeAllViews()
                        addProjectGrid(gridContainer, targetItems, emptyAction)
                    }
                    gridContainer.layoutParams = gridContainer.layoutParams.apply {
                        height = LinearLayout.LayoutParams.WRAP_CONTENT
                    }
                    gridContainer.requestLayout()
                    onFinished()
                }
            })
            start()
        }
    }

    private fun measureProjectGridHeight(
        items: List<IndexedProject>,
        emptyAction: (() -> Unit)?
    ): Int {
        val temp = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            visibility = View.INVISIBLE
        }
        addProjectGrid(temp, items, emptyAction)
        val width = container.width.takeIf { it > 0 } ?: activity.resources.displayMetrics.widthPixels
        temp.measure(
            View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED)
        )
        return temp.measuredHeight
    }

    private fun createProjectCard(item: IndexedProject): View {
        val project = item.project
        return AdaptiveProjectCardFrame(activity, dp(CARD_INFO_BAR_HEIGHT_DP)).apply {
            background = rect("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener { anchor ->
                showProjectActions(item.index, anchor)
                true
            }

            addView(createProjectCardContent(project), FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ))

            addView(createProjectInfoBar(project, isProjectWorking(project)) {
                openProjectConversations(item.index)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_INFO_BAR_HEIGHT_DP),
                Gravity.BOTTOM
            ))
        }
    }

    private fun createProjectCardContent(project: AppProject): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(10), dp(11), dp(10), dp(8))

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(projectThumbnail(project), LinearLayout.LayoutParams(dp(38), dp(38)).apply {
                    marginEnd = dp(12)
                })
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_VERTICAL
                    addProjectDetailText("来源：${project.projectOriginLabel()}")
                    addProjectDetailText("创建者：${projectOwner(project)}")
                    addProjectDetailText("成员：${projectMemberCount(project)}")
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

            project.projectCardCode()?.let { code ->
                addView(View(activity).apply {
                    setBackgroundColor(Color.parseColor("#A6AFBD"))
                    alpha = 0.72f
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(1)
                ).apply {
                    topMargin = dp(6)
                })

                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "项目代号：$code"
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_DETAIL_TEXT_SP)
                    setLineSpacing(dp(2).toFloat(), 1.0f)
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(4)
                })
                project.projectCardIntroduction()?.let { intro ->
                    addView(TextView(activity).apply {
                        includeFontPadding = false
                        text = intro
                        setTextColor(Color.parseColor("#7D8795"))
                        setTextSize(TypedValue.COMPLEX_UNIT_SP, 10f)
                        setLineSpacing(dp(2).toFloat(), 1.0f)
                    }, LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply {
                        topMargin = dp(6)
                    })
                }
            }
        }
    }

    private fun LinearLayout.addProjectDetailText(value: String) {
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = value
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor("#A6AFBD"))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_DETAIL_TEXT_SP)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            bottomMargin = dp(6)
        })
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor("#D2D2D2"))
            }
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
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    text = avatarText(project.title.ifBlank { "项目" })
                    setTextColor(Color.parseColor("#253140"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 17f)
                    setTypeface(typeface, Typeface.BOLD)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun createProjectInfoBar(project: AppProject, working: Boolean, onClick: () -> Unit): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(10), dp(5), dp(10), dp(5))
            val barBackground = rect(CARD_INFO_BAR_BG)
            background = barBackground
            if (working) startProjectConversationShimmer(this, barBackground, CARD_INFO_BAR_BG, CARD_INFO_BAR_SHIMMER_BG)
            isClickable = true
            foreground = selectableForeground()
            contentDescription = "${project.title.ifBlank { "项目" }}的项目 AI 会话"
            setOnClickListener { onClick() }

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = project.title.ifBlank { "未命名项目" }
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor("#F2F5FA"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_TITLE_TEXT_SP)
                    setTypeface(typeface, Typeface.BOLD)
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = projectTime(project)
                    maxLines = 1
                    setTextColor(Color.parseColor("#DDE8FC"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_TIME_TEXT_SP)
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginStart = dp(7)
                })
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectMeta(project)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor("#DDE8FC"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_META_TEXT_SP)
                setAutoSizeTextTypeUniformWithConfiguration(9, 11, 1, TypedValue.COMPLEX_UNIT_SP)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            })
        }
    }

    private fun projectMeta(project: AppProject): String {
        val kind = project.projectKindLabel()
        val stage = projectStageText(project.stage)
        val workspace = projectWorkspaceText(project, stage)
        return "$kind · ${project.projectOriginLabel()} · ${project.displayConversationCount()}个会话 · $workspace"
    }

    private fun projectWorkspaceText(project: AppProject, stage: String): String {
        if (stage == "运行中") return stage
        val label = cleanProjectText(project.workspaceHealthLabel)
        if (!label.isNullOrBlank()) return label
        return stage
    }

    private fun projectStageText(stage: String): String {
        val value = cleanProjectText(stage)
        return when (value?.lowercase()) {
            "running" -> "运行中"
            "done" -> "已完成"
            "failed" -> "失败"
            null -> "待提交需求"
            else -> value
        }
    }

    private fun projectOwner(project: AppProject): String {
        if (project.isSystemArchiveProject()) return SYSTEM_ARCHIVE_OWNER_ACCOUNT
        cleanProjectText(project.ownerAccount)?.let { return it }
        if (project.isJointDevelopmentProject()) return "未知"
        return AuthManager.displayName(activity).takeIf { it.isNotBlank() } ?: "未知"
    }

    private fun projectMemberCount(project: AppProject): Int {
        return project.memberCount?.coerceAtLeast(0) ?: if (project.isJointDevelopmentProject()) 0 else 1
    }

    private fun cleanProjectText(value: String?): String? {
        val text = value?.trim().orEmpty()
        return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun projectTime(project: AppProject): String {
        if (project.updatedAt <= 0L) return "时间"
        return formatTime(project.updatedAt).ifBlank { "时间" }
    }

    private fun createEmptyProjectSlot(emptyAction: (() -> Unit)?): View {
        return AdaptiveProjectCardFrame(activity).apply {
            background = rect("#181B20")
            emptyAction?.let { action ->
                contentDescription = "新建项目"
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { action() }
                addView(TextView(activity).apply {
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    text = "+"
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 34f)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun bottomSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(34)
            )
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
        const val COLLAPSED_PROJECT_LIMIT = 4
        const val SECTION_ANIMATION_MS = 260L
        const val CARD_INFO_BAR_HEIGHT_DP = 47
        const val CARD_INFO_BAR_BG = "#303338"
        const val CARD_INFO_BAR_SHIMMER_BG = "#283140"
        const val SECTION_TITLE_TEXT_SP = 16f
        const val CARD_TITLE_TEXT_SP = 14.2f
        const val CARD_TIME_TEXT_SP = 12.2f
        const val CARD_META_TEXT_SP = 10.8f
        const val CARD_DETAIL_TEXT_SP = 10.8f
    }
}

private class AdaptiveProjectCardFrame(
    context: Context,
    private val infoBarHeight: Int = 0
) : FrameLayout(context) {
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val desiredHeight = adaptiveHeight(width, widthMeasureSpec)
        val exactHeight = MeasureSpec.makeMeasureSpec(desiredHeight, MeasureSpec.EXACTLY)
        super.onMeasure(widthMeasureSpec, exactHeight)
    }

    private fun adaptiveHeight(width: Int, widthMeasureSpec: Int): Int {
        if (width <= 0 || infoBarHeight <= 0 || childCount == 0) return width
        val content = getChildAt(0)
        val lp = content.layoutParams as? FrameLayout.LayoutParams
            ?: return width
        val contentWidthSpec = getChildMeasureSpec(
            widthMeasureSpec,
            paddingLeft + paddingRight + lp.leftMargin + lp.rightMargin,
            lp.width
        )
        content.measure(
            contentWidthSpec,
            MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED)
        )
        val contentHeight = content.measuredHeight + lp.topMargin + lp.bottomMargin
        return (contentHeight + infoBarHeight).coerceAtLeast(width)
    }
}
