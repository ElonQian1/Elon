package com.elon.app.chatgptweb

internal object ChatGptWebNativeAttachmentPolicy {
    private val imageTypes = setOf("image/jpeg", "image/png", "image/webp")
    private val documentTypes = setOf(
        "text/plain", "application/pdf", "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.ms-powerpoint",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.oasis.opendocument.text", "application/rtf", "text/rtf",
        "text/csv", "text/tab-separated-values", "text/markdown", "application/json",
        "application/xml", "text/xml", "text/html",
    )

    fun supports(mimeType: String, size: Long, width: Int?, height: Int?): Boolean {
        if (size !in 1L..ChatGptWebNativeAttachmentReader.MAX_BYTES.toLong()) return false
        if (mimeType in documentTypes) return true
        // The production photo picker already decodes and normalizes to this pixel budget.
        return mimeType in imageTypes && width != null && height != null &&
            width in 1..16_384 && height in 1..16_384 && width.toLong() * height <= 4_000_000L
    }
}
