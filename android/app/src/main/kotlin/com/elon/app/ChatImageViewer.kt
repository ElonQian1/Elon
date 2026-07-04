package com.elon.app

import android.app.Dialog
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
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

        val annotationOverlay = ChatImageAnnotationOverlayView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setImageInfo(attachment.imageWidth, attachment.imageHeight, attachment.annotations)
        }

        val root = FrameLayout(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.BLACK)
            isClickable = true
            setOnClickListener {
                if (!annotationOverlay.collapseExpandedAnnotation()) {
                    dialog.dismiss()
                }
            }
            addView(image)
            addView(annotationOverlay)
            addView(createCloseButton(context, dialog))
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

    private fun Context.dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }
}
