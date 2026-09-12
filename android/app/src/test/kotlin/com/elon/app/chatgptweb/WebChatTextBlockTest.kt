package com.elon.app.chatgptweb

import com.elon.app.WebChatProductionContentPart
import com.elon.app.WebChatProductionRichContentPolicy
import com.elon.app.WebChatTextBlock
import com.elon.app.WebChatTextBlockExport
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class WebChatTextBlockTest {
    private fun block(kind: String = "writing", language: String = "") = WebChatTextBlock(
        "block-1", kind, "Draft", language, "  Original\r\n\r\n\r\nbody  \r\n", true,
    )

    @Test fun blockWireRoundTripPreservesOriginalWhitespace() {
        val source = block()
        assertEquals(source, WebChatTextBlock.parse(source.toJson()))
        assertEquals("", WebChatTextBlock.parse(source.copy(content = "").toJson())?.content)
    }

    @Test fun malformedBodiesAndUnsupportedVersionsAreRejectedInsteadOfTruncated() {
        assertNull(WebChatTextBlock.parse(block().toJson().put("content", JSONObject.NULL)))
        assertNull(WebChatTextBlock.parse(block().toJson().put("content", 12)))
        assertNull(WebChatTextBlock.parse(block().toJson().put("content", "a".repeat(WebChatTextBlock.MAX_CONTENT + 1))))
        assertNull(WebChatTextBlock.parse(block().toJson().put("version", 2)))
        assertNull(WebChatTextBlock.parse(block().toJson().put("kind", "executable")))
    }

    @Test fun nativeProtocolKeepsGeneratedBlocksReadOnlyUntilMessageCompletion() {
        val wire = JSONObject().put("type", "writing_block").put("text", "Draft").put("textBlock", block().toJson())
        val message = JSONObject().put("state", "streaming").put("content", JSONArray().put(wire))
        assertFalse(ChatGptWebMessagePartParser.parse(message).single().textBlock!!.complete)
        message.put("state", "completed")
        assertEquals(block(), ChatGptWebMessagePartParser.parse(message).single().textBlock)
        wire.put("type", "code")
        assertNull(ChatGptWebMessagePartParser.parse(message).single().textBlock)
    }

    @Test fun structuredCodeGetsANativeActionButLegacyMetadataDoesNotDuplicateTheBody() {
        val legacy = WebChatProductionContentPart("code", "Code")
        val structured = legacy.copy(textBlock = block("code", "python"))
        assertEquals(listOf(structured), WebChatProductionRichContentPolicy.fallbackParts(listOf(legacy, structured)))
    }

    @Test fun localExportsOfferRealTextFormatsAndReuseSourceLanguageExtensions() {
        assertEquals(listOf("md", "txt"), WebChatTextBlockExport.formats(block()).map { it.extension })
        assertEquals(listOf("py", "txt"), WebChatTextBlockExport.formats(block("code", "py")).map { it.extension })
        assertEquals(listOf("txt"), WebChatTextBlockExport.formats(block("code", "unknown")).map { it.extension })
        assertFalse(WebChatTextBlockExport.formats(block()).any { it.key == "pdf" || it.key == "docx" })
    }

    @Test fun exportNamesCannotEscapeTheDownloadDirectory() {
        val format = WebChatTextBlockExport.formats(block()).first()
        val name = WebChatTextBlockExport.name("../draft\\document", format)
        assertFalse(name.contains('/'))
        assertFalse(name.contains('\\'))
        assertTrue(name.endsWith(".md"))
        assertEquals("draft.md", WebChatTextBlockExport.name("draft.md", format))
    }

    @Test fun cachedSnapshotRestoresBodyWithoutGrantingProviderWriteAuthority() {
        val original = block()
        val part = ChatGptWebMessagePart("writing_block", "Draft", textBlock = original)
        val snapshot = ChatGptWebSnapshot("Conversation", "https://chatgpt.com/c/test", "", listOf(
            ChatGptWebMessage("answer-1", "assistant", "Body", "completed", listOf(part)),
        ), true, true, false, currentModel = "auto", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY)
        val restored = WebChatSnapshotCacheCodec.decode(WebChatSnapshotCacheCodec.encode(WebChatSnapshotCache(snapshot, 1)))!!.snapshot
        assertEquals(original, restored.messages.single().parts.single().textBlock)
        assertFalse(restored.authenticated)
        assertFalse(restored.composerReady)
    }
}
