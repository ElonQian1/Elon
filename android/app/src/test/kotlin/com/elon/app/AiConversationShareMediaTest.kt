package com.elon.app

import com.elon.app.AiConversationShareMediaException.Reason
import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class AiConversationShareMediaTest {
    @get:Rule val temporary = TemporaryFolder()
    private val handle = "image_" + "1".repeat(16)
    private val bytes = byteArrayOf(1, 2, 3, 4)
    private fun file(path: String, content: ByteArray = bytes): File = File(temporary.root, path).apply {
        parentFile.mkdirs()
        writeBytes(content)
    }
    private fun preparer(info: AiConversationShareMediaImageInfo? = AiConversationShareMediaImageInfo("image/jpeg", 2, 2)) =
        AiConversationShareMediaPreparer(AiConversationShareMediaSources(temporary.root, "com.elon.app")) { info }
    private fun image() = WebChatProductionContentPart("image", "private preview label",
        assetHandle = handle, imageSource = file("chatgpt-web-image-assets-v1/$handle.jpg").absolutePath,
        mediaType = "image/png", imageWidth = 100, imageHeight = 100)
    private fun message(id: String = "selected", parts: List<WebChatProductionContentPart> = emptyList(),
        attachments: List<ChatAttachment>? = null) = ChatMessage(role = "friend", content = "# Markdown\n\nFull body.",
        id = id, attachments = attachments, webChatMessage = WebChatProductionMessage("chatgpt_web", id,
            emptySet(), renderMarkdown = true, contentParts = parts))
    private fun attachment(path: String = "pending_attachments/image.png") = ChatAttachment(kind = "image",
        localPath = file(path).absolutePath, url = "https://vendor.invalid/private?token=secret",
        displayName = "private original name.png", mimeType = "image/png", sizeBytes = 999_999)

    @Test fun uploadsOnlySelectedImagesWithIndexBindingsAndByteDerivedMetadata() = runBlocking {
        val unselected = message("unselected", listOf(image().copy(imageSource = "/private/not-selected")))
        val selected = message("selected", listOf(image()), listOf(attachment()))
        val messages = listOf(unselected, selected)
        val uploads = mutableListOf<AiConversationShareMediaUpload>()
        val result = preparer().prepareMessages(messages.filter { it.id == "selected" }) {
            uploads += it
            "protected-asset-${uploads.size}"
        }
        assertEquals(listOf("m0p0", "m0a0"), result.keys.toList())
        assertEquals(listOf("protected-asset-1", "protected-asset-2"), result.values.toList())
        assertEquals(listOf("image-1.jpg", "image-2.jpg"), uploads.map { it.fileName })
        assertTrue(uploads.first().isPreview)
        assertFalse(uploads.last().isPreview)
        uploads.forEach {
            assertArrayEquals(bytes, it.bytes)
            assertEquals("image/jpeg", it.mimeType)
            assertEquals(2, it.width)
            assertEquals(2, it.height)
        }
    }

    @Test fun preservesCompleteMarkdownWritingAndCodeWithoutAnyUpload() = runBlocking {
        val code = WebChatTextBlock("code", "code", "Program", "kotlin", "println(\"complete\")\n", true)
        val writing = WebChatTextBlock("writing", "writing", "Document", "", "First\n\nLast\n", true)
        val selected = message(parts = listOf(WebChatProductionContentPart("code", "code", textBlock = code),
            WebChatProductionContentPart("writing_block", "writing", textBlock = writing)))
        val originalContent = selected.content
        val result = preparer().prepareMessages(listOf(selected)) { throw AssertionError("Unexpected upload") }
        assertTrue(result.isEmpty())
        assertEquals(originalContent, selected.content)
        assertEquals(code, selected.webChatMessage!!.contentParts[0].textBlock)
        assertEquals(writing, selected.webChatMessage!!.contentParts[1].textBlock)
    }

    @Test fun frozenMessageCallbackRemainsCompatibleWithTheParentCodecKeys() = runBlocking {
        val selected = message("chatgpt_web:selected", listOf(image()), listOf(attachment()))
        val results = preparer().prepare(listOf(selected)) { it.key }
        assertEquals(mapOf(
            AiConversationShareMediaKey("chatgpt_web:selected", AiConversationShareMediaKey.Location.CONTENT_PART, 0) to "m0p0",
            AiConversationShareMediaKey("chatgpt_web:selected", AiConversationShareMediaKey.Location.ATTACHMENT, 0) to "m0a0",
        ), results)
    }

    @Test fun artifactBackedBlocksAndFinanceCardsRemainAvailableForCanonicalCodec() = runBlocking {
        val writing = WebChatTextBlock("document", "writing", "Complete document", "", "First\n\nLast\n", true)
        val code = WebChatTextBlock("program", "code", "Program", "kotlin", "println(1)\n", true)
        val finance = WebChatProductionRichCard(WebChatProductionRichCard.Kind.FINANCE, "Synthetic finance",
            symbol = "TEST", primaryValue = "100.00", trend = WebChatProductionRichCard.Trend.POSITIVE,
            periods = listOf(WebChatProductionRichCard.Period("day", "1D", true)),
            metrics = listOf(WebChatProductionRichCard.Metric("Open", "99.00")),
            series = listOf(WebChatProductionRichCard.Series("price", "Price", "$")),
            points = listOf(WebChatProductionRichCard.Point("09:00", listOf(99.0)),
                WebChatProductionRichCard.Point("10:00", listOf(100.0))))
        val original = listOf(WebChatProductionContentPart("artifact", "writing", textBlock = writing),
            WebChatProductionContentPart("artifact", "code", textBlock = code),
            WebChatProductionContentPart("chart", "finance", richCard = finance))
        val selected = message(parts = original).apply { content = "" }
        assertTrue(preparer().prepareMessages(listOf(selected)) { unexpectedUpload() }.isEmpty())
        assertEquals(original, selected.webChatMessage!!.contentParts)
        assertEquals(finance.points, selected.webChatMessage!!.contentParts.last().richCard!!.points)
        assertEquals(writing.content, selected.webChatMessage!!.contentParts.first().textBlock!!.content)
    }

    @Test fun acceptsInlineMarkdownPartsButRejectsUnrepresentedEmptyParts() = runBlocking {
        for (type in listOf("code", "table", "math", "citation")) {
            val selected = message(parts = listOf(WebChatProductionContentPart(type, type)))
            assertTrue(preparer().prepareMessages(listOf(selected)) { unexpectedUpload() }.isEmpty())
            selected.content = ""
            rejected(Reason.UNSUPPORTED_PART) {
                preparer().prepareMessages(listOf(selected)) { unexpectedUpload() }
            }
        }
    }

    @Test fun coverIsOnlyTheFirstSelectedImageAndIsAbsentForTextOnlySelections() {
        val noImages = AiConversationShareMedia.plan(listOf(message()))
        assertEquals(null, AiConversationShareMedia.firstImageKey(noImages))
        val selected = message(parts = listOf(WebChatProductionContentPart("code", "inline"), image()),
            attachments = listOf(attachment()))
        val plan = AiConversationShareMedia.plan(listOf(message("text-first"), selected))
        assertEquals(listOf("m1p1", "m1a0"), plan.map { it.key })
        assertEquals("m1p1", AiConversationShareMedia.firstImageKey(plan))
    }

    @Test fun supportsTwoHundredSelectedMessagesAndExactlyTwelveImages() {
        val selected = List(200) { message("row-$it") }
        assertTrue(AiConversationShareMedia.plan(selected).isEmpty())
        rejected(Reason.INVALID_SELECTION) { AiConversationShareMedia.plan(selected + message("too-many")) }
        val last = selected.last().copy(attachments = List(12) { attachment() })
        val plan = AiConversationShareMedia.plan(selected.dropLast(1) + last)
        assertEquals(12, plan.size)
        assertEquals("m199a0", AiConversationShareMedia.firstImageKey(plan))
        assertEquals("m199a11", plan.last().key)
    }

    @Test fun pendingOrMissingSelectedImagesAbortBeforeAnyUpload() {
        val ready = image()
        for (unready in listOf(ready.copy(previewPending = true), ready.copy(imageSource = null))) {
            rejected(Reason.IMAGE_PENDING_RETRY) {
                preparer().prepareMessages(listOf(message(parts = listOf(ready, unready)))) { unexpectedUpload() }
            }
        }
        rejected(Reason.IMAGE_PENDING_RETRY) {
            preparer().prepareMessages(listOf(message(attachments = listOf(attachment())).apply { sendStatus = "pending" })) {
                unexpectedUpload()
            }
        }
    }

    @Test fun unsupportedPartsAndFilesNeverBecomeSilentTextOnlyShares() {
        for (type in listOf("file", "audio", "video", "artifact", "interactive", "rich_card", "future_type")) {
            rejected(Reason.UNSUPPORTED_PART) {
                preparer().prepareMessages(listOf(message(parts = listOf(image(), WebChatProductionContentPart(type, "unsupported"))))) {
                    unexpectedUpload()
                }
            }
        }
        rejected(Reason.UNSUPPORTED_ATTACHMENT) {
            preparer().prepareMessages(listOf(message(attachments = listOf(attachment(), ChatAttachment(kind = "file", mimeType = "application/pdf"))))) {
                unexpectedUpload()
            }
        }
        rejected(Reason.UNSUPPORTED_ATTACHMENT) {
            val annotated = attachment().copy(annotations = listOf(
                ChatImageAnnotation(x = 0f, y = 0f, width = 0.5f, height = 0.5f, note = "Selected annotation")))
            preparer().prepareMessages(listOf(message(attachments = listOf(annotated)))) { unexpectedUpload() }
        }
    }

    @Test fun incompleteOrAbsentTextBlocksAreExplicitRetryFailures() {
        for (block in listOf(null, WebChatTextBlock("writing", "writing", "", "", "partial", false))) {
            rejected(Reason.INCOMPLETE_TEXT_BLOCK_RETRY) {
                preparer().prepareMessages(listOf(message(parts = listOf(WebChatProductionContentPart("writing_block", "writing", textBlock = block))))) {
                    unexpectedUpload()
                }
            }
        }
        rejected(Reason.INCOMPLETE_TEXT_BLOCK_RETRY) {
            val partial = WebChatTextBlock("document", "writing", "", "", "partial", false)
            preparer().prepareMessages(listOf(message(parts = listOf(
                WebChatProductionContentPart("artifact", "writing", textBlock = partial))))) { unexpectedUpload() }
        }
    }

    @Test fun rejectsMissingOrInvalidBytesBeforeUploadingAnyOtherSelectedImage() {
        val missing = image().copy(assetHandle = "image_${"2".repeat(16)}",
            imageSource = File(temporary.root, "chatgpt-web-image-assets-v1/image_${"2".repeat(16)}.jpg").absolutePath)
        rejected(Reason.LOCAL_IMAGE_MISSING_RETRY) {
            preparer().prepareMessages(listOf(message(parts = listOf(image(), missing)))) { unexpectedUpload() }
        }
        rejected(Reason.INVALID_IMAGE) {
            preparer(null).prepareMessages(listOf(message(parts = listOf(image())))) { unexpectedUpload() }
        }
    }

    @Test fun rejectsUnsupportedRasterFormatsAndExcessiveDimensions() {
        for (info in listOf(AiConversationShareMediaImageInfo("image/svg+xml", 2, 2),
            AiConversationShareMediaImageInfo("image/gif", 2, 2),
            AiConversationShareMediaImageInfo("image/png", 0, 2),
            AiConversationShareMediaImageInfo("image/png", 20_000, 2),
            AiConversationShareMediaImageInfo("image/png", 10_000, 10_000))) {
            rejected(Reason.INVALID_IMAGE) {
                preparer(info).prepareMessages(listOf(message(parts = listOf(image())))) { unexpectedUpload() }
            }
        }
    }

    @Test fun rejectsDuplicateSelectionKeysAndBoundsTheWholeBatch() {
        val source = AiConversationShareMedia.plan(listOf(message(parts = listOf(image())))).single()
        rejected(Reason.INVALID_SELECTION) { preparer().prepare(listOf(source, source)) { unexpectedUpload() } }
        rejected(Reason.INVALID_SELECTION) { preparer().prepareMessages(emptyList()) { unexpectedUpload() } }
        val photo = attachment()
        rejected(Reason.BATCH_TOO_LARGE) {
            preparer().prepareMessages(listOf(message(attachments = List(13) { photo }))) { unexpectedUpload() }
        }
        RandomAccessFile(File(photo.localPath!!), "rw").use { it.setLength(AiConversationShareMediaPreparer.MAX_ASSET_BYTES.toLong()) }
        rejected(Reason.BATCH_TOO_LARGE) {
            preparer().prepareMessages(listOf(message(attachments = List(5) { photo }))) { unexpectedUpload() }
        }
    }

    @Test fun uploadCallbacksCannotRetargetLaterSelectionEntries() = runBlocking {
        val first = message("first", attachments = listOf(attachment("attachments/first.png")))
        val second = message("second", attachments = listOf(attachment("attachments/second.png")))
        val selected = mutableListOf(first, second)
        val result = preparer().prepareMessages(selected) {
            second.id = "replacement"
            second.attachments = listOf(ChatAttachment(kind = "image", localPath = "/private/secret.png"))
            selected.clear()
            "receipt"
        }
        assertEquals(listOf("m0a0", "m1a0"), result.keys.toList())
    }

    @Test fun uploadFailureAndCancellationNeverReturnPartialBindings() {
        for (failure in listOf(IllegalStateException("upload rejected"), CancellationException("cancelled"))) {
            var uploads = 0
            try {
                runBlocking {
                    preparer().prepareMessages(listOf(message(attachments = List(3) { attachment() }))) {
                        uploads++
                        if (uploads == 2) throw failure
                        "uploaded-asset-$uploads"
                    }
                }
                throw AssertionError("Expected upload failure")
            } catch (error: Exception) {
                // Coroutine stack-trace recovery may copy exceptions at the withContext boundary.
                assertEquals(failure.javaClass, error.javaClass)
                assertEquals(failure.message, error.message)
            }
            assertEquals("No upload may follow the failed or cancelled second upload", 2, uploads)
        }
    }

    @Test fun callerCancellationStopsUploadsWithoutReturningPartialBindings() = runBlocking {
        val uploadStarted = CompletableDeferred<Unit>()
        var uploads = 0
        var returnedBindings = false
        val job = launch {
            preparer().prepareMessages(listOf(message(attachments = List(3) { attachment() }))) {
                uploads++
                uploadStarted.complete(Unit)
                awaitCancellation()
            }
            returnedBindings = true
        }
        withTimeout(5_000) {
            uploadStarted.await()
            job.cancelAndJoin()
        }
        assertTrue(job.isCancelled)
        assertFalse("Cancellation must not return even partial bindings", returnedBindings)
        assertEquals(1, uploads)
    }

    @Test fun chineseFailuresAreActionableAndNeverIncludeSourceMaterial() {
        for (reason in Reason.values()) {
            val error = AiConversationShareMediaException(reason)
            val text = error.userMessage()
            assertTrue(text.any { it in '\u4e00'..'\u9fff' })
            assertFalse(text.contains("https:"))
            assertFalse(text.contains("content:"))
            assertFalse(text.contains("private-name"))
            assertEquals(reason.name, error.message)
        }
        assertTrue(AiConversationShareMediaException(Reason.ASSET_TOO_LARGE).userMessage().contains("12 MiB"))
        assertTrue(AiConversationShareMediaException(Reason.BATCH_TOO_LARGE).userMessage().contains("48 MiB"))
    }

    private fun unexpectedUpload(): String = throw AssertionError("Must reject before upload")
    private suspend fun <T : Any> AiConversationShareMediaPreparer.prepareMessages(
        selected: List<ChatMessage>, upload: suspend (AiConversationShareMediaUpload) -> T,
    ): Map<String, T> = prepare(AiConversationShareMedia.plan(selected), upload)
    private fun rejected(reason: Reason, block: suspend () -> Unit) {
        try { runBlocking { block() }; throw AssertionError("Expected $reason") }
        catch (error: AiConversationShareMediaException) { assertEquals(reason, error.reason) }
    }
}
