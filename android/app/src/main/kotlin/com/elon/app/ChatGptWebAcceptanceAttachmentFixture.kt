package com.elon.app

import java.io.File

internal object ChatGptWebAcceptanceAttachmentFixture {
    const val ID = "fixed_ascii_text_v1"
    const val FILE_NAME = "elon-chatgpt-attachment-fixture-v1.txt"
    const val MIME_TYPE = "text/plain"
    const val MEDIA_BATCH_ID = "fixed_media_batch_v1"
    const val IMAGE_NAME = "elon-chatgpt-media-fixture-v1.png"
    const val PDF_NAME = "elon-chatgpt-media-fixture-v1.pdf"
    val supportedIds = setOf(ID, MEDIA_BATCH_ID)

    internal data class Spec(val name: String, val mime: String, val label: String)
    private val text = Spec(FILE_NAME, MIME_TYPE, "测试文档")
    private val media = listOf(text, Spec(IMAGE_NAME, "image/png", "测试图片"),
        Spec(PDF_NAME, "application/pdf", "测试PDF"))

    fun supports(id: String): Boolean = id in supportedIds
    private fun specs(id: String): List<Spec> = when (id) {
        ID -> listOf(text)
        MEDIA_BATCH_ID -> media
        else -> error("Unknown acceptance fixture")
    }

    fun matchesSelection(cacheDir: File, attachments: List<PendingAttachment>, id: String): Boolean =
        supports(id) && attachments.size == specs(id).size &&
            attachments.map { it.fileName }.toSet() == specs(id).map { it.name }.toSet() &&
            attachments.all { matches(cacheDir, it) }

    fun prepareBatch(
        cacheDir: File,
        id: String,
        writeMedia: (Spec, File) -> Unit = ChatGptWebAcceptanceMediaFixture::write,
    ): List<PendingAttachment> {
        val selected = specs(id)
        val prepared = mutableListOf<PendingAttachment>()
        try {
            selected.forEach { spec -> prepared += prepare(cacheDir, spec, writeMedia) }
            return prepared
        } catch (error: Exception) {
            prepared.forEach { it.file.delete() }
            throw error
        }
    }

    fun prepare(cacheDir: File): PendingAttachment = prepare(cacheDir, text) { _, _ ->
        error("Text fixture does not use a media writer")
    }

    private fun prepare(cacheDir: File, spec: Spec, writeMedia: (Spec, File) -> Unit): PendingAttachment {
        val directory = fixtureDirectory(cacheDir).apply { mkdirs() }
        require(directory.isDirectory) { "Unable to create acceptance fixture directory" }
        val target = File(directory, spec.name)
        val temporary = File(directory, "${spec.name}.tmp")
        try {
            if (spec == text) temporary.writeText(CONTENT, Charsets.UTF_8) else writeMedia(spec, temporary)
            require(temporary.length() in 1L..262_144L) { "Acceptance fixture size invalid" }
            if (target.exists()) require(target.delete()) { "Unable to replace acceptance fixture" }
            require(temporary.renameTo(target)) { "Unable to commit acceptance fixture" }
        } finally {
            temporary.delete()
        }
        return PendingAttachment(
            kind = if (spec.mime == "image/png") "image" else "document",
            displayLabel = spec.label,
            displayName = spec.name,
            fileName = spec.name,
            mimeType = spec.mime,
            file = target,
            imageWidth = if (spec.mime == "image/png") 512 else null,
            imageHeight = if (spec.mime == "image/png") 384 else null,
        )
    }

    fun matches(cacheDir: File, attachment: PendingAttachment): Boolean =
        media.any { it.name == attachment.fileName && it.mime == attachment.mimeType } &&
            runCatching {
                attachment.file.canonicalFile == File(fixtureDirectory(cacheDir), attachment.fileName).canonicalFile
            }.getOrDefault(false)

    fun cleanup(cacheDir: File) {
        runCatching { fixtureDirectory(cacheDir).deleteRecursively() }
    }

    internal fun expectedContent(): String = CONTENT

    private fun fixtureDirectory(cacheDir: File): File = File(cacheDir, DIRECTORY)

    private const val DIRECTORY = "chatgpt_web_acceptance_fixture"
    private const val CONTENT = "ELON_CHATGPT_ATTACHMENT_FIXTURE_V1=ready\nNo user data is stored in this file.\n"
}

internal enum class ChatGptWebAcceptanceFixtureStageResult(val wireValue: String) {
    STAGED("staged"),
    ALREADY_STAGED("already_staged"),
    PENDING_ATTACHMENTS_PRESENT("pending_attachments_present"),
    FAILED("failed"),
}
