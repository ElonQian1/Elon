package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import kotlin.math.sin

internal class ChatAiSideMenuView(
    context: Context,
    private val conversations: () -> List<AppConversation>,
    private val activeConversationIndex: () -> Int,
    private val openConversation: (Int) -> Unit,
    private val isConversationWorking: (Int) -> Boolean,
    private val openProjectManagement: () -> Unit,
    private val showCreateConversationDialog: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) : FrameLayout(context) {
    private val conversationDirectoryGroup = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
    }
    private val directoryRowAnimators = mutableMapOf<View, ValueAnimator>()

    init {
        clipChildren = false
        clipToPadding = false
        buildTopMenu()
        buildConversationDirectory()
    }

    fun render() {
        updateConversationSummaries()
    }

    fun stopAnimations() {
        directoryRowAnimators.values.forEach { it.cancel() }
        directoryRowAnimators.clear()
    }

    private fun buildTopMenu() {
        val topMenu = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
        }
        addView(
            topMenu,
            LayoutParams(
                LayoutParams.WRAP_CONTENT,
                LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                leftMargin = dp(32)
                topMargin = dp(106)
            }
        )
        topMenu.addView(
            menuRow("项目", R.drawable.ic_side_menu_project) {
                requestClose(true)
                postDelayed({ openProjectManagement() }, DURATION_MS)
            }
        )
        topMenu.addView(
            menuRow("文件库", R.drawable.ic_side_menu_files) {
                Toast.makeText(context, "文件库功能准备中", Toast.LENGTH_SHORT).show()
            }
        )
        topMenu.addView(
            menuRow("设备", R.drawable.ic_side_menu_device) {
                Toast.makeText(context, "设备功能准备中", Toast.LENGTH_SHORT).show()
            }
        )
    }

    private fun buildConversationDirectory() {
        val chatScroll = ScrollView(context).apply {
            overScrollMode = View.OVER_SCROLL_NEVER
            isFillViewport = false
        }
        chatScroll.addView(
            conversationDirectoryGroup,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            )
        )
        addView(
            chatScroll,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                leftMargin = dp(32)
                rightMargin = dp(18)
                topMargin = dp(388)
                bottomMargin = dp(78)
            }
        )
        conversationDirectoryGroup.addView(conversationHeaderRow())
    }

    private fun updateConversationSummaries() {
        stopAnimations()
        while (conversationDirectoryGroup.childCount > 1) {
            conversationDirectoryGroup.removeViewAt(1)
        }
        val items = conversations()
        if (items.isEmpty()) {
            conversationDirectoryGroup.addView(directoryRow("暂无会话", active = false, working = false, onClick = {}))
            return
        }
        items.forEachIndexed { index, conversation ->
            conversationDirectoryGroup.addView(
                directoryRow(
                    title = conversation.title,
                    active = index == activeConversationIndex(),
                    working = isConversationWorking(index),
                    onClick = {
                        requestClose(true)
                        openConversation(index)
                    }
                )
            )
        }
    }

    private fun conversationHeaderRow(): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(44)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL

            addView(
                menuText("当前聊天").apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                    setTextColor(Color.parseColor("#D0D0D0"))
                }
            )
            addView(
                ImageButton(context).apply {
                    setImageResource(R.drawable.ic_add_circle_simple)
                    imageTintList = ColorStateList.valueOf(Color.parseColor("#D0D0D0"))
                    background = null
                    scaleType = ImageView.ScaleType.CENTER
                    contentDescription = "新建会话"
                    isClickable = true
                    foreground = selectableForeground()
                    setPadding(dp(4), dp(4), dp(4), dp(4))
                    setOnClickListener {
                        requestClose(true)
                        postDelayed({ showCreateConversationDialog() }, DURATION_MS)
                    }
                },
                LinearLayout.LayoutParams(dp(38), dp(38)).apply {
                    rightMargin = dp(8)
                }
            )
        }
    }

    private fun directoryRow(
        title: String,
        active: Boolean,
        working: Boolean,
        onClick: () -> Unit
    ): TextView {
        return menuText(title).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            ).apply {
                topMargin = dp(4)
            }
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            setTextColor(Color.parseColor(if (active) "#E0E0E0" else "#C9C9C9"))
            if (working) {
                startDirectoryRowShimmer(this)
            } else if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#242424"))
                }
            }
            setOnClickListener { onClick() }
        }
    }

    private fun menuRow(title: String, iconRes: Int, action: () -> Unit): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(dp(228), dp(46))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            setPadding(0, 0, dp(8), 0)

            addView(
                ImageView(context).apply {
                    setImageResource(iconRes)
                    imageTintList = ColorStateList.valueOf(Color.parseColor("#C9C9C9"))
                    scaleType = ImageView.ScaleType.CENTER
                },
                LinearLayout.LayoutParams(dp(26), dp(26))
            )
            addView(
                menuText(title).apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                    setPadding(dp(16), 0, 0, 0)
                }
            )
            setOnClickListener { action() }
        }
    }

    private fun menuText(title: String): TextView {
        return TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(dp(190), dp(42))
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#C9C9C9"))
            textSize = 17.5f
        }
    }

    private fun startDirectoryRowShimmer(row: View) {
        val baseColor = Color.parseColor("#242424")
        val highlightColor = Color.parseColor("#363636")
        val background = GradientDrawable().apply {
            cornerRadius = dp(8).toFloat()
            setColor(baseColor)
        }
        row.background = background
        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 1350L
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { valueAnimator ->
                val pulse = sin(Math.PI * valueAnimator.animatedFraction).toFloat()
                background.setColor(blendColor(baseColor, highlightColor, pulse))
            }
        }
        directoryRowAnimators[row] = animator
        row.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) = Unit
            override fun onViewDetachedFromWindow(v: View) {
                directoryRowAnimators.remove(v)?.cancel()
            }
        })
        animator.start()
    }

    private companion object {
        const val DURATION_MS = 260L
    }
}
