package com.elon.app

import java.io.File

internal object ChatGptWebAcceptanceAttachmentFixture {
    const val ID = "fixed_ascii_text_v1"
    const val FILE_NAME = "elon-chatgpt-attachment-fixture-v1.txt"
    const val MIME_TYPE = "text/plain"

    fun prepare(cacheDir: File): PendingAttachment {
        val directory = fixtureDirectory(cacheDir).apply { mkdirs() }
        require(directory.isDirectory) { "Unable to create acceptance fixture directory" }
        val target = File(directory, FILE_NAME)
        val temporary = File(directory, "$FILE_NAME.tmp")
        runCatching { temporary.delete() }
        temporary.writeText(CONTENT, Charsets.UTF_8)
        require(temporary.length() == CONTENT.toByteArray(Charsets.UTF_8).size.toLong()) {
            "Acceptance fixture length mismatch"
        }
        if (target.exists()) require(target.delete()) { "Unable to replace acceptance fixture" }
        require(temporary.renameTo(target)) { "Unable to commit acceptance fixture" }
        return PendingAttachment(
            kind = "document",
            displayLabel = "测试文档",
            displayName = FILE_NAME,
            fileName = FILE_NAME,
            mimeType = MIME_TYPE,
            file = target,
        )
    }

    fun matches(cacheDir: File, attachment: PendingAttachment): Boolean =
        attachment.fileName == FILE_NAME &&
            attachment.mimeType == MIME_TYPE &&
            runCatching {
                attachment.file.canonicalFile == File(fixtureDirectory(cacheDir), FILE_NAME).canonicalFile
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
