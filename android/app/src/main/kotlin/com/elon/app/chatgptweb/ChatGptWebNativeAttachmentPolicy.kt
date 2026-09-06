package com.elon.app.chatgptweb

internal object ChatGptWebNativeAttachmentPolicy {
    private val imageTypes = setOf("image/jpeg", "image/png", "image/webp")

    fun supports(mimeType: String, size: Long, width: Int?, height: Int?): Boolean {
        if (size !in 1L..ChatGptWebNativeAttachmentReader.MAX_BYTES.toLong()) return false
        if (mimeType == "text/plain") return true
        // The production photo picker already decodes and normalizes to this pixel budget.
        return mimeType in imageTypes && width != null && height != null &&
            width in 1..16_384 && height in 1..16_384 && width.toLong() * height <= 4_000_000L
    }
}
