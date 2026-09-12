package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatFileDownloadDialog
import com.elon.app.WebChatImageOriginal

internal class ChatGptWebImageSession(
    activity: AppCompatActivity,
    host: FrameLayout,
    private val pageAdapter: () -> ChatGptWebPageAdapter?,
    onChanged: () -> Unit,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val beginDownload: () -> String,
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
            downloadOriginal = { handle -> download { adapter, id -> adapter.downloadGalleryImage(handle, id) } },
        )
    }
    private val gallery by galleryDelegate

    fun downloadOriginal(original: WebChatImageOriginal): Boolean = download { adapter, id ->
        adapter.downloadConversationFile(original.path, original.asFile(), id)
    }

    private fun download(dispatch: (ChatGptWebPageAdapter, String) -> Unit): Boolean {
        val owner = consumerPort() ?: return false
        val adapter = pageAdapter() ?: return false
        if (adapter.nativeDownloads.snapshot()?.active == true) return false
        val id = beginDownload()
        dispatch(adapter, id)
        downloads.show(owner, id)
        return true
    }

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
