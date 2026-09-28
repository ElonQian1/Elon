package com.elon.app

import android.content.SharedPreferences
import android.content.Context
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
import kotlin.math.roundToInt

internal class ProjectPlazaFeaturedSection(
    private val activity: Context,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val reactionPrefs: SharedPreferences,
    private val openProjectSpace: (StoreProject) -> Unit,
    private val isProjectJoined: (StoreProject) -> Boolean,
    private val primaryAction: (StoreProject) -> ProjectPlazaPrimaryAction,
    private val onPrimaryAction: (StoreProject) -> Unit
) {
    fun build(projects: List<StoreProject>): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(buildSectionHeading(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        addView(buildCarousel(projects), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
    }

    private fun buildSectionHeading() = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(48)
        setPadding(dp(SIDE_MARGIN_DP), 0, dp(SIDE_MARGIN_DP), 0)
        addView(label("推荐", 16f, R.color.elon_plaza_text_primary, true))
        addView(label("左右滑动", 14f, R.color.elon_plaza_text_quiet), LinearLayout.LayoutParams(
            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f
        ).apply { marginStart = dp(9) })
    }

    private fun buildCarousel(projects: List<StoreProject>) = ProjectPlazaCarousel(activity).apply {
        configureContentInsets(dp(SIDE_MARGIN_DP), dp(TRAILING_PADDING_DP))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEach { project ->
                addView(buildCard(project), LinearLayout.LayoutParams(cardWidthPx(), LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                    marginEnd = dp(CARD_GAP_DP)
                })
            }
        })
    }

    private fun buildCard(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        minimumHeight = dp(CARD_HEIGHT_DP)
        background = rounded(R.color.elon_plaza_surface_card, CARD_RADIUS_DP, R.color.elon_plaza_border)
        clipToOutline = true
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "查看${project.displayTitle()}"
        setOnClickListener { openProjectSpace(project) }
        addView(buildCardBody(project), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        addView(buildActions(project), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = dp(CONTENT_PADDING_DP)
            marginEnd = dp(CONTENT_PADDING_DP)
            bottomMargin = dp(ACTION_BOTTOM_DP)
            topMargin = dp(16)
        })
    }

    private fun buildCardBody(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(CONTENT_PADDING_DP), dp(CONTENT_PADDING_DP), dp(CONTENT_PADDING_DP), 0)
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.TOP
            addView(projectPlazaProjectCover(activity, project, dp(COVER_SIZE_DP), dp(COVER_RADIUS_DP).toFloat(), 24f),
                LinearLayout.LayoutParams(dp(COVER_SIZE_DP), dp(COVER_SIZE_DP)))
            addView(buildStatusPill(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(12) })
        })
        addView(label(project.displayTitle(), 20f, R.color.elon_plaza_text_primary, true).apply {
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
            topMargin = dp(18)
        })
        addView(label(project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。", 13f,
            R.color.elon_plaza_text_secondary).apply {
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            setLineSpacing(dp(2).toFloat(), 1f)
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
            topMargin = dp(8)
        })
        addView(label("${project.memberCount.coerceAtLeast(0)} 协同者   ·   ${favoriteCount(project)} 收藏", 12f,
            R.color.elon_plaza_text_secondary), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(18) })
        addView(View(activity).apply { setBackgroundColor(activity.elonColor(R.color.elon_plaza_divider)) },
            LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(1)).apply { topMargin = dp(16) })
    }

    private fun buildStatusPill(project: StoreProject) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER
        minimumHeight = dp(STATUS_HEIGHT_DP)
        background = rounded(R.color.mobile_surface, STATUS_HEIGHT_DP / 2, R.color.mobile_outline_variant)
        setPadding(dp(12), dp(6), dp(12), dp(6))
        val status = projectPlazaAccessStatus(project, isProjectJoined(project))
        val statusColor = when (status.tone) {
            ProjectPlazaTone.SUCCESS -> R.color.elon_plaza_status_success
            ProjectPlazaTone.DANGER -> R.color.elon_plaza_status_danger
            ProjectPlazaTone.NEUTRAL -> R.color.elon_plaza_text_secondary
        }
        addView(View(activity).apply { background = rounded(statusColor, 4) },
            LinearLayout.LayoutParams(dp(7), dp(7)))
        addView(label(status.label, 14f, statusColor), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { marginStart = dp(7) })
    }

    private fun buildActions(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        val action = primaryAction(project)
        addView(label(action.label, 14f, if (action.enabled) R.color.elon_plaza_action_ink else R.color.mobile_on_surface_variant, true).apply {
            minimumHeight = dp(ACTION_HEIGHT_DP)
            setPadding(dp(8), dp(8), dp(8), dp(8))
            gravity = Gravity.CENTER
            background = rounded(if (action.enabled) R.color.elon_plaza_action else R.color.mobile_surface_container,
                ACTION_HEIGHT_DP / 2, R.color.elon_plaza_action_border)
            foreground = if (action.enabled) selectableForeground() else null
            setOnClickListener { onPrimaryAction(project) }
            isClickable = action.enabled
            isEnabled = action.enabled
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
        addView(LinearLayout(activity).apply {
            gravity = Gravity.END
            addView(reactionButton(project, "favorite", R.drawable.project_plaza_ui5_star, "收藏"), LinearLayout.LayoutParams(dp(48), dp(48)))
            addView(reactionButton(project, "liked", R.drawable.project_plaza_ui4_heart, "点赞"), LinearLayout.LayoutParams(dp(48), dp(48)))
        }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))
    }

    private fun reactionButton(project: StoreProject, key: String, drawableRes: Int, label: String) = FrameLayout(activity).apply {
        val icon = ImageView(activity).apply { setImageResource(drawableRes); scaleType = ImageView.ScaleType.CENTER_INSIDE }
        fun render() {
            val selected = reactionPrefs.getBoolean("${project.id}:$key", false)
            icon.alpha = if (selected) 1f else 0.7f
            contentDescription = if (selected) "取消$label" else label
        }
        isClickable = true
        foreground = selectableForeground()
        setOnClickListener {
            reactionPrefs.edit().putBoolean("${project.id}:$key", !reactionPrefs.getBoolean("${project.id}:$key", false)).apply()
            render()
        }
        addView(icon, FrameLayout.LayoutParams(dp(23), dp(23), Gravity.CENTER))
        render()
    }

    private fun label(textValue: String, sizeSp: Float, color: Int, bold: Boolean = false) = TextView(activity).apply {
        text = textValue
        includeFontPadding = false
        setTextColor(activity.elonColor(color))
        setTextSize(TypedValue.COMPLEX_UNIT_SP, sizeSp)
        if (bold) typeface = Typeface.DEFAULT_BOLD
    }

    private fun rounded(fill: Int, radiusDp: Int, stroke: Int? = null) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        setColor(activity.elonColor(fill))
        cornerRadius = dp(radiusDp).toFloat()
        stroke?.let { setStroke(dp(1), activity.elonColor(it)) }
    }

    private fun favoriteCount(project: StoreProject): String = when {
        (project.installCount ?: 0) >= 1000 -> "${(project.installCount ?: 0) / 100 / 10.0}k"
        (project.installCount ?: 0) > 0 -> project.installCount.toString()
        else -> "0"
    }

    private fun cardWidthPx(): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: dp(360)
        return (width * CARD_WIDTH_FRACTION).roundToInt()
    }

    private companion object {
        const val SIDE_MARGIN_DP = 22
        const val TRAILING_PADDING_DP = 88
        const val CARD_GAP_DP = 14
        const val CARD_WIDTH_FRACTION = 0.72f
        const val CARD_HEIGHT_DP = 310
        const val CARD_RADIUS_DP = 24
        const val CONTENT_PADDING_DP = 23
        const val ACTION_BOTTOM_DP = 16
        const val COVER_SIZE_DP = 58
        const val COVER_RADIUS_DP = 16
        const val ACTION_HEIGHT_DP = 48
        const val STATUS_HEIGHT_DP = 30
    }
}
