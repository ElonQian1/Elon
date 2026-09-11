package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatFileDownloadDialog

internal class ChatGptWebImageSession(
    activity: AppCompatActivity,
    host: FrameLayout,
    pageAdapter: () -> ChatGptWebPageAdapter?,
    onChanged: () -> Unit,
    consumerPort: () -> WebChatConsumerPort?,
    beginDownload: () -> String,
) {
    private val handler = Handler(Looper.getMainLooper())
    private val store = ChatGptWebImageAssetStore(activity.applicationContext)
    private val downloads = WebChatFileDownloadDialog(activity, host, consumerPort)
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
            requestPreview = { handle -> pageAdapter()?.let { it.requestImageAsset(handle); true } ?: false },
            downloadOriginal = download@{ handle ->
                val owner = consumerPort() ?: return@download false
                val adapter = pageAdapter() ?: return@download false
                if (adapter.nativeDownloads.snapshot()?.active == true) return@download false
                val id = beginDownload()
                adapter.downloadGalleryImage(handle, id)
                downloads.show(owner, id)
                true
            },
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
        downloads.dismiss()
        if (galleryDelegate.isInitialized()) gallery.destroy()
    }

    fun resetAssets() {
        assets.reset()
        handler.removeCallbacksAndMessages(null)
    }
}
