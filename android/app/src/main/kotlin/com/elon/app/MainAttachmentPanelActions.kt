package com.elon.app

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class MainAttachmentPanelActions(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val activeConversation: () -> AppConversation,
    private val attachmentPanel: () -> LinearLayout?,
    private val attachmentButton: () -> ImageButton?,
    private val collapseInputComposer: () -> Unit,
    private val collapseEmojiPanel: () -> Unit,
    private val openCameraAttachment: () -> Unit,
    private val openPhotoAttachment: () -> Unit,
    private val openDocumentAttachment: () -> Unit
) {
    var isOpen = false
        private set

    private var iconAnimationToken = 0

    fun buildAttachmentPanel(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(100)
            )
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(50), dp(4), dp(18), dp(14))
            visibility = View.GONE

            addView(createAttachmentAction("拍照", R.drawable.ic_input_camera_new, addEndMargin = true) {
                openCameraAttachment()
            })
            addView(createAttachmentAction("图片", R.drawable.ic_input_photo_new, addEndMargin = true) {
                openPhotoAttachment()
            })
            addView(createAttachmentAction("文件", R.drawable.ic_input_file_new, addEndMargin = false) {
                openDocumentAttachment()
            })
        }
    }

    fun toggleAttachmentPanel() {
        if (isOpen) collapseAttachmentPanel() else expandAttachmentPanel()
    }

    fun expandAttachmentPanel() {
        if (activeConversation().ended) return
        collapseEmojiPanel()
        collapseInputComposer()
        if (isOpen) return
        val panel = attachmentPanel() ?: return
        isOpen = true
        panel.visibility = View.VISIBLE
        animateAttachmentButtonIcon(expanded = true)
    }

    fun collapseAttachmentPanel() {
        val panel = attachmentPanel() ?: return
        val wasOpen = isOpen || panel.visibility == View.VISIBLE
        isOpen = false
        panel.visibility = View.GONE
        if (wasOpen) {
            animateAttachmentButtonIcon(expanded = false)
        } else {
            updateAttachmentButtonIcon(expanded = false)
        }
    }

    private fun createAttachmentAction(
        label: String,
        iconRes: Int,
        addEndMargin: Boolean = true,
        action: () -> Unit
    ): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(64), dp(72)).apply {
                if (addEndMargin) marginEnd = dp(14)
            }
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.parseColor("#111111"))
                setStroke(dp(1), Color.parseColor("#3A3A3A"))
            }
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(28), dp(28))
                setImageResource(iconRes)
                scaleType = ImageView.ScaleType.FIT_CENTER
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(5)
                }
                includeFontPadding = false
                text = label
                setTextColor(Color.parseColor("#D6D6D6"))
                textSize = 13f
            })
            setOnClickListener {
                collapseAttachmentPanel()
                action()
            }
        }
    }

    private fun animateAttachmentButtonIcon(expanded: Boolean) {
        val button = attachmentButton() ?: return
        val token = ++iconAnimationToken
        val targetAlpha = if (activeConversation().ended) 0.55f else 1f
        button.animate().cancel()
        button.rotation = 0f
        button.scaleX = 1f
        button.scaleY = 1f
        button.animate()
            .alpha(0.55f)
            .setDuration(70L)
            .withEndAction {
                if (token != iconAnimationToken) return@withEndAction
                updateAttachmentButtonIcon(expanded)
                button.animate()
                    .alpha(targetAlpha)
                    .setDuration(90L)
                    .start()
            }
            .start()
    }

    private fun updateAttachmentButtonIcon(expanded: Boolean) {
        val button = attachmentButton() ?: return
        button.setImageResource(R.drawable.ic_input_add_new)
        button.rotation = if (expanded) 45f else 0f
        button.contentDescription = if (expanded) "收起更多输入功能" else "展开更多输入功能"
    }
}
