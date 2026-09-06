package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity

internal class ChatGptWebImageSession(
    activity: AppCompatActivity,
    host: FrameLayout,
    pageAdapter: () -> ChatGptWebPageAdapter?,
    onChanged: () -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val store = ChatGptWebImageAssetStore(activity.applicationContext)
    val assets = ChatGptWebImageAssetCoordinator(
        store = store,
        request = { handle ->
            pageAdapter()?.let { adapter ->
                adapter.requestImageAsset(handle)
                true
            } ?: false
        },
        schedule = { task, delayMs -> handler.postDelayed(task, delayMs) },
        cancel = handler::removeCallbacks,
        dispatch = { task -> handler.post(task) },
        onChanged = onChanged,
    )
    private val galleryDelegate = lazy(LazyThreadSafetyMode.NONE) {
        ChatGptWebImageGalleryController(activity, host, store,
            requestPage = { id, operation, handles -> pageAdapter()?.syncImageGallery(id, operation, handles) == true },
            cancelPage = { id -> pageAdapter()?.cancelImageGallery(id) },
        )
    }
    private val gallery by galleryDelegate

    fun show(onCreateImage: () -> Unit): Boolean = gallery.show(onCreateImage)

    fun acceptGallery(snapshot: ChatGptWebImageGallerySnapshot) {
        if (galleryDelegate.isInitialized()) gallery.accept(snapshot)
    }

    fun acceptAsset(asset: ChatGptWebImageAsset) {
        if (asset.galleryRequestId == null) assets.accept(asset)
        else if (galleryDelegate.isInitialized()) gallery.accept(asset)
    }

    fun dismissGallery() {
        if (galleryDelegate.isInitialized()) gallery.destroy()
    }

    fun resetAssets() {
        assets.reset()
        handler.removeCallbacksAndMessages(null)
    }
}
