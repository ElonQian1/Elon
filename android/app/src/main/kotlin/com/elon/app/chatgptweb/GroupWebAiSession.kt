package com.elon.app.chatgptweb

import android.net.Uri
import android.view.ViewGroup
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.beginWebChatBackgroundInteraction

/** One isolated temporary document used for group configuration or a group request. */
internal class GroupWebAiSession(
    private val activity: AppCompatActivity,
    private val onEvent: (ChatGptWebEvent) -> Unit,
    private val onFailure: () -> Unit,
) {
    private var closed = false
    private val view = createChatGptBackgroundWebView(activity, null, {}, { it.onReceiveValue(null) })
    val adapter = ChatGptWebPageAdapter(activity, view, { if (!closed) onEvent(it) }, {})

    fun start() {
        activity.findViewById<ViewGroup>(android.R.id.content).addView(view, 0,
            ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        view.beginWebChatBackgroundInteraction()
        view.settings.mediaPlaybackRequiresUserGesture = true
        view.webViewClient = ChatGptWebViewClient(
            onPageStarted = { adapter.onPageStarted(it) }, onPageReady = { adapter.onPageReady(it) },
            onBlockedNavigation = { if (!closed) onFailure() }, onPageError = { if (!closed) onFailure() },
            rewriteAllowedMainFrameUrl = { null },
        )
        adapter.install()
        ChatGptWebProxyController(activity).prepare {
            if (!closed) view.loadUrl("https://chatgpt.com/?temporary-chat=true")
        }
    }

    fun close() {
        if (closed) return
        closed = true
        adapter.dispose()
        view.stopLoading()
        (view.parent as? ViewGroup)?.removeView(view)
        view.destroy()
    }

    companion object {
        fun ready(snapshot: ChatGptWebSnapshot): Boolean {
            val uri = Uri.parse(snapshot.url)
            return !snapshot.loginRequired && (snapshot.composerReady || snapshot.privateSendReady) &&
                !snapshot.streaming && uri.scheme == "https" && uri.host == "chatgpt.com" &&
                uri.path in listOf("", "/") && uri.getQueryParameter("temporary-chat") == "true" &&
                snapshot.messages.isEmpty() && snapshot.draft.isBlank()
        }
    }
}
