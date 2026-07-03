package com.elon.app

import android.app.Dialog
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.ViewGroup
import android.view.Window
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.TextView

internal object ChatImageViewer {
    fun show(context: Context, attachment: ChatAttachment) {
        val source = chatAttachmentImageSource(attachment) ?: return
        val dialog = Dialog(context).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
            window?.setBackgroundDrawable(ColorDrawable(Color.BLACK))
        }

        val image = ImageView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            contentDescription = attachment.displayName ?: "图片"
            scaleType = ImageView.ScaleType.FIT_CENTER
            setBackgroundColor(Color.BLACK)
            setImageResource(android.R.drawable.ic_menu_gallery)
        }

        val notePanel = createAnnotationNotePanel(context)
        val annotationOverlay = ChatImageAnnotationOverlayView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setImageInfo(attachment.imageWidth, attachment.imageHeight, attachment.annotations)
            onAnnotationClick = { annotation ->
                showAnnotationNote(notePanel, annotation.note)
            }
        }

        val root = FrameLayout(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.BLACK)
            isClickable = true
            setOnClickListener {
                if (notePanel.visibility == android.view.View.VISIBLE) {
                    hideAnnotationNote(notePanel)
                } else {
                    dialog.dismiss()
                }
            }
            addView(image)
            addView(annotationOverlay)
            addView(createCloseButton(context, dialog))
            attachment.displayName?.takeIf { it.isNotBlank() }?.let { name ->
                addView(createTitle(context, name))
            }
            addView(notePanel)
        }

        dialog.setContentView(root)
        dialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )
        dialog.show()
        dialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )

        image.tag = source
        ChatImagePreviewLoader.load(context, source) { bitmap ->
            image.post {
                if (image.tag == source) {
                    image.setImageBitmap(bitmap)
                    val width = attachment.imageWidth ?: bitmap.width
                    val height = attachment.imageHeight ?: bitmap.height
                    annotationOverlay.setImageInfo(width, height, attachment.annotations)
                }
            }
        }
    }

    private fun createCloseButton(context: Context, dialog: Dialog): TextView {
        return TextView(context).apply {
            layoutParams = FrameLayout.LayoutParams(context.dp(44), context.dp(44), Gravity.TOP or Gravity.END).apply {
                topMargin = context.dp(22)
                marginEnd = context.dp(18)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "×"
            setTextColor(Color.parseColor("#D6D6D6"))
            textSize = 30f
            setOnClickListener { dialog.dismiss() }
        }
    }

    private fun createTitle(context: Context, name: String): TextView {
        return TextView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM
            ).apply {
                leftMargin = context.dp(20)
                rightMargin = context.dp(20)
                bottomMargin = context.dp(24)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            maxLines = 1
            text = name
            setTextColor(Color.parseColor("#DDEEEEEE"))
            textSize = 13f
        }
    }

    private fun createAnnotationNotePanel(context: Context): TextView {
        return TextView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM
            ).apply {
                leftMargin = context.dp(20)
                rightMargin = context.dp(20)
                bottomMargin = context.dp(72)
            }
            background = GradientDrawable().apply {
                cornerRadius = context.dp(10).toFloat()
                setColor(Color.parseColor("#EE171717"))
                setStroke(context.dp(1), Color.parseColor("#333333"))
            }
            gravity = Gravity.START
            includeFontPadding = true
            isClickable = true
            maxLines = 10
            setPadding(context.dp(16), context.dp(14), context.dp(16), context.dp(14))
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 15f
            visibility = android.view.View.GONE
            alpha = 0f
        }
    }

    private fun showAnnotationNote(panel: TextView, note: String) {
        panel.animate().cancel()
        panel.text = note.trim()
        panel.visibility = android.view.View.VISIBLE
        panel.alpha = 0f
        panel.translationY = panel.context.dp(8).toFloat()
        panel.animate()
            .alpha(1f)
            .translationY(0f)
            .setDuration(140L)
            .start()
    }

    private fun hideAnnotationNote(panel: TextView) {
        panel.animate().cancel()
        panel.animate()
            .alpha(0f)
            .translationY(panel.context.dp(8).toFloat())
            .setDuration(120L)
            .withEndAction {
                panel.visibility = android.view.View.GONE
                panel.translationY = 0f
            }
            .start()
    }

    private fun Context.dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }
}
