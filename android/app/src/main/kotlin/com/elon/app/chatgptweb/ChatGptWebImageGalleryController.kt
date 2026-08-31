package com.elon.app.chatgptweb

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
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
) {
    private var dialog: Dialog? = null
    private var statusView: TextView? = null
    private var grid: GridLayout? = null
    private var sync: ChatGptWebImageGallerySync? = null
    private var syncState = ChatGptWebImageGallerySnapshot.STATE_LOADING
    private val renderStoreChanges = Runnable {
        recordSuccessfulSyncIfUsable()
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
            addView(footer(nextDialog, onCreateImage))
        }
        nextDialog.setContentView(root)
        nextDialog.setOnDismissListener {
            store.removeListener(storeListener)
            host.removeCallbacks(renderStoreChanges)
            sync?.cancel()
            sync = null
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
        if (store.handles().isNotEmpty() && store.hasFreshGallerySync()) {
            syncState = ChatGptWebImageGallerySnapshot.STATE_READY
            renderStatus()
        } else {
            startSync()
        }
        return true
    }

    fun destroy() {
        sync?.cancel()
        sync = null
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

    private fun startSync() {
        sync?.cancel()
        syncState = ChatGptWebImageGallerySnapshot.STATE_LOADING
        val current = store.entries().size
        statusView?.text = if (current > 0) {
            "$current 张图片 · 正在后台同步"
        } else {
            "正在同步图像…"
        }
        sync = ChatGptWebImageGallerySync(activity, host, store, ::renderSyncState).also {
            it.start()
        }
    }

    private fun renderSyncState(state: ChatGptWebImageGallerySnapshot) {
        syncState = state.state
        recordSuccessfulSyncIfUsable()
        renderStatus()
        renderEntries()
    }

    private fun recordSuccessfulSyncIfUsable() {
        if (syncState == ChatGptWebImageGallerySnapshot.STATE_READY && store.handles().isNotEmpty()) {
            store.markGallerySynced()
        }
    }

    private fun renderStatus() {
        val cached = store.entries().size
        statusView?.text = when (syncState) {
            ChatGptWebImageGallerySnapshot.STATE_LOADING -> if (cached > 0) {
                "$cached 张图片 · 正在后台同步"
            } else {
                "正在同步图像…"
            }
            ChatGptWebImageGallerySnapshot.STATE_READY -> if (cached > 0) {
                "已同步 $cached 张图片"
            } else {
                "还没有同步到图片，可创建图片或稍后重试"
            }
            else -> if (cached > 0) {
                "同步失败，已显示 $cached 张本地图片"
            } else {
                "图像同步失败，请重试或打开官网图像"
            }
        }
    }

    private fun renderEntries() {
        val target = grid ?: return
        val entries = store.entries()
        target.removeAllViews()
        entries.forEachIndexed { index, entry ->
            val image = ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setBackgroundColor(ContextCompat.getColor(context, R.color.elon_surface_card))
                setImageResource(R.drawable.ic_attach_photos)
                contentDescription = "图像 ${index + 1}"
                tag = entry.localPath
                setOnClickListener {
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
            ChatImagePreviewLoader.loadSampled(
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
