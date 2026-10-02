package com.elon.app

import android.app.Dialog
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.PointF
import android.graphics.RectF
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.PopupMenu
import android.widget.TextView
import android.widget.Toast
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.davemorrissey.labs.subscaleview.ImageSource
import com.davemorrissey.labs.subscaleview.SubsamplingScaleImageView
import com.davemorrissey.labs.subscaleview.decoder.CompatDecoderFactory
import com.davemorrissey.labs.subscaleview.decoder.SkiaImageDecoder
import com.davemorrissey.labs.subscaleview.decoder.SkiaImageRegionDecoder
import java.util.concurrent.Executors
import java.util.concurrent.Future

internal object ChatImageViewer {
    private val io = Executors.newFixedThreadPool(2)
    private val main = Handler(Looper.getMainLooper())

    fun show(context: Context, attachment: ChatAttachment, onDownloadOriginal: (() -> Boolean)? = null) {
        val source = chatAttachmentImageSource(attachment) ?: return
        val dialog = Dialog(context).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
            window?.setBackgroundDrawable(ColorDrawable(Color.BLACK))
        }
        val image = SubsamplingScaleImageView(context).apply {
            contentDescription = "高清图片：${attachment.displayName.orEmpty()}"
            setOrientation(SubsamplingScaleImageView.ORIENTATION_USE_EXIF)
            setMaxTileSize(1024)
            setMaxScale(4f)
            setDoubleTapZoomScale(2f)
            setBitmapDecoderFactory(CompatDecoderFactory(SkiaImageDecoder::class.java, Bitmap.Config.ARGB_8888))
            setRegionDecoderFactory(CompatDecoderFactory(SkiaImageRegionDecoder::class.java, Bitmap.Config.ARGB_8888))
        }
        val overlay = ChatImageAnnotationOverlayView(context).apply {
            setImageInfo(attachment.imageWidth, attachment.imageHeight, attachment.annotations)
            imageRectProvider = {
                val rotated = image.appliedOrientation == 90 || image.appliedOrientation == 270
                val width = if (rotated) image.sHeight else image.sWidth
                val height = if (rotated) image.sWidth else image.sHeight
                val start = image.sourceToViewCoord(0f, 0f)
                val end = image.sourceToViewCoord(width.toFloat(), height.toFloat())
                if (!image.isReady || start == null || end == null) null
                else RectF(start.x, start.y, end.x, end.y)
            }
        }
        val readingButton = TextView(context).apply {
            setTextColor(Color.WHITE)
            textSize = 14f
            gravity = Gravity.CENTER
            minWidth = dp(context, 88)
            minimumHeight = dp(context, 48)
            setPadding(dp(context, 12), 0, dp(context, 12), 0)
        }
        val reading = ChatImageReadingController(image, readingButton)
        image.setOnStateChangedListener(object : SubsamplingScaleImageView.DefaultOnStateChangedListener() {
            override fun onScaleChanged(newScale: Float, origin: Int) { overlay.invalidate(); reading.stateChanged() }
            override fun onCenterChanged(newCenter: PointF?, origin: Int) { overlay.invalidate(); reading.stateChanged() }
        })
        val status = TextView(context).apply {
            minimumHeight = dp(context, 48)
            setBackgroundColor(Color.parseColor("#E6000000"))
            setTextColor(Color.WHITE)
            textSize = 14f
            gravity = Gravity.CENTER
            setPadding(dp(context, 60), dp(context, 12), dp(context, 60), dp(context, 12))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
            accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        }
        val root = FrameLayout(context).apply {
            setBackgroundColor(Color.BLACK)
            addView(image, FrameLayout.LayoutParams(-1, -1))
            addView(overlay, FrameLayout.LayoutParams(-1, -1))
            addView(status, FrameLayout.LayoutParams(-1, -2, Gravity.TOP))
        }
        status.addOnLayoutChangeListener { _, _, _, _, _, _, _, _, _ ->
            for (view in listOf(image, overlay)) {
                val params = view.layoutParams as FrameLayout.LayoutParams
                if (params.topMargin != status.height || params.bottomMargin != dp(context, 56)) {
                    params.topMargin = status.height
                    params.bottomMargin = dp(context, 56)
                    view.layoutParams = params
                }
            }
        }
        var closed = false
        var generation = 0
        var request: Future<*>? = null
        var lease: ChatImageDiskCache.Lease? = null
        val save = icon(context, android.R.drawable.stat_sys_download,
            if (onDownloadOriginal != null) "下载原图" else "保存原图")
        save.isEnabled = onDownloadOriginal != null
        val retry = icon(context, android.R.drawable.ic_popup_sync, "重试加载图片").apply { visibility = View.GONE }

        fun failed() {
            if (closed) return
            status.text = "图片加载失败"
            retry.visibility = View.VISIBLE
        }
        fun load() {
            val current = ++generation
            request?.cancel(true)
            reading.reset()
            image.recycle()
            lease?.close()
            lease = null
            save.isEnabled = onDownloadOriginal != null
            status.text = "正在加载高清图片…"
            retry.visibility = View.GONE
            request = io.submit {
                val result = runCatching { ChatImageDiskCache.acquire(context.applicationContext, source) }
                main.post {
                    if (closed || current != generation) {
                        result.getOrNull()?.close()
                        return@post
                    }
                    result.onSuccess { loaded ->
                        lease = loaded
                        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                        BitmapFactory.decodeFile(loaded.file.path, bounds)
                        // Android's region decoder cannot read GIF/BMP; retain their static preview.
                        val tiled = bounds.outMimeType !in setOf("image/gif", "image/bmp", "image/x-ms-bmp")
                        image.setImage(ImageSource.uri(Uri.fromFile(loaded.file)).tiling(tiled))
                    }.onFailure { failed() }
                }
            }
        }
        image.setOnImageEventListener(object : SubsamplingScaleImageView.DefaultOnImageEventListener() {
            override fun onReady() { if (!closed) reading.ready() }
            override fun onImageLoaded() {
                if (closed) return
                status.text = "${image.sWidth} × ${image.sHeight}"
                overlay.setImageInfo(image.sWidth, image.sHeight, attachment.annotations)
                save.isEnabled = true
            }
            override fun onImageLoadError(e: Exception) { failed() }
            override fun onTileLoadError(e: Exception) { failed() }
        })
        retry.setOnClickListener {
            image.recycle()
            lease?.close()
            lease = null
            ChatImageDiskCache.remove(context, source)
            load()
        }
        save.setOnClickListener {
            if (onDownloadOriginal != null) {
                if (onDownloadOriginal()) { save.isEnabled = false; save.alpha = 0.5f }
                else Toast.makeText(context, "当前无法下载，请稍后重试", Toast.LENGTH_SHORT).show()
            } else {
                save.isEnabled = false
                io.submit {
                    val result = runCatching {
                        ChatImageOriginalExport.save(context.applicationContext, source, attachment)
                    }
                    main.post {
                        if (closed) return@post
                        save.isEnabled = true
                        result.onSuccess { exportIntent ->
                            if (exportIntent == null) Toast.makeText(context, "原图已保存到相册", Toast.LENGTH_SHORT).show()
                            else runCatching { context.startActivity(exportIntent) }
                                .onFailure { Toast.makeText(context, "无法导出原图", Toast.LENGTH_SHORT).show() }
                        }.onFailure { Toast.makeText(context, "保存失败，请重试", Toast.LENGTH_SHORT).show() }
                    }
                }
            }
        }
        val close = icon(context, android.R.drawable.ic_menu_close_clear_cancel, "关闭图片")
        close.setOnClickListener { dialog.dismiss() }
        root.addView(close, FrameLayout.LayoutParams(dp(context, 48), dp(context, 48), Gravity.TOP or Gravity.START))
        val more = icon(context, R.drawable.ic_more_vertical, "图片选项")
        more.setOnClickListener {
            PopupMenu(context, more).apply {
                menu.add("清理图片缓存").setOnMenuItemClickListener {
                    io.submit {
                        val removed = ChatImageDiskCache.clearUnused(context.applicationContext)
                        main.post {
                            if (!closed) Toast.makeText(context,
                                "已清理 %.1f MB，正在查看的图片保留".format(removed / 1048576.0), Toast.LENGTH_SHORT).show()
                        }
                    }
                    true
                }
            }.show()
        }
        root.addView(more, FrameLayout.LayoutParams(dp(context, 48), dp(context, 48), Gravity.TOP or Gravity.END))
        val actions = LinearLayout(context).apply {
            setBackgroundColor(Color.parseColor("#E6000000"))
            gravity = Gravity.CENTER
            addView(retry)
            addView(readingButton)
            addView(icon(context, android.R.drawable.ic_menu_zoom, "原始比例").apply {
                setOnClickListener {
                    if (image.isReady) image.setScaleAndCenter(1f, image.center)
                }
            })
            addView(save)
        }
        root.addView(actions, FrameLayout.LayoutParams(-1, dp(context, 56), Gravity.BOTTOM))
        dialog.setContentView(root)
        val lifecycle = (context as? LifecycleOwner)?.lifecycle
        val lifecycleObserver = object : DefaultLifecycleObserver {
            override fun onDestroy(owner: LifecycleOwner) { dialog.dismiss() }
        }
        lifecycle?.addObserver(lifecycleObserver)
        dialog.setOnDismissListener {
            lifecycle?.removeObserver(lifecycleObserver)
            closed = true
            generation++
            request?.cancel(true)
            image.setOnImageEventListener(null)
            reading.release()
            image.recycle()
            lease?.close()
            lease = null
        }
        dialog.show()
        dialog.window?.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        load()
    }

    private fun icon(context: Context, resource: Int, label: String) = ImageButton(context).apply {
        layoutParams = ViewGroup.LayoutParams(dp(context, 48), dp(context, 48))
        setImageResource(resource)
        setColorFilter(Color.WHITE)
        background = ColorDrawable(Color.TRANSPARENT)
        contentDescription = label
        tooltipText = label
    }
    private fun dp(context: Context, value: Int) = (value * context.resources.displayMetrics.density).toInt()
}
