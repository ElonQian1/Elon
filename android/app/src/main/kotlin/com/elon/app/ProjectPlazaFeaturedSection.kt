package com.elon.app

import android.content.SharedPreferences
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.Layout
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

internal class ProjectPlazaFeaturedSection(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val reactionPrefs: SharedPreferences,
    private val openProjectSpace: (StoreProject) -> Unit,
    private val isProjectJoined: (StoreProject) -> Boolean,
    private val primaryAction: (StoreProject) -> ProjectPlazaPrimaryAction,
    private val onPrimaryAction: (StoreProject) -> Unit
) {
    private var positionIndicator: TextView? = null

    fun build(projects: List<StoreProject>): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(buildSectionHeading(projects.size), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(SECTION_HEADING_HEIGHT_DP)
        ))
        addView(buildCarousel(projects), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            cardHeightPx()
        ))
    }

    fun heightPx(): Int = dp(SECTION_HEADING_HEIGHT_DP) + cardHeightPx()

    private fun buildSectionHeading(projectCount: Int) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(PLAZA_SIDE_MARGIN_DP), 0, dp(PLAZA_SIDE_MARGIN_DP), 0)
        addView(TextView(activity).apply {
            text = "精选项目"
            includeFontPadding = false
            setTextColor(activity.elonColor(R.color.elon_plaza_text_primary))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            typeface = Typeface.DEFAULT_BOLD
            gravity = Gravity.CENTER_VERTICAL
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        addView(TextView(activity).apply {
            positionIndicator = this
            text = if (projectCount > 0) "01 / ${projectCount.toString().padStart(2, '0')}" else "00 / 00"
            includeFontPadding = false
            setTextColor(activity.elonColor(R.color.elon_plaza_text_quiet))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
            typeface = Typeface.MONOSPACE
            gravity = Gravity.CENTER_VERTICAL
            contentDescription = "精选项目位置"
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.MATCH_PARENT
        ))
    }

    private fun buildCarousel(projects: List<StoreProject>) = ProjectPlazaCarousel(activity).apply {
        configureContentInsets(dp(PLAZA_SIDE_MARGIN_DP), dp(PLAZA_TRAILING_PADDING_DP))
        onActiveCardChanged = { index ->
            positionIndicator?.text = "${(index + 1).toString().padStart(2, '0')} / ${projects.size.toString().padStart(2, '0')}"
        }
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEachIndexed { index, project ->
                val card = buildCard(project, index).apply {
                    val initialScale = if (index == 0) 1f else PROJECT_PLAZA_CARD_MIN_SCALE
                    scaleX = initialScale
                    scaleY = initialScale
                }
                addView(card, LinearLayout.LayoutParams(cardWidthPx(), cardHeightPx()).apply {
                    marginEnd = dp(FEATURED_CARD_GAP_DP)
                })
            }
        })
        post { refreshCardScales() }
    }

    private fun buildCard(project: StoreProject, index: Int) = FrameLayout(activity).apply {
        background = ProjectPlazaMetalPanelDrawable(activity)
        clipToOutline = true
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "查看${project.displayTitle()}"
        setOnClickListener { openProjectSpace(project) }
        addView(buildCardHeader(project, index), FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            dp(CARD_HEADER_HEIGHT_DP),
            Gravity.TOP
        ))
        addView(buildCardBody(project), FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT,
            Gravity.TOP
        ).apply {
            topMargin = dp(CARD_HEADER_HEIGHT_DP)
        })
        addView(buildActions(project), FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            dp(ACTION_HEIGHT_DP),
            Gravity.BOTTOM
        ).apply {
            marginStart = dp(CARD_CONTENT_PADDING_DP)
            marginEnd = dp(CARD_CONTENT_PADDING_DP)
            bottomMargin = dp(CARD_ACTION_BOTTOM_DP)
        })
    }

    private fun buildCardHeader(project: StoreProject, index: Int) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        background = metalHeader()
        setPadding(dp(CARD_CONTENT_PADDING_DP), 0, dp(CARD_CONTENT_PADDING_DP), 0)
        addView(View(activity).apply {
            background = rect(activity.elonColor(R.color.elon_plaza_signal), FEATURE_RAIL_WIDTH_DP / 2)
            contentDescription = null
        }, LinearLayout.LayoutParams(dp(FEATURE_RAIL_WIDTH_DP), dp(FEATURE_RAIL_HEIGHT_DP)).apply {
            marginEnd = dp(12)
        })
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(TextView(activity).apply {
                text = "精选节点"
                includeFontPadding = false
                setTextColor(activity.elonColor(R.color.elon_plaza_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
                typeface = Typeface.DEFAULT_BOLD
            })
            addView(TextView(activity).apply {
                text = "NODE ${(index + 1).toString().padStart(2, '0')}"
                includeFontPadding = false
                setTextColor(activity.elonColor(R.color.elon_plaza_text_secondary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 10f)
                typeface = Typeface.MONOSPACE
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(8)
            })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        val status = statusStyle(project)
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(View(activity).apply {
                background = rect(toneColor(status.tone), STATUS_DOT_DP / 2)
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(STATUS_DOT_DP), dp(STATUS_DOT_DP)))
            addView(TextView(activity).apply {
                text = status.label
                includeFontPadding = false
                setTextColor(activity.elonColor(R.color.elon_plaza_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(7)
            })
        })
    }

    private fun buildCardBody(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(
            dp(CARD_CONTENT_PADDING_DP),
            dp(CARD_CONTENT_PADDING_DP),
            dp(CARD_CONTENT_PADDING_DP),
            0
        )
        addView(buildIdentity(project))
        addView(View(activity).apply {
            background = rect(activity.elonColor(R.color.elon_plaza_divider))
            contentDescription = null
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(1)
        ).apply {
            topMargin = dp(18)
        })
        addView(buildFacts(project), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(12)
        })
    }

    private fun buildIdentity(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.TOP
        addView(projectPlazaProjectCover(
            activity = activity,
            project = project,
            sizePx = dp(COVER_SIZE_DP),
            radiusPx = dp(COVER_RADIUS_DP).toFloat(),
            fallbackTextSp = 28f
        ), LinearLayout.LayoutParams(dp(COVER_SIZE_DP), dp(COVER_SIZE_DP)))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                text = project.displayTitle()
                includeFontPadding = false
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(activity.elonColor(R.color.elon_plaza_text_primary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
                setLineSpacing(0f, 1.02f)
                breakStrategy = Layout.BREAK_STRATEGY_BALANCED
                typeface = Typeface.DEFAULT_BOLD
            })
            addView(TextView(activity).apply {
                text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
                includeFontPadding = false
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(activity.elonColor(R.color.elon_plaza_text_secondary))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
                setLineSpacing(0f, 1.08f)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(8)
            })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
            marginStart = dp(16)
            topMargin = dp(2)
        })
    }

    private fun buildFacts(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        val build = projectPlazaBuildStatus(project.lastTaskStatus)
        addView(factColumn("创建者", project.ownerLabel()), LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            1.2f
        ))
        addView(factColumn("成员", "${project.memberCount.coerceAtLeast(0)} 人"), LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            0.8f
        ))
        addView(factColumn("最近构建", build.label, toneColor(build.tone)), LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            1.2f
        ))
    }

    private fun factColumn(
        label: String,
        value: String,
        valueColor: Int = activity.elonColor(R.color.elon_plaza_text_primary)
    ) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(TextView(activity).apply {
            text = label
            includeFontPadding = false
            setTextColor(activity.elonColor(R.color.elon_plaza_text_secondary))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 11f)
        })
        addView(TextView(activity).apply {
            text = value
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(valueColor)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(3)
        })
    }

    private fun buildActions(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        val action = primaryAction(project)
        addView(TextView(activity).apply {
            text = action.label
            gravity = Gravity.CENTER
            includeFontPadding = false
            background = if (action.enabled) {
                ProjectPlazaMetalActionDrawable(activity)
            } else {
                rect(activity.elonColor(R.color.elon_plaza_surface_search), ACTION_HEIGHT_DP / 2)
            }
            foreground = if (action.enabled) selectableForeground() else null
            isClickable = action.enabled
            isEnabled = action.enabled
            setTextColor(
                activity.elonColor(
                    if (action.enabled) R.color.elon_plaza_action_ink else R.color.elon_plaza_text_quiet
                )
            )
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f)
            typeface = Typeface.DEFAULT_BOLD
            contentDescription = "${action.label}${project.displayTitle()}"
            setOnClickListener { onPrimaryAction(project) }
        }, LinearLayout.LayoutParams(0, dp(ACTION_HEIGHT_DP), 1f))
        addView(reactionImageButton(project, "favorite", R.drawable.project_plaza_ui5_star, "收藏"), LinearLayout.LayoutParams(
            dp(ACTION_HEIGHT_DP),
            dp(ACTION_HEIGHT_DP)
        ).apply {
            marginStart = dp(8)
        })
        addView(reactionImageButton(project, "liked", R.drawable.project_plaza_ui4_heart, "点赞"), LinearLayout.LayoutParams(
            dp(ACTION_HEIGHT_DP),
            dp(ACTION_HEIGHT_DP)
        ).apply {
            marginStart = dp(8)
        })
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
        val iconWidth = if (drawableRes == R.drawable.project_plaza_ui5_star) dp(22) else dp(21)
        val iconHeight = dp(20)
        addView(icon, FrameLayout.LayoutParams(iconWidth, iconHeight, Gravity.CENTER))
        render()
    }

    private fun statusStyle(project: StoreProject): ProjectPlazaStatus =
        projectPlazaAccessStatus(project, isProjectJoined(project))

    private fun toneColor(tone: ProjectPlazaTone): Int = when (tone) {
        ProjectPlazaTone.SUCCESS -> activity.elonColor(R.color.elon_plaza_status_success)
        ProjectPlazaTone.DANGER -> activity.elonColor(R.color.elon_plaza_status_danger)
        ProjectPlazaTone.NEUTRAL -> activity.elonColor(R.color.elon_plaza_text_quiet)
    }

    private fun StoreProject.ownerLabel(): String = ownerAccount.trim()
        .takeIf { it.isNotBlank() && it != "?" }
        ?: "未知"

    private fun cardWidthPx(): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: dp(360)
        return (width * FEATURED_CARD_WIDTH_FRACTION).roundToInt()
    }

    private fun cardHeightPx(): Int {
        val baseHeight = (cardWidthPx() * FEATURED_CARD_HEIGHT_RATIO).roundToInt()
        val fontScale = activity.resources.configuration.fontScale
        return baseHeight + if (fontScale >= LARGE_TEXT_SCALE) dp(LARGE_TEXT_EXTRA_HEIGHT_DP) else 0
    }

    private fun rect(color: Int, radiusDp: Int = 0, strokeColor: Int? = null): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(color)
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
            strokeColor?.let { setStroke(dp(1), it) }
        }
    }

    private fun metalHeader(): GradientDrawable = GradientDrawable(
        GradientDrawable.Orientation.TL_BR,
        intArrayOf(
            activity.elonColor(R.color.elon_plaza_surface_card_high),
            activity.elonColor(R.color.elon_plaza_surface_header),
            activity.elonColor(R.color.elon_plaza_surface_card),
        )
    ).apply {
        shape = GradientDrawable.RECTANGLE
    }

    private companion object {
        const val PLAZA_SIDE_MARGIN_DP = 20
        const val PLAZA_TRAILING_PADDING_DP = 98
        const val SECTION_HEADING_HEIGHT_DP = 42
        const val FEATURED_CARD_GAP_DP = 10
        const val FEATURED_CARD_WIDTH_FRACTION = 0.6871795f
        const val FEATURED_CARD_HEIGHT_RATIO = 1.2014925f
        const val CARD_HEADER_HEIGHT_DP = 54
        const val FEATURE_RAIL_WIDTH_DP = 3
        const val FEATURE_RAIL_HEIGHT_DP = 24
        const val CARD_CONTENT_PADDING_DP = 18
        const val CARD_ACTION_BOTTOM_DP = 16
        const val COVER_SIZE_DP = 76
        const val COVER_RADIUS_DP = 12
        const val ACTION_HEIGHT_DP = 48
        const val STATUS_DOT_DP = 7
        const val LARGE_TEXT_SCALE = 1.15f
        const val LARGE_TEXT_EXTRA_HEIGHT_DP = 54
    }
}
