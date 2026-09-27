package com.elon.app

import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.content.Context
import android.content.res.ColorStateList
import com.google.android.material.button.MaterialButton

internal class ProjectManagementHomeView(
    private val activity: Context,
    private val container: LinearLayout,
    private val segmentContainer: LinearLayout?,
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
        container.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        val indexed = projects().mapIndexed { index, project -> IndexedProject(index, project) }
        val personal = indexed
            .filter { !it.project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<IndexedProject> { it.project.isSystemArchiveProject() }
                    .thenByDescending { conversationWorkingSortKey(isProjectWorking(it.project)) }
                    .thenByDescending { it.project.updatedAt }
            )
        val joint = indexed
            .filter { it.project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<IndexedProject> { conversationWorkingSortKey(isProjectWorking(it.project)) }
                    .thenByDescending { it.project.updatedAt }
            )
        val showJoint = jointProjectsExpanded() && !personalProjectsExpanded()
        val visibleProjects = if (showJoint) joint else personal

        if (renderFixedSegment(showJoint)) {
            container.addView(fixedSegmentSpacer())
        } else {
            container.addView(createSegmentRow(showJoint), segmentLayoutParams())
        }
        if (visibleProjects.isEmpty()) {
            container.addView(createEmptyState(showJoint), firstRowLayoutParams())
        } else {
            visibleProjects.forEachIndexed { rowIndex, item ->
                container.addView(createProjectRow(item), rowLayoutParams(rowIndex))
            }
        }
        container.addView(bottomSpacer())
    }

    private fun renderFixedSegment(showJoint: Boolean): Boolean {
        val target = segmentContainer ?: return false
        target.removeAllViews()
        target.setBackgroundColor(activity.elonColor(R.color.elon_bg_app))
        target.layoutParams = target.layoutParams.apply {
            height = LinearLayout.LayoutParams.WRAP_CONTENT
        }
        target.addView(createSegmentRow(showJoint), fixedSegmentLayoutParams())
        return true
    }

    private fun createSegmentRow(showJoint: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(segmentButton("独立", selected = !showJoint) {
                setPersonalProjectsExpanded(true)
                setJointProjectsExpanded(false)
                render()
            }, segmentButtonLayoutParams())
            addView(segmentButton("联合", selected = showJoint) {
                setPersonalProjectsExpanded(false)
                setJointProjectsExpanded(true)
                render()
            }, segmentButtonLayoutParams(marginStartPx = SEGMENT_GAP_DP))
        }
    }

    private fun segmentButton(
        label: String,
        selected: Boolean,
        onClick: () -> Unit
    ): TextView {
        return MaterialButton(activity).apply {
            text = label
            isAllCaps = false
            isSelected = selected
            minimumHeight = dp(48)
            minHeight = dp(48)
            insetTop = 0; insetBottom = 0
            setFontSizeSp(FONT_SEGMENT_SP)
            setPadding(dp(16), dp(12), dp(16), dp(12))
            setTextColor(activity.elonColor(if (selected) R.color.mobile_on_primary_container else R.color.mobile_on_surface_variant))
            backgroundTintList = ColorStateList.valueOf(activity.elonColor(
                if (selected) R.color.elon_segment_selected else R.color.elon_bg_app))
            cornerRadius = dp(12)
            setOnClickListener { onClick() }
        }
    }

    private fun segmentButtonLayoutParams(marginStartPx: Int = 0): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            1f
        ).apply {
            marginStart = dp(marginStartPx)
        }
    }

    private fun createProjectRow(item: IndexedProject): View {
        val project = item.project
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            isClickable = true
            isFocusable = true
            minimumHeight = dp(72)
            setPadding(0, dp(12), 0, dp(12))
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener { anchor ->
                showProjectActions(item.index, anchor)
                true
            }

            addView(projectThumbnail(project), LinearLayout.LayoutParams(
                dp(THUMB_SIZE_DP),
                dp(THUMB_SIZE_DP)
            ))

            addView(projectTextColumn(project), LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            ).apply {
                marginStart = dp(TEXT_START_GAP_DP)
                marginEnd = dp(TEXT_END_GAP_DP)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                gravity = Gravity.CENTER
                text = "›"
                setTextColor(activity.elonColor(R.color.elon_text_placeholder))
                setFontSizeSp(FONT_CHEVRON_SP)
            }, LinearLayout.LayoutParams(
                dp(CHEVRON_WIDTH_DP),
                dp(THUMB_SIZE_DP)
            ))
        }
    }

    private fun projectTextColumn(project: AppProject): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            minimumHeight = dp(THUMB_SIZE_DP)
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = project.title.ifBlank { "项目名称" }
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(activity.elonColor(R.color.elon_text_list_title))
                setFontSizeSp(FONT_LIST_TITLE_SP)
                setTypeface(typeface, Typeface.NORMAL)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectIntroduction(project)
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(activity.elonColor(R.color.elon_text_placeholder))
                setFontSizeSp(FONT_LIST_DESC_SP)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(DESC_TOP_MARGIN_DP)
            })

            addView(LinearLayout(activity).apply {
                orientation = if (resources.configuration.fontScale >= 1.3f) LinearLayout.VERTICAL else LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addMetaText("创建者：${projectOwner(project)}", weighted = orientation == LinearLayout.HORIZONTAL)
                addMetaText("成员：${projectMemberCount(project)}", marginStartPx = if (orientation == LinearLayout.HORIZONTAL) META_GAP_DP else 0)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(META_TOP_MARGIN_DP)
            })
        }
    }

    private fun LinearLayout.addMetaText(value: String, marginStartPx: Int = 0, weighted: Boolean = false) {
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = value
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(activity.elonColor(R.color.elon_text_placeholder))
            setFontSizeSp(FONT_META_SP)
        }, LinearLayout.LayoutParams(
            if (weighted) 0 else LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            if (weighted) 1f else 0f
        ).apply {
            marginStart = dp(marginStartPx)
        })
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = roundedPx(activity.elonColor(R.color.elon_button_primary_bg), THUMB_RADIUS_DP)
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
                    text = projectInitial(project)
                    setTextColor(activity.elonColor(R.color.elon_button_primary_text))
                    setFontSizeSp(FONT_THUMB_INITIAL_SP)
                    setTypeface(typeface, Typeface.BOLD)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun createEmptyState(showJoint: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            minimumHeight = dp(160)
            setPadding(dp(16), dp(24), dp(16), dp(24))
            isFocusable = !showJoint
            isClickable = !showJoint
            foreground = if (showJoint) null else selectableForeground()
            if (!showJoint) setOnClickListener { showCreateProjectDialog() }

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = if (showJoint) "暂无联合项目" else "还没有项目，点击 + 创建"
                setTextColor(activity.elonColor(R.color.elon_text_placeholder))
                setFontSizeSp(FONT_EMPTY_SP)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun segmentLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(SEGMENT_SIDE_DP)
            marginEnd = dp(SEGMENT_SIDE_DP)
            topMargin = dp(SEGMENT_TOP_MARGIN_DP)
        }
    }

    private fun firstRowLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(ROW_SIDE_DP)
            marginEnd = dp(ROW_END_DP)
            topMargin = dp(FIRST_ROW_TOP_MARGIN_DP)
        }
    }

    private fun rowLayoutParams(index: Int): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(ROW_SIDE_DP)
            marginEnd = dp(ROW_END_DP)
            topMargin = dp(if (index == 0) FIRST_ROW_TOP_MARGIN_DP else ROW_GAP_DP)
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

    private fun projectIntroduction(project: AppProject): String {
        return cleanProjectText(project.projectCardIntroduction())
            ?: cleanProjectText(project.subtitle)
            ?: "暂无简介"
    }

    private fun projectInitial(project: AppProject): String {
        val text = cleanProjectText(project.title) ?: return "\u9879"
        val index = text.indexOfFirst { !it.isWhitespace() }
        if (index < 0) return "\u9879"
        val codePoint = text.codePointAt(index)
        return String(Character.toChars(codePoint)).uppercase()
    }

    private fun cleanProjectText(value: String?): String? {
        val text = value?.trim().orEmpty()
        return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun fixedSegmentSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                segmentContainer?.let { target ->
                    target.measure(View.MeasureSpec.makeMeasureSpec(activity.resources.displayMetrics.widthPixels, View.MeasureSpec.EXACTLY),
                        View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED))
                    target.measuredHeight
                } ?: dp(FIXED_SEGMENT_BAR_HEIGHT_DP)
            )
        }
    }

    private fun fixedSegmentLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(SEGMENT_SIDE_DP)
            marginEnd = dp(SEGMENT_SIDE_DP)
            topMargin = dp(FIXED_SEGMENT_TOP_MARGIN_DP)
        }
    }

    private fun bottomSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(BOTTOM_SPACER_DP)
            )
        }
    }

    private fun TextView.setFontSizeSp(value: Int) {
        setTextSize(TypedValue.COMPLEX_UNIT_SP, value.toFloat())
    }

    private fun roundedPx(color: Int, radiusPx: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(color)
            cornerRadius = dp(radiusPx).toFloat()
        }
    }

    private companion object {
        const val SEGMENT_SIDE_DP = 16
        const val ROW_SIDE_DP = 16
        const val ROW_END_DP = 16
        const val SEGMENT_TOP_MARGIN_DP = 16
        const val SEGMENT_HEIGHT_DP = 48
        const val SEGMENT_WIDTH_DP = 96
        const val SEGMENT_GAP_DP = 8
        const val FIXED_SEGMENT_TOP_MARGIN_DP = 8
        const val FIXED_SEGMENT_BAR_HEIGHT_DP = FIXED_SEGMENT_TOP_MARGIN_DP + SEGMENT_HEIGHT_DP
        const val FIRST_ROW_TOP_MARGIN_DP = 12
        const val ROW_HEIGHT_DP = 88
        const val ROW_GAP_DP = 4
        const val THUMB_SIZE_DP = 48
        const val THUMB_RADIUS_DP = 12
        const val TEXT_START_GAP_DP = 16
        const val TEXT_END_GAP_DP = 8
        const val DESC_TOP_MARGIN_DP = 4
        const val META_TOP_MARGIN_DP = 4
        const val META_GAP_DP = 12
        const val CHEVRON_WIDTH_DP = 24
        const val EMPTY_HEIGHT_DP = 160
        const val BOTTOM_SPACER_DP = 24

        const val FONT_SEGMENT_SP = 14
        const val FONT_LIST_TITLE_SP = 16
        const val FONT_LIST_DESC_SP = 14
        const val FONT_META_SP = 12
        const val FONT_EMPTY_SP = 16
        const val FONT_CHEVRON_SP = 24
        const val FONT_THUMB_INITIAL_SP = 24
    }
}
