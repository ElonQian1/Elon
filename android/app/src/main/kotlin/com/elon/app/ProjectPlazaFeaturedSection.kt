package com.elon.app

import android.content.SharedPreferences
import android.graphics.Color
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
    @Suppress("unused") private val isProjectJoined: (StoreProject) -> Boolean
) {
    fun build(projects: List<StoreProject>): View = ProjectPlazaCarousel(activity).apply {
        configureContentInsets(dp(PLAZA_SIDE_MARGIN_DP), dp(PLAZA_TRAILING_PADDING_DP))
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            projects.forEachIndexed { index, project ->
                val card = buildCard(project).apply {
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

    fun heightPx(): Int = cardHeightPx()

    private fun buildCard(project: StoreProject) = FrameLayout(activity).apply {
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
                    addView(reactionImageButton(project, "liked", R.drawable.project_plaza_ui4_heart, "点赞"), LinearLayout.LayoutParams(dp(30), dp(36)).apply {
                        marginStart = dp(1)
                    })
                }, LinearLayout.LayoutParams(0, dp(36), 1f))
            })
            addView(TextView(activity).apply {
                text = project.displayTitle()
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_PRIMARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                topMargin = dp(11)
            })
            addView(TextView(activity).apply {
                text = project.description?.takeIf { it.isNotBlank() } ?: "这个项目还没有填写简介。"
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(COLOR_TEXT_TERTIARY))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f)
                setLineSpacing(0f, 1.05f)
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(28)).apply {
                topMargin = dp(6)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                repeat(2) { index ->
                    addView(mediaPlaceholder(), LinearLayout.LayoutParams(dp(60), dp(85)).apply {
                        if (index == 1) marginStart = dp(7)
                    })
                }
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(85)).apply {
                topMargin = dp(9)
            })
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

    private fun cardWidthPx(): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: dp(360)
        return (width * FEATURED_CARD_WIDTH_FRACTION).roundToInt()
    }

    private fun cardHeightPx(): Int = (cardWidthPx() * FEATURED_CARD_HEIGHT_RATIO).roundToInt()

    private fun rect(color: String, radiusDp: Int = 0): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
            if (radiusDp > 0) cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun designPx(value: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.takeIf { it > 0 } ?: DESIGN_WIDTH_PX
        return (value * (width / DESIGN_WIDTH_PX.toFloat())).roundToInt()
    }

    private companion object {
        const val COLOR_TEXT_PRIMARY = "#D9D9D9"
        const val COLOR_TEXT_TERTIARY = "#777777"
        const val PLAZA_SIDE_MARGIN_DP = 20
        const val PLAZA_TRAILING_PADDING_DP = 98
        const val FEATURED_CARD_GAP_DP = 9
        const val FEATURED_CARD_WIDTH_FRACTION = 0.6564706f
        const val FEATURED_CARD_HEIGHT_RATIO = 1.1266428f
        const val DESIGN_WIDTH_PX = 1275
    }
}
