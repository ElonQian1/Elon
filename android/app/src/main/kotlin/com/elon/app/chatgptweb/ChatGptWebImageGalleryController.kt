package com.elon.app.chatgptweb

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.SystemClock
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.widget.Button
import android.widget.FrameLayout
import android.widget.GridLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.ChatAttachment
import com.elon.app.ChatImagePreviewLoader
import com.elon.app.ChatImageViewer
import com.elon.app.R

internal class ChatGptWebImageGalleryController(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val store: ChatGptWebImageAssetStore,
    private val requestPage: (String, String, Set<String>) -> Boolean,
    private val cancelPage: (String) -> Unit,
) {
    private var dialog: Dialog? = null
    private var statusView: TextView? = null
    private var grid: GridLayout? = null
    private var activeRequestId: String? = null
    private var dispatchAttempt: Runnable? = null
    private var pageSnapshot: ChatGptWebImageGallerySnapshot? = null
    private var previousPage: ImageButton? = null
    private var nextPage: ImageButton? = null
    private var pageLabel: TextView? = null
    private var syncState = ChatGptWebImageGallerySnapshot.STATE_LOADING
    private val syncTimeout = Runnable {
        activeRequestId?.let(cancelPage)
        activeRequestId = null
        dispatchAttempt?.let(host::removeCallbacks)
        syncState = ChatGptWebImageGallerySnapshot.STATE_FAILED
        renderStatus()
    }
    private val renderStoreChanges = Runnable {
        renderEntries()
        renderStatus()
    }
    private val storeListener: () -> Unit = {
        activity.runOnUiThread {
            host.removeCallbacks(renderStoreChanges)
            host.postDelayed(renderStoreChanges, STORE_RENDER_DEBOUNCE_MS)
        }
    }

    fun show(onCreateImage: () -> Unit): Boolean {
        if (activity.isFinishing || activity.isDestroyed) return false
        dialog?.let {
            if (!it.isShowing) it.show()
            return true
        }
        val nextDialog = Dialog(activity).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
            window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        }
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(ContextCompat.getColor(context, R.color.elon_bg_app))
            setPadding(dp(14), dp(20), dp(14), dp(14))
            addView(header(nextDialog))
            statusView = TextView(context).also { status ->
                status.setTextColor(ContextCompat.getColor(context, R.color.elon_text_secondary))
                status.textSize = 13f
                status.setPadding(dp(4), dp(8), dp(4), dp(10))
                addView(status, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ))
            }
            grid = GridLayout(context).also { gallery ->
                gallery.columnCount = 3
                gallery.alignmentMode = GridLayout.ALIGN_BOUNDS
                val scroll = ScrollView(context).apply { addView(gallery) }
                addView(scroll, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f,
                ))
            }
            addView(pagination())
            addView(footer(nextDialog, onCreateImage))
        }
        nextDialog.setContentView(root)
        nextDialog.setOnDismissListener {
            store.removeListener(storeListener)
            host.removeCallbacks(renderStoreChanges)
            cancelSync()
            pageSnapshot = null
            previousPage = null
            nextPage = null
            pageLabel = null
            dialog = null
            statusView = null
            grid = null
        }
        dialog = nextDialog
        store.addListener(storeListener)
        nextDialog.show()
        nextDialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT,
        )
        renderEntries()
        startSync("open")
        return true
    }

    fun destroy() {
        cancelSync()
        dialog?.dismiss()
        dialog = null
    }

    private fun header(dialog: Dialog): View = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        addView(TextView(context).apply {
            text = "‹"
            textSize = 40f
            gravity = Gravity.CENTER
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
            contentDescription = "返回聊天"
            setOnClickListener { dialog.dismiss() }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(TextView(context).apply {
            text = "图像"
            textSize = 20f
            gravity = Gravity.CENTER
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
        }, LinearLayout.LayoutParams(0, dp(48), 1f))
        addView(ImageButton(context).apply {
            setImageResource(R.drawable.ic_side_menu_refresh)
            setColorFilter(ContextCompat.getColor(context, R.color.elon_icon_primary))
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = "同步图像"
            setOnClickListener { startSync() }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun footer(dialog: Dialog, onCreateImage: () -> Unit): View = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(12), 0, 0)
        addView(Button(context).apply {
            text = "创建图片"
            isAllCaps = false
            setOnClickListener {
                dialog.dismiss()
                onCreateImage()
            }
        }, LinearLayout.LayoutParams(0, dp(52), 1f).apply { marginEnd = dp(8) })
        addView(Button(context).apply {
            text = "官网图像"
            isAllCaps = false
            setOnClickListener {
                activity.startActivity(
                    ChatGptWebOfficialFallbackIntent.create(activity, IMAGES_URL),
                )
            }
        }, LinearLayout.LayoutParams(0, dp(52), 1f).apply { marginStart = dp(8) })
    }

    private fun pagination(): View = LinearLayout(activity).apply {
        gravity = Gravity.CENTER
        previousPage = ImageButton(context).apply {
            setImageResource(R.drawable.ic_toolbar_back_custom)
            contentDescription = "上一页"
            setBackgroundColor(Color.TRANSPARENT)
            setColorFilter(ContextCompat.getColor(context, R.color.elon_icon_primary))
            setOnClickListener { startSync("previous") }
        }.also { addView(it, LinearLayout.LayoutParams(dp(48), dp(48))) }
        pageLabel = TextView(context).apply {
            gravity = Gravity.CENTER
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_secondary))
            textSize = 14f
        }.also { addView(it, LinearLayout.LayoutParams(dp(100), dp(48))) }
        nextPage = ImageButton(context).apply {
            setImageResource(R.drawable.ic_toolbar_back_custom)
            rotation = 180f
            contentDescription = "下一页"
            setBackgroundColor(Color.TRANSPARENT)
            setColorFilter(ContextCompat.getColor(context, R.color.elon_icon_primary))
            setOnClickListener { startSync("next") }
        }.also { addView(it, LinearLayout.LayoutParams(dp(48), dp(48))) }
    }

    private fun cancelSync() {
        activeRequestId?.let(cancelPage)
        activeRequestId = null
        host.removeCallbacks(syncTimeout)
        dispatchAttempt?.let(host::removeCallbacks)
        dispatchAttempt = null
    }

    private fun startSync(operation: String = "refresh") {
        cancelSync()
        syncState = ChatGptWebImageGallerySnapshot.STATE_LOADING
        renderStatus()
        val id = "mcp_gallery" + SystemClock.elapsedRealtimeNanos().toString(36)
        activeRequestId = id
        host.postDelayed(syncTimeout, 40_000L)
        val cachedHandles = store.handles()
        dispatchAttempt = object : Runnable {
            override fun run() {
                if (activeRequestId != id || dialog == null) return
                if (!requestPage(id, operation, cachedHandles)) host.postDelayed(this, 500L)
            }
        }
        dispatchAttempt?.run()
    }

    fun accept(state: ChatGptWebImageGallerySnapshot) {
        if (dialog == null || state.requestId == null || state.requestId != activeRequestId) return
        syncState = state.state
        if (state.handles != null) pageSnapshot = state
        if (state.state != ChatGptWebImageGallerySnapshot.STATE_LOADING) host.removeCallbacks(syncTimeout)
        renderStatus()
        renderEntries()
    }

    fun accept(asset: ChatGptWebImageAsset) {
        if (dialog != null && asset.galleryRequestId == activeRequestId && asset.ready &&
            asset.handle in pageSnapshot?.handles.orEmpty()) store.save(asset) {}
    }

    private fun renderStatus() {
        val page = pageSnapshot
        val visibleHandles = page?.handles.orEmpty().toSet()
        val cached = store.handles().count(visibleHandles::contains)
        val loading = syncState == ChatGptWebImageGallerySnapshot.STATE_LOADING
        previousPage?.isEnabled = !loading && page?.hasPrevious == true
        nextPage?.isEnabled = !loading && page?.hasNext == true
        previousPage?.alpha = if (previousPage?.isEnabled == true) 1f else 0.35f
        nextPage?.alpha = if (nextPage?.isEnabled == true) 1f else 0.35f
        pageLabel?.text = page?.let { "第 ${it.pageIndex + 1} 页" }.orEmpty()
        statusView?.text = when (syncState) {
            ChatGptWebImageGallerySnapshot.STATE_LOADING -> if (cached > 0) {
                "$cached 张图片 · 正在后台同步"
            } else {
                "正在同步图像…"
            }
            ChatGptWebImageGallerySnapshot.STATE_READY -> if ((page?.observedCount ?: 0) > 0) {
                "本页 ${page?.observedCount} 张图片"
            } else {
                "还没有创建的图片"
            }
            ChatGptWebImageGallerySnapshot.STATE_PARTIAL -> "已加载 $cached 张图片，部分图片暂未加载"
            else -> if (cached > 0) {
                "同步失败，已显示 $cached 张本地图片"
            } else {
                "图像同步失败，请重试或打开官网图像"
            }
        }
    }

    private fun renderEntries() {
        val target = grid ?: return
        val entries = store.entries().associateBy { it.handle }
        target.removeAllViews()
        pageSnapshot?.handles.orEmpty().forEachIndexed { index, handle ->
            val entry = entries[handle]
            val image = ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setBackgroundColor(ContextCompat.getColor(context, R.color.elon_surface_card))
                setImageResource(R.drawable.ic_attach_photos)
                contentDescription = "图像 ${index + 1}"
                tag = entry?.localPath
                setOnClickListener {
                    if (entry == null) return@setOnClickListener
                    ChatImageViewer.show(
                        activity,
                        ChatAttachment(
                            kind = "image",
                            displayName = "图像 ${index + 1}",
                            mimeType = "image/jpeg",
                            localPath = entry.localPath,
                            imageWidth = entry.width,
                            imageHeight = entry.height,
                        ),
                    )
                }
            }
            if (entry != null) ChatImagePreviewLoader.loadSampled(
                activity,
                entry.localPath,
                GALLERY_PREVIEW_MAX_PIXELS,
            ) { bitmap ->
                image.post {
                    if (image.tag == entry.localPath) image.setImageBitmap(bitmap)
                }
            }
            target.addView(image, GridLayout.LayoutParams().apply {
                width = 0
                height = dp(132)
                columnSpec = GridLayout.spec(GridLayout.UNDEFINED, 1f)
                setMargins(dp(2), dp(2), dp(2), dp(2))
            })
        }
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val IMAGES_URL = "https://chatgpt.com/images"
        const val GALLERY_PREVIEW_MAX_PIXELS = 160_000
        const val STORE_RENDER_DEBOUNCE_MS = 500L
    }
}
