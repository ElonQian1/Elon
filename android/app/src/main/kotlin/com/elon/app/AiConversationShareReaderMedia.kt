package com.elon.app

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.view.Gravity
import android.view.View
import android.view.Window
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

internal class AiConversationShareReaderMedia(private val activity: AppCompatActivity) {
    private var dialog: Dialog? = null

    fun close() { dialog?.dismiss(); dialog = null }

    fun open(part: WebChatProductionContentPart) {
        if (activity.isFinishing || activity.isDestroyed) return
        close()
        val popup = Dialog(activity).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
            setOwnerActivity(activity)
            setCanceledOnTouchOutside(false)
        }
        dialog = popup
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(color(R.color.elon_bg_app))
        }
        root.addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(ImageButton(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
                setImageResource(R.drawable.ic_project_space_chevron_right)
                rotation = 180f
                background = ColorDrawable(Color.TRANSPARENT)
                setColorFilter(color(R.color.elon_text_primary))
                setPadding(dp(14), dp(14), dp(14), dp(14))
                contentDescription = activity.getString(R.string.ai_conversation_share_back)
                tooltipText = contentDescription
                setOnClickListener { close() }
            })
            addView(TextView(activity).apply {
                text = part.label
                textSize = 16f
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
                setTextColor(color(R.color.elon_text_primary))
            }, LinearLayout.LayoutParams(0, -2, 1f))
        })
        val source = AiConversationShareReaderPresentation.localImageSource(part.imageSource)
        if (part.type == "image" && source != null) {
            val image = ImageView(activity).apply {
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = part.label
            }
            root.addView(image, LinearLayout.LayoutParams(-1, 0, 1f))
            ChatImagePreviewLoader.load(activity, source) { bitmap ->
                image.post { if (dialog === popup && popup.isShowing) image.setImageBitmap(bitmap) }
            }
        } else {
            val content = if (part.richCard != null) {
                WebChatProductionRichCardViews.detail(activity, part.richCard)
            } else TextView(activity).apply {
                textSize = 15f
                setTextColor(color(R.color.elon_text_primary))
                setPadding(dp(20), dp(16), dp(20), dp(24))
                val body = part.textBlock?.content ?: if (part.type == "image")
                    activity.getString(R.string.ai_conversation_share_image_unavailable) else part.label
                val message = ChatMessage("friend", body, webChatMessage = WebChatProductionMessage(
                    "snapshot", "detail", emptySet(), renderMarkdown = part.textBlock?.kind != "code"))
                if (!WebChatProductionRichContentBinder.bindMessageText(this, message)) text = body
                AiConversationShareReaderLinks.bind(this)
            }
            root.addView(ScrollView(activity).apply { addView(content) }, LinearLayout.LayoutParams(-1, 0, 1f))
        }
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val safe = insets.getInsets(WindowInsetsCompat.Type.systemBars())
            view.setPadding(safe.left, safe.top, safe.right, safe.bottom)
            insets
        }
        popup.setContentView(root)
        popup.setOnDismissListener {
            root.removeAllViews()
            if (dialog === popup) dialog = null
        }
        popup.show()
        popup.window?.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            setLayout(-1, -1)
            WindowCompat.setDecorFitsSystemWindows(this, false)
        }
        ViewCompat.requestApplyInsets(root)
    }

    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
    private fun color(token: Int) = ContextCompat.getColor(activity, token)
}
