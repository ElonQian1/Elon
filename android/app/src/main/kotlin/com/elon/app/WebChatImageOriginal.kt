package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebFileDownloadPolicy
import org.json.JSONObject

data class WebChatImageOriginal internal constructor(val path: String, val name: String, val handle: String) {
    internal fun asFile() = WebChatConversationFile(
        id = handle, messageId = "", name = name, kind = "image", role = "", mediaType = "",
        downloadHandle = handle,
    )

    companion object {
        private val PATH = Regex("(?:/g/g-p-[a-fA-F0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?/c/[A-Za-z0-9_-]{1,160}")

        internal fun parse(value: JSONObject?): WebChatImageOriginal? {
            value ?: return null
            val path = value.opt("path") as? String ?: return null
            val name = value.opt("name") as? String ?: return null
            val handle = value.opt("handle") as? String ?: return null
            if (!PATH.matches(path) || !ChatGptWebFileDownloadPolicy.HANDLE.matches(handle) ||
                name.length !in 1..180 || name.isBlank() || name.any { it < ' ' || it == '\u007f' }) return null
            return WebChatImageOriginal(path, name, handle)
        }
    }
}
