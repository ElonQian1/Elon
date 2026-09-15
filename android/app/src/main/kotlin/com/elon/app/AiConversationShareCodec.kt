package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal object AiConversationShareCodec {
    const val SCHEMA = "elon.ai_conversation_share.v1"
    const val PREFIX = "【一龙AI对话】\n"
    private val opaque = Regex("[A-Za-z0-9_.-]{1,160}")

    fun parseCard(content: String): AiConversationShareCard? = runCatching {
        if (!content.startsWith(PREFIX) || content.length > 10_000) return null
        val value = JSONObject(content.removePrefix(PREFIX))
        require(value.getString("schema") == SCHEMA)
        val id = value.getString("snapshot_id").also { require(it.startsWith("ai_snapshot_") && opaque.matches(it)) }
        val group = value.getString("group_id").also { require(opaque.matches(it) && it != "." && it != "..") }
        AiConversationShareCard(id, group, value.text("title", 120), value.text("summary", 300),
            value.getString("provider").also { require(it == "chatgpt") },
            value.optString("sender_name", ""), value.getInt("message_count").also { require(it in 1..200) },
            value.optional("cover_asset_id")?.also { require(assetId(it)) })
    }.getOrNull()

    fun document(draft: AiConversationShareDraft, title: String, summary: String,
        assets: Map<AiConversationShareMediaKey, AiConversationShareAsset>, cover: Boolean): JSONObject {
        require(draft.provider == "chatgpt") { "当前支持分享 ChatGPT 会话" }
        require(title.isNotBlank() && title.length <= 120 && summary.length <= 300)
        var blockIndex = 0
        val rows = draft.messages.mapIndexed { index, message ->
            val parts = JSONArray()
            message.webChatMessage?.contentParts.orEmpty().forEachIndexed { partIndex, part ->
                val key = AiConversationShareMediaKey(message.id!!, AiConversationShareMediaKey.Location.CONTENT_PART, partIndex)
                val next = when {
                    part.type == "image" -> imagePart(part.label, assets[key] ?: error("图片未准备好"))
                    part.textBlock != null -> {
                        val block = part.textBlock
                        require(block.complete) { "写作块尚未完成" }
                        JSONObject().put("type", if (block.kind == "code") "code" else "writing_block")
                            .put("label", part.label).put("text_block", JSONObject()
                                .put("version", 1).put("id", "block_${blockIndex++}").put("kind", block.kind)
                                .put("title", block.title).put("language", block.language)
                                .put("content", block.content).put("complete", true))
                    }
                    part.richCard != null -> JSONObject().put("type", "rich_card").put("label", part.label)
                        .put("card", AiConversationShareRichCardCodec.encode(part.richCard))
                    part.type in setOf("code", "table", "math", "citation") && message.content.isNotBlank() -> null
                    else -> error("所选消息包含尚不支持分享的内容：${part.label.take(40)}")
                }
                if (next != null) parts.put(next)
            }
            message.attachments.orEmpty().forEachIndexed { attachmentIndex, attachment ->
                require(attachment.isImage()) { "所选文件暂不支持合并分享" }
                val key = AiConversationShareMediaKey(message.id!!, AiConversationShareMediaKey.Location.ATTACHMENT, attachmentIndex)
                parts.put(imagePart(attachment.displayName ?: "图片", assets[key] ?: error("图片未准备好")))
            }
            JSONObject().put("id", "message_$index").put("role", if (message.role == "user") "user" else "assistant")
                .put("content", message.content).put("created_at_ms", message.createdAtMs.coerceAtLeast(0))
                .put("gap_before", index in draft.gaps).put("parts", parts)
        }
        return JSONObject().put("schema", SCHEMA).put("provider", draft.provider).put("title", title)
            .put("summary", summary).put("messages", JSONArray(rows))
            .put("cover_asset_id", if (cover) assets.values.firstOrNull()?.id ?: JSONObject.NULL else JSONObject.NULL)
            .also { require(it.toString().toByteArray(Charsets.UTF_8).size <= 2 * 1024 * 1024) { "分享内容超过大小限制" } }
    }

    fun snapshot(value: JSONObject, originalCard: AiConversationShareCard): AiConversationShareSnapshot {
        require(value.getString("snapshot_id") == originalCard.id && value.getString("group_id") == originalCard.groupId)
        val doc = value.getJSONObject("document")
        require(doc.getString("schema") == SCHEMA && doc.getString("provider") == "chatgpt")
        val rows = doc.getJSONArray("messages")
        require(rows.length() in 1..200)
        val ownerName = value.optString("owner_name", originalCard.senderName).ifBlank { "分享者" }
        val ids = hashSetOf<String>()
        val gaps = hashSetOf<Int>()
        val messages = List(rows.length()) { index ->
            val row = rows.getJSONObject(index)
            val id = row.getString("id").also { require(opaque.matches(it) && ids.add(it)) }
            val role = row.getString("role").also { require(it == "user" || it == "assistant") }
            if (row.optBoolean("gap_before") && index > 0) gaps.add(index)
            val parts = row.getJSONArray("parts").also { require(it.length() <= 256) }
            val nativeParts = List(parts.length()) { decodePart(parts.getJSONObject(it)) }
            ChatMessage(role = if (role == "user") "user" else "friend", content = row.text("content", 120_000),
                id = id, senderLabel = if (role == "user") ownerName else "ChatGPT",
                senderAvatarResId = if (role == "assistant") WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB).avatarResId else null,
                createdAtMs = row.getLong("created_at_ms"),
                webChatMessage = WebChatProductionMessage("shared_chatgpt", id, emptySet(), true, nativeParts))
        }
        val card = originalCard.copy(title = doc.text("title", 120), summary = doc.text("summary", 300),
            senderName = ownerName, messageCount = messages.size, coverAssetId = doc.optional("cover_asset_id"))
        return AiConversationShareSnapshot(card, messages, ownerId = value.getString("owner_id"), gaps = gaps)
    }

    private fun imagePart(label: String, asset: AiConversationShareAsset) = JSONObject()
        .put("type", "image").put("label", label.take(180)).put("asset_id", asset.id).put("media_type", asset.mimeType)

    private fun decodePart(value: JSONObject): WebChatProductionContentPart {
        val type = value.getString("type")
        val label = value.text("label", 180)
        val block = value.optJSONObject("text_block")?.let {
            WebChatTextBlock.parse(it) ?: error("分享的写作块格式无效")
        }
        val asset = value.optional("asset_id")?.also { require(assetId(it)) }
        require(type in setOf("image", "code", "writing_block", "rich_card", "citation", "table", "math", "unavailable"))
        require(type != "image" || asset != null)
        return WebChatProductionContentPart(type, label, language = value.optional("language"),
            mediaType = value.optional("media_type"), assetHandle = asset,
            previewPending = type == "image", textBlock = block,
            richCard = value.optJSONObject("card")?.let(AiConversationShareRichCardCodec::decode))
    }

    internal fun assetId(id: String) = id.startsWith("article_media_") && opaque.matches(id)
    internal fun JSONObject.optional(key: String) = (opt(key) as? String)?.takeIf(String::isNotBlank)
    private fun JSONObject.text(key: String, max: Int) = getString(key).also { require(it.length <= max) }
}

internal data class AiConversationShareAsset(val id: String, val mimeType: String)
