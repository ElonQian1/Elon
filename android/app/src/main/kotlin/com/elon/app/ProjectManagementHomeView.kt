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
        container.setBackgroundColor(Color.parseColor(COLOR_APP_BG))

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

        val allProjects = personal + joint
        if (allProjects.isEmpty()) {
            container.addView(createEmptyProjectCard(), cardLayoutParams())
        } else {
            allProjects.forEach { item ->
                container.addView(createProjectCard(item), cardLayoutParams())
            }
        }
        container.addView(bottomSpacer())
    }

    private fun createProjectCard(item: IndexedProject): View {
        val project = item.project
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = rect(COLOR_CARD_BODY, PROJECT_CARD_RADIUS_DP)
            clipToOutline = true
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener { anchor ->
                showProjectActions(item.index, anchor)
                true
            }

            addView(createCardHeader(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_HEADER_HEIGHT_DP)
            ))
            addView(createCardBody(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(CARD_BODY_HEIGHT_DP)
            ))
        }
    }

    private fun createCardHeader(project: AppProject): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            background = topRoundedRect(COLOR_CARD_HEADER, PROJECT_CARD_RADIUS_DP)

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = project.title.ifBlank { "未命名项目" }
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_TITLE_TEXT_SP)
                setTypeface(typeface, Typeface.BOLD)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectMeta(project)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_META_TEXT_SP)
            }, LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.WRAP_CONTENT,
                1.45f
            ).apply {
                marginStart = dp(18)
            })
        }
    }

    private fun createCardBody(project: AppProject): View {
        return FrameLayout(activity).apply {
            background = rect(COLOR_CARD_BODY, 0)

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL

                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL

                    addView(projectThumbnail(project), LinearLayout.LayoutParams(
                        dp(THUMB_SIZE_DP),
                        dp(THUMB_SIZE_DP)
                    ).apply {
                        marginEnd = dp(14)
                    })

                    addView(LinearLayout(activity).apply {
                        orientation = LinearLayout.VERTICAL
                        addProjectDetailText("创建者：${projectOwner(project)}")
                        addProjectDetailText("成员：${projectMemberCount(project)}")
                    }, LinearLayout.LayoutParams(
                        0,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        1f
                    ))
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ))

                addView(View(activity).apply {
                    setBackgroundColor(Color.parseColor(COLOR_DIVIDER))
                    alpha = 0.74f
                }, LinearLayout.LayoutParams(
                    dp(DIVIDER_WIDTH_DP),
                    dp(1)
                ).apply {
                    topMargin = dp(12)
                })

                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "简介：${projectIntroduction(project)}"
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_DETAIL_TEXT_SP)
                    setLineSpacing(dp(2).toFloat(), 1.0f)
                }, LinearLayout.LayoutParams(
                    dp(INTRO_WIDTH_DP),
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(11)
                })
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(24)
                rightMargin = dp(132)
                topMargin = dp(35)
            })

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectTime(project)
                maxLines = 1
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_META_TEXT_SP)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.END or Gravity.TOP
            ).apply {
                rightMargin = dp(24)
                topMargin = dp(42)
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
            setTextSize(TypedValue.COMPLEX_UNIT_SP, CARD_DETAIL_TEXT_SP)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            bottomMargin = dp(9)
        })
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor(COLOR_THUMB_BG))
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

    private fun createEmptyProjectCard(): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            background = rect(COLOR_CARD_BODY, PROJECT_CARD_RADIUS_DP)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreateProjectDialog() }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "+"
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor(COLOR_TEXT_SECONDARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 38f)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(EMPTY_CARD_HEIGHT_DP)
            ))
        }
    }

    private fun cardLayoutParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(16)
            marginEnd = dp(16)
            topMargin = dp(8)
        }
    }

    private fun projectMeta(project: AppProject): String {
        val kind = if (project.isJointDevelopmentProject()) "联合项目" else "个人独立"
        val stage = projectStageText(project.stage)
        val workspace = projectWorkspaceText(project, stage)
        return "$kind · ${project.displayConversationCount()}个会话 · $workspace"
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
            "done" -> "交付完成"
            "failed" -> "需要处理"
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

    private fun projectIntroduction(project: AppProject): String {
        return cleanProjectText(project.projectCardIntroduction())
            ?: cleanProjectText(project.subtitle)
            ?: "暂无简介"
    }

    private fun cleanProjectText(value: String?): String? {
        val text = value?.trim().orEmpty()
        return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun projectTime(project: AppProject): String {
        if (project.updatedAt <= 0L) return "时间"
        return formatTime(project.updatedAt).ifBlank { "时间" }
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

    private fun topRoundedRect(color: String, radiusDp: Int): GradientDrawable {
        val radius = dp(radiusDp).toFloat()
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            cornerRadii = floatArrayOf(radius, radius, radius, radius, 0f, 0f, 0f, 0f)
        }
    }

    private companion object {
        const val COLOR_APP_BG = "#101010"
        const val COLOR_CARD_HEADER = "#202024"
        const val COLOR_CARD_BODY = "#2A2A2A"
        const val COLOR_TEXT_PRIMARY = "#D6D6D6"
        const val COLOR_TEXT_SECONDARY = "#A8A8A8"
        const val COLOR_DIVIDER = "#A8A8A8"
        const val COLOR_THUMB_BG = "#D2D2D2"

        const val PROJECT_CARD_RADIUS_DP = 12
        const val CARD_HEADER_HEIGHT_DP = 54
        const val CARD_BODY_HEIGHT_DP = 184
        const val THUMB_SIZE_DP = 40
        const val DIVIDER_WIDTH_DP = 196
        const val INTRO_WIDTH_DP = 206
        const val EMPTY_CARD_HEIGHT_DP = 238

        const val CARD_TITLE_TEXT_SP = 19.5f
        const val CARD_META_TEXT_SP = 15f
        const val CARD_DETAIL_TEXT_SP = 13.2f
    }
}
