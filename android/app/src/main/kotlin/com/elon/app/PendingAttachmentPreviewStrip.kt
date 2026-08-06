package com.elon.app

import android.content.Context
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import java.io.File

internal class PendingAttachmentPreviewStrip(
    private val context: Context,
    private val pendingAttachments: MutableList<PendingAttachment>,
    private val onChanged: () -> Unit,
    private val onEditImage: (Int) -> Unit
) {
    private val list = LinearLayout(context).apply {
        layoutParams = ViewGroup.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(context.dp(14), context.dp(6), context.dp(14), context.dp(6))
    }

    val view: HorizontalScrollView = HorizontalScrollView(context).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            context.dp(74)
        )
        visibility = View.GONE
        isFillViewport = false
        overScrollMode = View.OVER_SCROLL_NEVER
        isHorizontalScrollBarEnabled = false
        setBackgroundColor(Color.parseColor("#0E1116"))
        addView(list)
    }

    fun refresh() {
        list.removeAllViews()
        if (pendingAttachments.isEmpty()) {
            view.visibility = View.GONE
            return
        }
        view.visibility = View.VISIBLE
        pendingAttachments.forEachIndexed { index, attachment ->
            list.addView(createPreview(attachment, index))
        }
    }

    private fun createPreview(attachment: PendingAttachment, index: Int): View {
        val wrapper = FrameLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(context.dp(62), context.dp(62)).apply {
                marginEnd = context.dp(8)
            }
        }
        val isImage = attachment.isImage()
        wrapper.addView(if (isImage) createImagePreview(attachment) else createFilePreview(attachment))
        if (isImage) {
            wrapper.addView(createEditButton(index))
        }
        wrapper.addView(createRemoveButton(index))
        return wrapper
    }

    private fun createImagePreview(attachment: PendingAttachment): ImageView {
        return ImageView(context).apply {
            layoutParams = FrameLayout.LayoutParams(context.dp(58), context.dp(58), Gravity.BOTTOM or Gravity.START)
            background = roundedBackground("#20262E")
            contentDescription = attachment.displayName
            scaleType = ImageView.ScaleType.CENTER_CROP
            setOnClickListener { ChatImageViewer.show(context, attachment.toChatAttachment()) }
            loadPendingAttachmentThumbnail(attachment.file)?.let {
                setImageBitmap(it)
            } ?: setImageResource(android.R.drawable.ic_menu_gallery)
        }
    }

    private fun createFilePreview(attachment: PendingAttachment): TextView {
        return TextView(context).apply {
            layoutParams = FrameLayout.LayoutParams(context.dp(58), context.dp(58), Gravity.BOTTOM or Gravity.START)
            background = roundedBackground("#20262E").apply {
                setStroke(context.dp(1), Color.parseColor("#3AFFFFFF"))
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            setPadding(context.dp(6), 0, context.dp(6), 0)
            setTextColor(Color.parseColor("#F8F7F4"))
            text = attachment.displayName
            textSize = 11f
        }
    }

    private fun createEditButton(index: Int): ImageButton {
        return ImageButton(context).apply {
            layoutParams = FrameLayout.LayoutParams(context.dp(28), context.dp(28), Gravity.TOP or Gravity.START)
            background = ColorDrawable(Color.TRANSPARENT)
            contentDescription = "编辑图片"
            scaleType = ImageView.ScaleType.CENTER
            setPadding(context.dp(2), context.dp(2), context.dp(2), context.dp(2))
            setImageResource(R.drawable.ic_chat_image_edit_marker)
            setOnClickListener {
                onEditImage(index)
            }
        }
    }

    private fun createRemoveButton(index: Int): TextView {
        return TextView(context).apply {
            layoutParams = FrameLayout.LayoutParams(context.dp(20), context.dp(20), Gravity.TOP or Gravity.END)
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#CC202020"))
                setStroke(context.dp(1), Color.parseColor("#66FFFFFF"))
            }
            contentDescription = "移除附件"
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "×"
            setTextColor(Color.parseColor("#F8F7F4"))
            textSize = 14f
            setOnClickListener {
                pendingAttachments.getOrNull(index)?.let { removed ->
                    runCatching { removed.file.delete() }
                    pendingAttachments.removeAt(index)
                    refresh()
                    onChanged()
                }
            }
        }
    }

    private fun roundedBackground(color: String): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = context.dp(8).toFloat()
            setColor(Color.parseColor(color))
        }
    }

    private fun PendingAttachment.isImage(): Boolean {
        return kind == "image" || mimeType.startsWith("image/")
    }

    private fun PendingAttachment.toChatAttachment(): ChatAttachment {
        return ChatAttachment(
            kind = kind,
            displayName = displayName,
            fileName = fileName,
            mimeType = mimeType,
            localPath = file.absolutePath,
            sizeBytes = file.length(),
            imageWidth = imageWidth,
            imageHeight = imageHeight,
            annotations = annotations
        )
    }

    private fun loadPendingAttachmentThumbnail(file: File) = runCatching {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.absolutePath, bounds)
        val options = BitmapFactory.Options().apply {
            inSampleSize = attachmentPreviewSampleSize(bounds.outWidth, bounds.outHeight, context.dp(160))
        }
        BitmapFactory.decodeFile(file.absolutePath, options)
    }.getOrNull()

    private fun attachmentPreviewSampleSize(width: Int, height: Int, target: Int): Int {
        if (width <= target && height <= target) return 1
        var sample = 1
        while ((width / sample) > target || (height / sample) > target) {
            sample *= 2
        }
        return sample
    }

    private fun Context.dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }
}
