package com.elon.app.chatgptweb

internal object ChatGptWebComposerOptionSemantics {
    const val MODEL = "model"
    const val ATTACHMENT_CAMERA = "attachment_camera"
    const val ATTACHMENT_PHOTOS = "attachment_photos"
    const val ATTACHMENT_FILE = "attachment_file"
    const val WEB_SEARCH = "web_search"
    const val DEEP_RESEARCH = "deep_research"
    const val IMAGE_GENERATION = "image_generation"
    const val CANVAS = "canvas"
    const val STUDY = "study"
    const val AGENT = "agent"
    const val TOOL = "tool"

    val KNOWN = setOf(
        MODEL,
        ATTACHMENT_CAMERA,
        ATTACHMENT_PHOTOS,
        ATTACHMENT_FILE,
        WEB_SEARCH,
        DEEP_RESEARCH,
        IMAGE_GENERATION,
        CANVAS,
        STUDY,
        AGENT,
        TOOL,
    )

    fun fallback(section: String): String = if (section == MODEL) MODEL else TOOL

    fun isAttachment(value: String): Boolean = value in ATTACHMENTS

    private val ATTACHMENTS = setOf(
        ATTACHMENT_CAMERA,
        ATTACHMENT_PHOTOS,
        ATTACHMENT_FILE,
    )
}
