package com.elon.app

import android.content.Context
import android.util.AtomicFile
import android.util.Base64
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.TimeUnit

internal class AiConversationShareApiException(val status: Int, message: String) : Exception(message) {
    val accessDenied get() = status in setOf(401, 403, 404, 410)
}

/** All methods are blocking IO. Only first-party auth is used; provider credentials never leave WebView. */
internal class AiConversationShareApi(private val context: Context, http: OkHttpClient, private val server: String) {
    private val client = http.newBuilder().followRedirects(false).followSslRedirects(false)
        .callTimeout(25, TimeUnit.SECONDS).build()
    private val authorized = mutableSetOf<String>()
    private var cacheSession: String? = null

    fun groupReplyDraft(groupId: String, messageId: String): AiConversationShareDraft {
        require(ID.matches(messageId) && messageId.startsWith("gai"))
        val groupBase = groupPath(groupId).removeSuffix("/ai-snapshots")
        val value = json(Request.Builder().url("$groupBase/messages/$messageId/ai-context").get(),
            2 * 1024 * 1024, socialSession(context))
        val card = AiConversationShareCard("preview", groupId, "群聊 AI 精选讨论", "", "chatgpt", "群聊成员", 1)
        val snapshot = AiConversationShareCodec.snapshot(value, card)
        return AiConversationShareDraft("chatgpt", snapshot.card.title,
            AiConversationShareDraftBuilder.excerpt(snapshot.messages), snapshot.messages, snapshot.gaps)
    }

    fun read(card: AiConversationShareCard): AiConversationShareSnapshot {
        val session = socialSession(context)
        val result = json(Request.Builder().url(path(card)).get(), 3 * 1024 * 1024, session)
        return AiConversationShareCodec.snapshot(result, card).also {
            synchronized(authorized) {
                if (cacheSession != session) { authorized.clear(); cacheSession = session }
                authorized.add(key(card))
            }
        }
    }

    fun upload(groupId: String, image: AiConversationShareMediaUpload, session: String): AiConversationShareAsset {
        require(image.bytes.size in 1..MAX_ASSET_BYTES)
        val payload = JSONObject().put("base64", Base64.encodeToString(image.bytes, Base64.NO_WRAP))
        val response = json(Request.Builder().url("${groupPath(groupId)}/assets")
            .post(payload.toString().toRequestBody(JSON)), 4096, session)
        val id = response.getString("asset_id").also { require(AiConversationShareCodec.assetId(it)) }
        val mime = response.getString("mime_type").also { require(it in IMAGE_TYPES) }
        return AiConversationShareAsset(id, mime)
    }

    fun publish(groupId: String, document: JSONObject, session: String): AiConversationShareCard {
        require(document.toString().toByteArray(Charsets.UTF_8).size <= 2 * 1024 * 1024)
        val digest = operationDigest(groupId, document)
        val prefs = context.getSharedPreferences("ai_conversation_share_operations", Context.MODE_PRIVATE)
        val id = synchronized(operationLock) {
            prefs.getString(digest, null) ?: UUID.randomUUID().toString().also {
                check(prefs.edit().putString(digest, it).commit()) { "无法保存发送状态，请稍后重试" }
            }
        }
        val payload = JSONObject().put("idempotency_key", id).put("document", document)
        val response = json(Request.Builder().url(groupPath(groupId))
            .post(payload.toString().toRequestBody(JSON)), 24_000, session)
        val message = response.getJSONObject("message")
        val card = AiConversationShareCodec.parseCard(message.getString("content"))
            ?: error("分享回执无效，请重试确认发送结果")
        require(card.groupId == groupId && card.id == response.getString("snapshot_id"))
        return card
    }

    fun acknowledgePublish(groupId: String, document: JSONObject, session: String) {
        ensureSession(session)
        val digest = operationDigest(groupId, document)
        synchronized(operationLock) {
            context.getSharedPreferences("ai_conversation_share_operations", Context.MODE_PRIVATE)
                .edit().remove(digest).commit()
        }
    }

    private fun operationDigest(groupId: String, document: JSONObject): String {
        val owner = AuthManager.userId(context) ?: throw AiConversationShareApiException(401, "请先登录一龙账号")
        return hash("$server|$owner|$groupId|$document")
    }

    fun asset(card: AiConversationShareCard, assetId: String): ByteArray {
        require(AiConversationShareCodec.assetId(assetId))
        return bytes(Request.Builder().url("${path(card)}/assets/$assetId").get(), MAX_ASSET_BYTES,
            socialSession(context), true)
    }

    fun loadImage(card: AiConversationShareCard, assetId: String): String {
        require(AiConversationShareCodec.assetId(assetId))
        val session = socialSession(context)
        val file = File(cacheFolder(card), assetId)
        val allowed = synchronized(authorized) { cacheSession == session && key(card) in authorized }
        if (allowed && file.isFile && file.length() in 1L..MAX_ASSET_BYTES.toLong()) {
            ensureSession(session)
            return file.absolutePath
        }
        val bytes = asset(card, assetId)
        ensureSession(session)
        file.parentFile?.mkdirs()
        val atomic = AtomicFile(file)
        val output = atomic.startWrite()
        try { output.write(bytes); ensureSession(session); atomic.finishWrite(output) }
        catch (failure: Exception) { atomic.failWrite(output); throw failure }
        trimImages(file)
        return file.absolutePath
    }

    fun revoke(card: AiConversationShareCard) {
        json(Request.Builder().url(path(card)).delete(), 4096, socialSession(context))
        invalidate(card)
    }

    fun invalidate(card: AiConversationShareCard) {
        synchronized(authorized) { authorized.remove(key(card)) }
        cacheFolder(card).listFiles()?.forEach { if (it.isFile) it.delete() }
    }

    private fun json(builder: Request.Builder, max: Int, session: String): JSONObject =
        JSONObject(bytes(builder, max, session).toString(Charsets.UTF_8))

    private fun bytes(builder: Request.Builder, max: Int, session: String, image: Boolean = false): ByteArray {
        ensureSession(session)
        client.newCall(AuthManager.applyAuth(context, builder.header("Cache-Control", "no-cache")).build())
            .execute().use { response ->
                ensureSession(session)
                if (!response.isSuccessful) throw AiConversationShareApiException(response.code, when (response.code) {
                    401 -> "请先登录一龙账号"
                    403 -> "你已不在此群聊中，无法查看这份记录"
                    404, 410 -> "这份聊天记录已撤回或无法访问"
                    409 -> "发送记录已变化，请重新打开分享预览"
                    413 -> "所选内容或图片过大，请减少后再分享"
                    400 -> "分享内容格式不支持，请检查所选消息和图片"
                    else -> "服务暂时不可用，请稍后重试"
                })
                val body = response.body ?: error("分享响应为空")
                if (image) require(body.contentType()?.let { "${it.type}/${it.subtype}" } in IMAGE_TYPES)
                require(body.contentLength() <= max) { "分享响应超过大小限制" }
                val result = body.byteStream().use { input ->
                    val output = java.io.ByteArrayOutputStream()
                    val buffer = ByteArray(8192)
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        require(output.size().toLong() + count <= max) { "分享响应超过大小限制" }
                        output.write(buffer, 0, count)
                    }
                    output.toByteArray()
                }
                ensureSession(session)
                return result
            }
    }

    private fun ensureSession(session: String) {
        if (!AuthManager.isLoggedIn(context) || socialSession(context) != session)
            throw AiConversationShareApiException(401, "账号已变化，请重新打开分享")
    }
    private fun groupPath(group: String): String {
        require(ID.matches(group) && group != "." && group != "..")
        return "${server.trimEnd('/')}/api/me/groups/$group/ai-snapshots"
    }
    private fun path(card: AiConversationShareCard): String {
        require(ID.matches(card.id) && card.id.startsWith("ai_snapshot_"))
        return "${groupPath(card.groupId)}/${card.id}"
    }
    private fun key(card: AiConversationShareCard) = "${card.groupId}/${card.id}"
    private fun cacheFolder(card: AiConversationShareCard): File {
        require(card.id.startsWith("ai_snapshot_") && ID.matches(card.id) && ID.matches(card.groupId) &&
            card.groupId != "." && card.groupId != "..")
        val owner = hash("$server|${AuthManager.userId(context)}")
        return File(context.cacheDir, "ai_conversation_share/$owner/${card.groupId}/${card.id}")
    }
    private fun trimImages(keep: File) {
        val files = File(context.cacheDir, "ai_conversation_share").walkTopDown().filter(File::isFile).toList()
        var total = files.sumOf(File::length)
        for (file in files.sortedBy(File::lastModified)) {
            if (total <= 48L * 1024 * 1024) break
            if (file != keep) { val length = file.length(); if (file.delete()) total -= length }
        }
    }
    companion object {
        const val MAX_ASSET_BYTES = 12 * 1024 * 1024
        private val IMAGE_TYPES = setOf("image/png", "image/jpeg", "image/webp")
        private val ID = Regex("[A-Za-z0-9_.-]{1,160}")
        private val JSON = "application/json; charset=utf-8".toMediaType()
        private val operationLock = Any()
        private fun hash(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray())
            .joinToString("") { "%02x".format(it) }
    }
}
