package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import java.text.DateFormat
import java.util.Date
import kotlin.math.sin

internal class MainHomeRows(
    private val activity: AppCompatActivity,
    private val timeFormatter: DateFormat,
    private val activeProjectIndexProvider: () -> Int,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int) -> Unit,
    private val openConversation: (Int) -> Unit,
    private val showConversationActions: (Int) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private var conversationHomeRowAnimator: ValueAnimator? = null

    fun createProjectRow(index: Int, project: AppProject): View {
        val wrapper = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            ).apply {
                topMargin = if (index == 0) 0 else 1
            }
        }

        val row = LinearLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor(if (index == activeProjectIndexProvider()) "#292929" else "#202020"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(index) }
            setOnLongClickListener {
                showProjectActions(index)
                true
            }
        }

        row.addView(createAvatarView(project.title, 44, 18f))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(12)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = project.title
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = "${project.conversations.size} 个会话 · ${project.stage}"
            setTextColor(Color.parseColor("#A9A9A9"))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(8)
                topMargin = dp(17)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(project.updatedAt))
            setTextColor(Color.parseColor("#C4C4C4"))
            textSize = 13f
        })
        wrapper.addView(row)

        if (index == activeProjectIndexProvider()) {
            wrapper.addView(View(activity).apply {
                layoutParams = FrameLayout.LayoutParams(dp(8), dp(8)).apply {
                    gravity = Gravity.START or Gravity.TOP
                    leftMargin = dp(10)
                    topMargin = dp(10)
                }
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor("#FF4D4F"))
                }
            })
        }

        return wrapper
    }

    fun createConversationRow(index: Int, conversation: AppConversation, active: Boolean): View {
        val row = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(66)
            )
            setBackgroundColor(Color.parseColor("#242424"))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), 0, dp(14), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openConversation(index) }
            setOnLongClickListener {
                showConversationActions(index)
                true
            }
        }

        row.addView(createAvatarView(conversation.title, 44, 17f))

        val middle = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(10)
            }
            orientation = LinearLayout.VERTICAL
        }
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.title
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 16f
        })
        middle.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            }
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            maxLines = 1
            text = conversation.subtitle
            setTextColor(conversationSubtitleColor(conversation.subtitle))
            textSize = 13f
        })
        row.addView(middle)

        row.addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP
                marginStart = dp(7)
                topMargin = dp(16)
            }
            includeFontPadding = false
            text = timeFormatter.format(Date(conversation.updatedAt))
            setTextColor(Color.parseColor("#C4C4C4"))
            textSize = 12f
        })
        updateConversationRowShimmer(row, active, false)
        return row
    }

    fun updateConversationRowShimmer(row: View, active: Boolean, homeRow: Boolean) {
        if (active) {
            startConversationRowShimmer(row, homeRow)
        } else {
            stopConversationRowShimmer(row, homeRow)
        }
    }

    fun createConversationDivider(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                marginStart = dp(68)
            }
            setBackgroundColor(Color.parseColor("#343434"))
        }
    }

    fun cancelHomeRowShimmer() {
        conversationHomeRowAnimator?.cancel()
        conversationHomeRowAnimator = null
    }

    private fun startConversationRowShimmer(row: View, homeRow: Boolean) {
        if (homeRow && conversationHomeRowAnimator?.isRunning == true) {
            return
        }
        if (homeRow) {
            conversationHomeRowAnimator?.cancel()
        }

        val baseColor = Color.parseColor("#242424")
        val highlightColor = Color.parseColor("#303030")
        row.setBackgroundColor(baseColor)

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val fraction = valueAnimator.animatedFraction
                val pulse = sin(Math.PI * fraction).toFloat()
                row.setBackgroundColor(blendColor(baseColor, highlightColor, pulse))
            }
        }

        if (homeRow) {
            conversationHomeRowAnimator = animator
        } else {
            row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) = Unit
                override fun onViewDetachedFromWindow(v: View) {
                    animator.cancel()
                }
            })
        }
        animator.start()
    }

    private fun stopConversationRowShimmer(row: View, homeRow: Boolean) {
        if (homeRow) {
            cancelHomeRowShimmer()
        }
        row.setBackgroundColor(Color.parseColor("#242424"))
    }

    private fun createAvatarView(title: String, sizeDp: Int, textSizeSp: Float): View {
        val size = dp(sizeDp)
        if (title.startsWith(activity.getString(R.string.app_name))) {
            return ImageView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(size, size)
                contentDescription = activity.getString(R.string.app_name)
                scaleType = ImageView.ScaleType.FIT_CENTER
                setImageResource(R.drawable.ic_app_brand)
            }
        }

        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(size, size)
            setBackgroundResource(R.drawable.bg_mock_avatar)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = avatarText(title)
            setTextColor(Color.parseColor("#333333"))
            textSize = textSizeSp
            setTypeface(typeface, Typeface.BOLD)
        }
    }
}
