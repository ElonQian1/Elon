package com.elon.app

import android.graphics.Color
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
import androidx.appcompat.app.AppCompatActivity
import kotlin.math.roundToInt

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
        container.setBackgroundColor(Color.parseColor(COLOR_BG))
        val indexed = projects().mapIndexed { index, project -> IndexedProject(index, project) }
        val personal = indexed
            .filter { !it.project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<IndexedProject> { it.project.isSystemArchiveProject() }
                    .thenByDescending { it.project.updatedAt }
            )
        val joint = indexed
            .filter { it.project.isJointDevelopmentProject() }
            .sortedByDescending { it.project.updatedAt }
        val showJoint = jointProjectsExpanded() && !personalProjectsExpanded()
        val visibleProjects = if (showJoint) joint else personal

        container.addView(createSegmentRow(showJoint), segmentLayoutParams())
        if (visibleProjects.isEmpty()) {
            container.addView(createEmptyState(showJoint), firstRowLayoutParams())
        } else {
            visibleProjects.forEachIndexed { rowIndex, item ->
                container.addView(createProjectRow(item), rowLayoutParams(rowIndex))
            }
        }
        container.addView(bottomSpacer())
    }

    private fun createSegmentRow(showJoint: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(segmentButton("独立", selected = !showJoint) {
                setPersonalProjectsExpanded(true)
                setJointProjectsExpanded(false)
                render()
            })
            addView(segmentButton("联合", selected = showJoint) {
                setPersonalProjectsExpanded(false)
                setJointProjectsExpanded(true)
                render()
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                designPx(SEGMENT_HEIGHT_PX)
            ).apply {
                marginStart = designPx(SEGMENT_GAP_PX)
            })
        }
    }

    private fun segmentButton(
        label: String,
        selected: Boolean,
        onClick: () -> Unit
    ): TextView {
        return TextView(activity).apply {
            text = label
            includeFontPadding = false
            gravity = Gravity.CENTER
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
            setDesignTextSize(SEGMENT_TEXT_PX)
            setTypeface(typeface, Typeface.NORMAL)
            setPadding(designPx(if (selected) 62 else 0), 0, designPx(if (selected) 62 else 0), 0)
            if (selected) background = roundedPx(COLOR_SEGMENT_SELECTED, SEGMENT_HEIGHT_PX / 2)
            minWidth = designPx(if (selected) 210 else 96)
        }
    }

    private fun createProjectRow(item: IndexedProject): View {
        val project = item.project
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener { anchor ->
                showProjectActions(item.index, anchor)
                true
            }

            addView(projectThumbnail(project), LinearLayout.LayoutParams(
                designPx(THUMB_SIZE_PX),
                designPx(THUMB_SIZE_PX)
            ))

            addView(projectTextColumn(project), LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            ).apply {
                marginStart = designPx(TEXT_START_GAP_PX)
                marginEnd = designPx(TEXT_END_GAP_PX)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                gravity = Gravity.CENTER
                text = "›"
                setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
                setDesignTextSize(CHEVRON_TEXT_PX)
            }, LinearLayout.LayoutParams(
                designPx(CHEVRON_WIDTH_PX),
                LinearLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun projectTextColumn(project: AppProject): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = project.title.ifBlank { "项目名称" }
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_LIST_TITLE))
                setDesignTextSize(LIST_TITLE_TEXT_PX)
                setTypeface(typeface, Typeface.NORMAL)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectIntroduction(project)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
                setDesignTextSize(LIST_DESC_TEXT_PX)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = designPx(DESC_TOP_MARGIN_PX)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addMetaText("创建者：${projectOwner(project)}")
                addMetaText("成员：${projectMemberCount(project)}", marginStartPx = META_GAP_PX)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = designPx(META_TOP_MARGIN_PX)
            })
        }
    }

    private fun LinearLayout.addMetaText(value: String, marginStartPx: Int = 0) {
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = value
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
            setDesignTextSize(META_TEXT_PX)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = designPx(marginStartPx)
        })
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = roundedPx(COLOR_THUMB_BG, THUMB_RADIUS_PX)
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
            }
        }
    }

    private fun createEmptyState(showJoint: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            isClickable = !showJoint
            foreground = if (showJoint) null else selectableForeground()
            if (!showJoint) setOnClickListener { showCreateProjectDialog() }

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = if (showJoint) "暂无联合项目" else "还没有项目，点击 + 创建"
                setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
                setDesignTextSize(EMPTY_TEXT_PX)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun segmentLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            designPx(SEGMENT_HEIGHT_PX)
        ).apply {
            marginStart = designPx(SEGMENT_SIDE_PX)
            marginEnd = designPx(SEGMENT_SIDE_PX)
            topMargin = designPx(SEGMENT_TOP_MARGIN_PX)
        }
    }

    private fun firstRowLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            designPx(EMPTY_HEIGHT_PX)
        ).apply {
            marginStart = designPx(ROW_SIDE_PX)
            marginEnd = designPx(ROW_END_PX)
            topMargin = designPx(FIRST_ROW_TOP_MARGIN_PX)
        }
    }

    private fun rowLayoutParams(index: Int): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            designPx(ROW_HEIGHT_PX)
        ).apply {
            marginStart = designPx(ROW_SIDE_PX)
            marginEnd = designPx(ROW_END_PX)
            topMargin = designPx(if (index == 0) FIRST_ROW_TOP_MARGIN_PX else ROW_GAP_PX)
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

    private fun cleanProjectText(value: String?): String? {
        val text = value?.trim().orEmpty()
        return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun bottomSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                designPx(BOTTOM_SPACER_PX)
            )
        }
    }

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private fun TextView.setDesignTextSize(value: Int) {
        setTextSize(TypedValue.COMPLEX_UNIT_PX, designPx(value).toFloat())
    }

    private fun roundedPx(color: String, radiusPx: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            cornerRadius = designPx(radiusPx).toFloat()
        }
    }

    private fun rounded(color: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private companion object {
        const val COLOR_BG = "#000000"
        const val COLOR_SEGMENT_SELECTED = "#1A1A1A"
        const val COLOR_TEXT_PRIMARY = "#D9D9D9"
        const val COLOR_TEXT_LIST_TITLE = "#FFFFFF"
        const val COLOR_TEXT_PLACEHOLDER = "#AFAFAF"
        const val COLOR_THUMB_BG = "#FFFFFF"

        const val DESIGN_WIDTH_PX = 1272
        const val SEGMENT_SIDE_PX = 84
        const val ROW_SIDE_PX = 112
        const val ROW_END_PX = 92
        const val SEGMENT_TOP_MARGIN_PX = 154
        const val SEGMENT_HEIGHT_PX = 138
        const val SEGMENT_GAP_PX = 120
        const val FIRST_ROW_TOP_MARGIN_PX = 102
        const val ROW_HEIGHT_PX = 176
        const val ROW_GAP_PX = 142
        const val THUMB_SIZE_PX = 172
        const val THUMB_RADIUS_PX = 10
        const val TEXT_START_GAP_PX = 58
        const val TEXT_END_GAP_PX = 44
        const val DESC_TOP_MARGIN_PX = 17
        const val META_TOP_MARGIN_PX = 18
        const val META_GAP_PX = 112
        const val CHEVRON_WIDTH_PX = 52
        const val EMPTY_HEIGHT_PX = 520
        const val BOTTOM_SPACER_PX = 120

        const val SEGMENT_TEXT_PX = 54
        const val LIST_TITLE_TEXT_PX = 43
        const val LIST_DESC_TEXT_PX = 43
        const val META_TEXT_PX = 35
        const val EMPTY_TEXT_PX = 40
        const val CHEVRON_TEXT_PX = 58
    }
}
