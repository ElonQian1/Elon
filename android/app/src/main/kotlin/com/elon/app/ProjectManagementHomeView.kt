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
                dp(SEGMENT_HEIGHT_DP)
            ).apply {
                marginStart = dp(38)
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
            setTextSize(TypedValue.COMPLEX_UNIT_SP, SEGMENT_TEXT_SP)
            setTypeface(typeface, Typeface.NORMAL)
            setPadding(dp(if (selected) 21 else 0), 0, dp(if (selected) 21 else 0), 0)
            if (selected) background = rounded(COLOR_SEGMENT_SELECTED, SEGMENT_HEIGHT_DP / 2)
            minWidth = dp(if (selected) 72 else 64)
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
                dp(THUMB_SIZE_DP),
                dp(THUMB_SIZE_DP)
            ))

            addView(projectTextColumn(project), LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1f
            ).apply {
                marginStart = dp(20)
                marginEnd = dp(14)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                gravity = Gravity.CENTER
                text = "›"
                setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, CHEVRON_TEXT_SP)
            }, LinearLayout.LayoutParams(
                dp(24),
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
                setTextSize(TypedValue.COMPLEX_UNIT_SP, LIST_TITLE_TEXT_SP)
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
                setTextSize(TypedValue.COMPLEX_UNIT_SP, LIST_DESC_TEXT_SP)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(7)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addMetaText("创建者：${projectOwner(project)}")
                addMetaText("成员：${projectMemberCount(project)}", marginStartDp = 34)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(8)
            })
        }
    }

    private fun LinearLayout.addMetaText(value: String, marginStartDp: Int = 0) {
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = value
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(Color.parseColor(COLOR_TEXT_PLACEHOLDER))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, META_TEXT_SP)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(marginStartDp)
        })
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = rounded(COLOR_THUMB_BG, THUMB_RADIUS_DP)
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
                setTextSize(TypedValue.COMPLEX_UNIT_SP, EMPTY_TEXT_SP)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun segmentLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(SEGMENT_HEIGHT_DP)
        ).apply {
            marginStart = dp(PAGE_SIDE_DP)
            marginEnd = dp(PAGE_SIDE_DP)
            topMargin = dp(SEGMENT_TOP_MARGIN_DP)
        }
    }

    private fun firstRowLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(EMPTY_HEIGHT_DP)
        ).apply {
            marginStart = dp(PAGE_SIDE_DP)
            marginEnd = dp(PAGE_SIDE_DP)
            topMargin = dp(FIRST_ROW_TOP_MARGIN_DP)
        }
    }

    private fun rowLayoutParams(index: Int): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(ROW_HEIGHT_DP)
        ).apply {
            marginStart = dp(PAGE_SIDE_DP)
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

    private fun cleanProjectText(value: String?): String? {
        val text = value?.trim().orEmpty()
        return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun bottomSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            )
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

        const val PAGE_SIDE_DP = 32
        const val ROW_END_DP = 28
        const val SEGMENT_TOP_MARGIN_DP = 56
        const val SEGMENT_HEIGHT_DP = 52
        const val FIRST_ROW_TOP_MARGIN_DP = 48
        const val ROW_HEIGHT_DP = 106
        const val ROW_GAP_DP = 20
        const val THUMB_SIZE_DP = 56
        const val THUMB_RADIUS_DP = 6
        const val EMPTY_HEIGHT_DP = 220

        const val SEGMENT_TEXT_SP = 18f
        const val LIST_TITLE_TEXT_SP = 17f
        const val LIST_DESC_TEXT_SP = 17f
        const val META_TEXT_SP = 14f
        const val EMPTY_TEXT_SP = 16f
        const val CHEVRON_TEXT_SP = 34f
    }
}
