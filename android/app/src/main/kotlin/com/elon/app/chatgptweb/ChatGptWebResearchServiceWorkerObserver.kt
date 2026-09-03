package com.elon.app.chatgptweb

import android.webkit.ServiceWorkerClient
import android.webkit.ServiceWorkerController
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import com.elon.app.BuildConfig

/** Research-build-only request-shape observer for traffic owned by a service worker. */
internal object ChatGptWebResearchServiceWorkerObserver {
    @Volatile
    private var installed = false

    @Synchronized
    fun install() {
        if (!BuildConfig.CHATGPT_PRIVATE_RESEARCH_ENABLED || installed) return
        ServiceWorkerController.getInstance().setServiceWorkerClient(
            object : ServiceWorkerClient() {
                override fun shouldInterceptRequest(request: WebResourceRequest): WebResourceResponse? {
                    ChatGptWebPrivateResearchEventRecorder.recordResourceRequest(
                        method = request.method,
                        url = request.url.toString(),
                        contentType = request.requestHeaders.entries.firstOrNull {
                            it.key.equals("content-type", ignoreCase = true)
                        }?.value,
                    )
                    return null
                }
            },
        )
        installed = true
    }
}
