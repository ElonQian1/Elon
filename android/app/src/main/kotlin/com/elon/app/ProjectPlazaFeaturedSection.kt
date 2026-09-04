package com.elon.app

import android.content.SharedPreferences
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
    fun build(projects: List<StoreProject>): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        addView(buildSectionHeading(), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(SECTION_HEADING_HEIGHT_DP)
        ))
        addView(buildCarousel(projects), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            cardHeightPx()
        ))
    }

    fun heightPx(): Int = dp(SECTION_HEADING_HEIGHT_DP) + cardHeightPx()

    private fun buildSectionHeading() = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(SIDE_MARGIN_DP), 0, dp(SIDE_MARGIN_DP), 0)
        addView(label("推荐", 16f, R.color.elon_plaza_text_primary, true))
        addView(label("左右滑动探索标杆", 11f, R.color.elon_plaza_text_quiet), LinearLayout.LayoutParams(
            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f
        ).apply { marginStart = dp(9) })
        addView(label("换一换 ↻", 12f, R.color.elon_plaza_signal).apply {
            gravity = Gravity.END or Gravity.CENTER_VERTICAL
        }, LinearLayout.LayoutParams(dp(72), dp(42)).apply {
            gravity = Gravity.CENTER_VERTICAL
        })
    }

    private fun buildCarousel(projects: List<StoreProject>) = ProjectPlazaCarousel(activity).apply {
        configureContentInsets(dp(SIDE_MARGIN_DP), dp(TRAILING_PADDING_DP))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEach { project ->
                addView(buildCard(project), LinearLayout.LayoutParams(cardWidthPx(), cardHeightPx()).apply {
                    marginEnd = dp(CARD_GAP_DP)
                })
            }
        })
    }

    private fun buildCard(project: StoreProject) = FrameLayout(activity).apply {
        background = rounded(R.color.elon_plaza_surface_card, CARD_RADIUS_DP, R.color.elon_plaza_border)
        clipToOutline = true
        isClickable = true
        foreground = selectableForeground()
        contentDescription = "查看${project.displayTitle()}"
        setOnClickListener { openProjectSpace(project) }
        addView(buildCardBody(project), FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.TOP
        ))
        addView(buildActions(project), FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT, dp(ACTION_HEIGHT_DP), Gravity.BOTTOM
        ).apply {
            marginStart = dp(CONTENT_PADDING_DP)
            marginEnd = dp(CONTENT_PADDING_DP)
            bottomMargin = dp(ACTION_BOTTOM_DP)
        })
    }

    private fun buildCardBody(project: StoreProject) = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(CONTENT_PADDING_DP), dp(CONTENT_PADDING_DP), dp(CONTENT_PADDING_DP), 0)
        addView(LinearLayout(activity).apply {
            gravity = Gravity.TOP
            addView(projectPlazaProjectCover(activity, project, dp(COVER_SIZE_DP), dp(COVER_RADIUS_DP).toFloat(), 24f),
                LinearLayout.LayoutParams(dp(COVER_SIZE_DP), dp(COVER_SIZE_DP)))
            addView(View(activity), LinearLayout.LayoutParams(0, 1, 1f))
            addView(buildStatusPill(project), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, dp(STATUS_HEIGHT_DP)
            ))
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
        background = rounded(R.color.elon_plaza_status_surface, STATUS_HEIGHT_DP / 2, R.color.elon_plaza_status_border)
        setPadding(dp(12), 0, dp(12), 0)
        addView(View(activity).apply { background = rounded(R.color.elon_plaza_status_success, 4) },
            LinearLayout.LayoutParams(dp(7), dp(7)))
        val status = projectPlazaAccessStatus(project, isProjectJoined(project))
        addView(label(if (status.tone == ProjectPlazaTone.SUCCESS) "运行中" else status.label, 11f,
            R.color.elon_plaza_status_success), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { marginStart = dp(7) })
    }

    private fun buildActions(project: StoreProject) = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        val action = primaryAction(project)
        addView(label(action.label, 13f, if (action.enabled) R.color.elon_plaza_text_primary else R.color.elon_plaza_text_quiet, true).apply {
            gravity = Gravity.CENTER
            background = rounded(R.color.elon_plaza_action, ACTION_HEIGHT_DP / 2, R.color.elon_plaza_action_border)
            foreground = if (action.enabled) selectableForeground() else null
            isClickable = action.enabled
            isEnabled = action.enabled
            setOnClickListener { onPrimaryAction(project) }
        }, LinearLayout.LayoutParams(dp(104), dp(ACTION_HEIGHT_DP)))
        addView(View(activity), LinearLayout.LayoutParams(0, 1, 1f))
        addView(reactionButton(project, "favorite", R.drawable.project_plaza_ui5_star, "收藏"), LinearLayout.LayoutParams(dp(44), dp(44)))
        addView(reactionButton(project, "liked", R.drawable.project_plaza_ui4_heart, "点赞"), LinearLayout.LayoutParams(dp(44), dp(44)))
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

    private fun cardHeightPx(): Int = dp(CARD_HEIGHT_DP)

    private companion object {
        const val SIDE_MARGIN_DP = 22
        const val TRAILING_PADDING_DP = 88
        const val SECTION_HEADING_HEIGHT_DP = 46
        const val CARD_GAP_DP = 14
        const val CARD_WIDTH_FRACTION = 0.72f
        const val CARD_HEIGHT_DP = 310
        const val CARD_RADIUS_DP = 24
        const val CONTENT_PADDING_DP = 23
        const val ACTION_BOTTOM_DP = 16
        const val COVER_SIZE_DP = 58
        const val COVER_RADIUS_DP = 16
        const val ACTION_HEIGHT_DP = 42
        const val STATUS_HEIGHT_DP = 30
    }
}
