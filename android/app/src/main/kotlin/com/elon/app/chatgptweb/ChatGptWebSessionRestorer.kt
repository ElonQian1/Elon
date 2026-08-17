package com.elon.app.chatgptweb

import android.content.Context

internal class ChatGptWebSessionRestorer(context: Context) {
    private val stateStore = ChatGptWebSessionStateStore(context)

    fun restoreUrl(): String = stateStore.restoreUrl()

    fun onPageReady(url: String) {
        stateStore.saveUrl(url)
    }

    fun clear() {
        stateStore.clear()
    }
}
