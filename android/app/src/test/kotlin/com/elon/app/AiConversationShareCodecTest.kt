package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class AiConversationShareCodecTest {
    private val card = AiConversationShareCard("ai_snapshot_selected", "group_selected", "Card title",
        "Card summary", "chatgpt", "Sharer", 2)
    private val previewAsset = AiConversationShareAsset("article_media_preview", "image/png")
    private val attachmentAsset = AiConversationShareAsset("article_media_attachment", "image/jpeg")

    @Test fun encodesExactWireFieldsCanonicalRolesAndIdsWithoutPrivateSourceMetadata() {
        val (draft, assets) = imageFixture()
        val document = encode(draft, assets)
        assertEquals(setOf("schema", "provider", "title", "summary", "messages", "cover_asset_id"), keys(document))
        assertEquals("elon.ai_conversation_share.v1", document.getString("schema"))
        assertEquals("chatgpt", document.getString("provider"))
        assertEquals("Selected title", document.getString("title"))
        assertEquals("Selected summary", document.getString("summary"))
        assertEquals(previewAsset.id, document.getString("cover_asset_id"))
        val rows = document.getJSONArray("messages")
        assertEquals(2, rows.length())
        for (index in 0..1) {
            val row = rows.getJSONObject(index)
            assertEquals(setOf("id", "role", "content", "created_at_ms", "gap_before", "parts"), keys(row))
            assertEquals("message_$index", row.getString("id"))
            assertEquals(if (index == 0) "user" else "assistant", row.getString("role"))
            assertEquals(draft.messages[index].content, row.getString("content"))
            assertEquals(index == 1, row.getBoolean("gap_before"))
            assertEquals(1, row.getJSONArray("parts").length())
            val part = row.getJSONArray("parts").getJSONObject(0)
            assertEquals(setOf("type", "label", "asset_id", "media_type"), keys(part))
            val asset = if (index == 0) previewAsset else attachmentAsset
            assertEquals("image", part.getString("type"))
            assertEquals(asset.id, part.getString("asset_id"))
            assertEquals(asset.mimeType, part.getString("media_type"))
        }
        assertEquals(123L, rows.getJSONObject(0).getLong("created_at_ms"))
        assertEquals(0L, rows.getJSONObject(1).getLong("created_at_ms"))
        val wire = document.toString()
        for (forbidden in listOf("provider-source-user", "provider-source-assistant", "provider-message-context",
            "app-only-image.jpg", "app-only-attachment.png", "private-original.png", "private-conversation",
            "download_" + "b".repeat(32), "image_" + "a".repeat(16), "vendor.invalid", "private-token",
            "imageSource", "imageOriginal", "sourceMessageId", "localPath", "assetHandle", "actions",
            "private-model", "private-node", "private-evidence", "private-codex")) {
            assertFalse("Transport metadata leaked: $forbidden", wire.contains(forbidden))
        }
        assertEquals("/app/cache/app-only-image.jpg", draft.messages[0].webChatMessage!!.contentParts[0].imageSource)
    }

    @Test fun keepsMarkdownAndCompleteArtifactBackedBlocksByteForByteWithCanonicalBlockIds() {
        val markdown = "# Heading\n\n| Name | Value |\n| --- | --- |\n| A | 1 |\n\n```kotlin\nval x = 1\n```\n"
        val code = WebChatTextBlock("provider-code-block", "code", "Program", "kotlin",
            "  fun main() {\n    println(\"whole body\")\n  }\n", true, "provider-code-message")
        val writing = WebChatTextBlock("provider-writing-block", "writing", "Document", "",
            "First paragraph.\n\n**Last paragraph.**  \n", true, "provider-writing-message")
        val source = message("friend", "provider-rich-message", markdown, listOf(
            WebChatProductionContentPart("artifact", "Program", textBlock = code),
            WebChatProductionContentPart("artifact", "Document", textBlock = writing)))
        val document = encode(draft(listOf(source)))
        val row = document.getJSONArray("messages").getJSONObject(0)
        assertEquals(markdown, row.getString("content"))
        val parts = row.getJSONArray("parts")
        assertEquals(2, parts.length())
        for ((index, block) in listOf(code, writing).withIndex()) {
            val part = parts.getJSONObject(index)
            assertEquals(setOf("type", "label", "text_block"), keys(part))
            assertEquals(if (index == 0) "code" else "writing_block", part.getString("type"))
            val encoded = part.getJSONObject("text_block")
            assertEquals(setOf("version", "id", "kind", "title", "language", "content", "complete"), keys(encoded))
            assertEquals(1, encoded.getInt("version"))
            assertEquals("block_$index", encoded.getString("id"))
            assertEquals(block.kind, encoded.getString("kind"))
            assertEquals(block.title, encoded.getString("title"))
            assertEquals(block.language, encoded.getString("language"))
            assertEquals(block.content, encoded.getString("content"))
            assertTrue(encoded.getBoolean("complete"))
            assertFalse(document.toString().contains(block.id))
            assertFalse(document.toString().contains(block.sourceMessageId!!))
        }
        val restored = AiConversationShareCodec.snapshot(envelope(document), card).messages.single()
        assertEquals(markdown, restored.content)
        val blocks = restored.webChatMessage!!.contentParts.map { it.textBlock!! }
        assertEquals(listOf(code.content, writing.content), blocks.map { it.content })
        assertEquals(listOf("block_0", "block_1"), blocks.map { it.id })
        assertTrue(blocks.all { it.complete && it.sourceMessageId == null })
        assertEquals(code, source.webChatMessage!!.contentParts.first().textBlock)
    }

    @Test fun inlineCodeTableMathAndCitationPartsDoNotDuplicateOrRewriteMarkdown() {
        val markdown = "```text\nkept exactly\n```\n\n|A|B|\n|-|-|\n|1|2|\n\nEquation: x = 2.\n[Source](https://example.org/)\n"
        val parts = listOf("code", "table", "math", "citation").map { WebChatProductionContentPart(it, "inline $it") }
        val row = encode(draft(listOf(message("friend", "provider-inline", markdown, parts))))
            .getJSONArray("messages").getJSONObject(0)
        assertEquals(markdown, row.getString("content"))
        assertEquals(0, row.getJSONArray("parts").length())
    }

    @Test fun financeCardPreservesAllStructuredDisplayFieldsThroughTheWire() {
        val finance = WebChatProductionRichCard(WebChatProductionRichCard.Kind.FINANCE, "Synthetic instrument",
            description = "Selected finance card", symbol = "TEST", primaryValue = "100.00", secondaryValue = "+1.00",
            trend = WebChatProductionRichCard.Trend.POSITIVE,
            periods = listOf(WebChatProductionRichCard.Period("day", "1D", true)),
            metrics = listOf(WebChatProductionRichCard.Metric("Open", "99.00")),
            series = listOf(WebChatProductionRichCard.Series("price", "Price", "$", " USD")),
            points = listOf(WebChatProductionRichCard.Point("09:00", listOf(99.0)),
                WebChatProductionRichCard.Point("10:00", listOf(100.0))))
        val document = encode(draft(listOf(message("friend", "provider-finance", "", listOf(
            WebChatProductionContentPart("chart", "Finance", richCard = finance))))))
        val part = document.getJSONArray("messages").getJSONObject(0).getJSONArray("parts").getJSONObject(0)
        assertEquals(setOf("type", "label", "card"), keys(part))
        assertEquals("rich_card", part.getString("type"))
        val encoded = part.getJSONObject("card")
        assertEquals(setOf("kind", "title", "description", "symbol", "primary_value", "secondary_value",
            "trend", "periods", "metrics", "series", "points"), keys(encoded))
        assertEquals("finance", encoded.getString("kind"))
        assertEquals("100.00", encoded.getString("primary_value"))
        assertEquals("positive", encoded.getString("trend"))
        assertEquals(100.0, encoded.getJSONArray("points").getJSONObject(1).getJSONArray("values").getDouble(0), 0.0)
        val restored = AiConversationShareCodec.snapshot(envelope(document), card).messages.single()
        assertEquals(finance, restored.webChatMessage!!.contentParts.single().richCard)
    }

    @Test fun missingOrMisboundAssetsRejectInsteadOfDroppingSelectedImages() {
        val (draft, assets) = imageFixture()
        val partKey = assets.keys.first()
        val wrongMessageKey = partKey.copy(messageId = "unselected-message")
        val wrongPartKey = partKey.copy(index = 1)
        for (provided in listOf(emptyMap(), mapOf(partKey to previewAsset),
            assets.minus(partKey) + (wrongMessageKey to previewAsset),
            assets.minus(partKey) + (wrongPartKey to previewAsset))) {
            assertThrows(IllegalStateException::class.java) { encode(draft, provided) }
        }
    }

    @Test fun incompleteWritingAndUnsupportedFilesRejectInsteadOfPublishingPartialContent() {
        val partial = WebChatTextBlock("partial", "writing", "Document", "", "not complete", false)
        val source = message("friend", "provider-partial", "", listOf(
            WebChatProductionContentPart("artifact", "Document", textBlock = partial)))
        assertThrows(IllegalArgumentException::class.java) { encode(draft(listOf(source))) }
        val file = message("user", "provider-file", "Selected file").copy(
            attachments = listOf(ChatAttachment(kind = "file", mimeType = "application/pdf", displayName = "Document")))
        assertThrows(IllegalArgumentException::class.java) { encode(draft(listOf(file))) }
    }

    @Test fun snapshotRejectsWrongSnapshotOrGroupBeforeTrustingTheDocument() {
        val (draft, assets) = imageFixture()
        for ((key, wrongValue) in listOf("snapshot_id" to "ai_snapshot_other", "group_id" to "group_other")) {
            val response = envelope(encode(draft, assets)).put(key, wrongValue)
            assertThrows(IllegalArgumentException::class.java) { AiConversationShareCodec.snapshot(response, card) }
        }
    }

    @Test fun imagesRolesAndSelectionGapsRoundtripWithoutRestoringPrivateFetchState() {
        val (draft, assets) = imageFixture()
        val restored = AiConversationShareCodec.snapshot(envelope(encode(draft, assets)), card)
        assertEquals(setOf(1), restored.gaps)
        assertEquals(listOf("message_0", "message_1"), restored.messages.map { it.id })
        assertEquals(listOf("user", "friend"), restored.messages.map { it.role })
        assertEquals(draft.messages.map { it.content }, restored.messages.map { it.content })
        assertEquals(listOf("Snapshot owner", "ChatGPT"), restored.messages.map { it.senderLabel })
        assertEquals("owner_selected", restored.ownerId)
        assertEquals(card.id, restored.card.id)
        assertEquals(card.groupId, restored.card.groupId)
        assertEquals("Selected title", restored.card.title)
        assertEquals("Selected summary", restored.card.summary)
        assertEquals(2, restored.card.messageCount)
        assertEquals(previewAsset.id, restored.card.coverAssetId)
        for ((index, row) in restored.messages.withIndex()) {
            val metadata = row.webChatMessage!!
            assertEquals("shared_chatgpt", metadata.providerWireValue)
            assertEquals("message_$index", metadata.sourceMessageId)
            assertTrue(metadata.renderMarkdown)
            assertTrue(metadata.actions.isEmpty())
            val image = metadata.contentParts.single()
            val asset = if (index == 0) previewAsset else attachmentAsset
            assertEquals("image", image.type)
            assertEquals(asset.id, image.assetHandle)
            assertEquals(asset.mimeType, image.mediaType)
            assertTrue(image.previewPending)
            assertNull(image.imageSource)
            assertNull(image.imageOriginal)
            assertNull(row.attachments)
        }
    }

    @Test fun optionalCoverIsNullWhenDisabledOrNoSelectedImageExists() {
        val (selectedDraft, assets) = imageFixture()
        val disabled = AiConversationShareCodec.document(selectedDraft, "Selected title", "Selected summary", assets, false)
        assertTrue(disabled.has("cover_asset_id"))
        assertTrue(disabled.isNull("cover_asset_id"))
        assertNull(AiConversationShareCodec.snapshot(envelope(disabled), card).card.coverAssetId)
        val textOnly = encode(draft(listOf(message("friend", "provider-text", "Text only"))))
        assertTrue(textOnly.isNull("cover_asset_id"))
    }

    @Test fun cardParserUsesCanonicalWireFieldsAndRejectsMalformedIdentities() {
        val value = JSONObject().put("schema", AiConversationShareCodec.SCHEMA).put("snapshot_id", card.id)
            .put("group_id", card.groupId).put("title", card.title).put("summary", card.summary)
            .put("provider", card.provider).put("sender_name", card.senderName).put("message_count", 2)
            .put("cover_asset_id", previewAsset.id)
        assertEquals(card.copy(coverAssetId = previewAsset.id),
            AiConversationShareCodec.parseCard(AiConversationShareCodec.PREFIX + value.toString()))
        assertNull(AiConversationShareCodec.parseCard(value.toString()))
        for ((key, invalid) in listOf("snapshot_id" to "not_a_snapshot", "group_id" to "../group",
            "cover_asset_id" to "https://vendor.invalid/private", "schema" to "unknown", "provider" to "unknown")) {
            val malformed = JSONObject(value.toString()).put(key, invalid)
            assertNull(AiConversationShareCodec.parseCard(AiConversationShareCodec.PREFIX + malformed.toString()))
        }
    }

    private fun imageFixture(): Pair<AiConversationShareDraft, Map<AiConversationShareMediaKey, AiConversationShareAsset>> {
        val image = WebChatProductionContentPart("image", "Selected image", assetHandle = "image_" + "a".repeat(16),
            imageSource = "/app/cache/app-only-image.jpg", imageWidth = 640, imageHeight = 480,
            imageOriginal = WebChatImageOriginal("/c/private-conversation", "private-original.png", "download_" + "b".repeat(32)))
        val user = message("user", "provider-source-user", "Selected **question**", listOf(image)).apply {
            modelUsed = "private-model"
            nodeId = "private-node"
            evidenceDetails = "private-evidence"
            codexThreadUri = "private-codex"
        }
        val assistant = message("friend", "provider-source-assistant", "Selected answer\n").copy(createdAtMs = -1,
            attachments = listOf(ChatAttachment(kind = "image", displayName = "Selected attachment",
                fileName = "app-only-attachment.png", mimeType = "image/png", localPath = "/app/cache/app-only-attachment.png",
                url = "https://vendor.invalid/private?token=private-token")))
        return draft(listOf(user, assistant), setOf(1)) to linkedMapOf(
            AiConversationShareMediaKey(user.id!!, AiConversationShareMediaKey.Location.CONTENT_PART, 0) to previewAsset,
            AiConversationShareMediaKey(assistant.id!!, AiConversationShareMediaKey.Location.ATTACHMENT, 0) to attachmentAsset)
    }

    private fun message(role: String, id: String, body: String, parts: List<WebChatProductionContentPart> = emptyList()) =
        ChatMessage(role, body, id = "chatgpt_web:$id", createdAtMs = 123,
            webChatMessage = WebChatProductionMessage("chatgpt_web", "provider-message-context",
                setOf(WebChatMessageAction.COPY, WebChatMessageAction.MORE), true, parts))

    private fun draft(messages: List<ChatMessage>, gaps: Set<Int> = emptySet()) =
        AiConversationShareDraft("chatgpt", "Draft title", "Draft summary", messages, gaps)

    private fun encode(draft: AiConversationShareDraft, assets: Map<AiConversationShareMediaKey, AiConversationShareAsset> = emptyMap()) =
        AiConversationShareCodec.document(draft, "Selected title", "Selected summary", assets, true)

    private fun envelope(document: JSONObject) = JSONObject().put("snapshot_id", card.id).put("group_id", card.groupId)
        .put("owner_id", "owner_selected").put("owner_name", "Snapshot owner").put("document", document)

    private fun keys(value: JSONObject): Set<String> = value.keys().asSequence().toSet()
}
