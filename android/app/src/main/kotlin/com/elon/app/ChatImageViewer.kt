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
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast

internal object ChatImageViewer {
    fun show(context: Context, attachment: ChatAttachment, onDownloadOriginal: (() -> Boolean)? = null) {
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
            onDownloadOriginal?.let { download ->
                addView(ImageButton(context).apply {
                    layoutParams = FrameLayout.LayoutParams(context.dp(48), context.dp(48), Gravity.BOTTOM or Gravity.END).apply {
                        bottomMargin = context.dp(24)
                        marginEnd = context.dp(18)
                    }
                    setImageResource(android.R.drawable.stat_sys_download)
                    setColorFilter(Color.WHITE)
                    background = ColorDrawable(Color.TRANSPARENT)
                    contentDescription = "下载原图"
                    tooltipText = contentDescription
                    setOnClickListener {
                        if (download()) { isEnabled = false; alpha = 0.5f }
                        else Toast.makeText(context, "当前无法下载，请重新打开图片重试", Toast.LENGTH_SHORT).show()
                    }
                })
            }
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
            setTextColor(Color.parseColor("#F8F7F4"))
            textSize = 30f
            setOnClickListener { dialog.dismiss() }
        }
    }

    private fun Context.dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }
}
