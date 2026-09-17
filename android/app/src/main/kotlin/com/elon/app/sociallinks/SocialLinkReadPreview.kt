package com.elon.app.sociallinks

import android.webkit.WebView
import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import org.json.JSONObject

/** Bounded local metadata observed from the article the user actually opened. No body/cookies. */
internal object SocialLinkReadPreview {
    private data class Entry(val expires: Long, val item: SocialLink)
    private val entries = object : LinkedHashMap<String, Entry>(128, .75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Entry>?) = size > 128
    }
    private fun key(server: String, owner: String?, url: String) = "$server\n$owner\n$url"
    @Synchronized fun cached(server: String, owner: String?, url: String, now: Long = System.currentTimeMillis()): SocialLink? {
        val entry = entries[key(server, owner, url)] ?: return null
        return entry.item.takeIf { entry.expires > now }
    }
    @Synchronized fun remember(server: String, owner: String?, item: SocialLink, now: Long = System.currentTimeMillis()) {
        entries[key(server, owner, item.url)] = Entry(now + 24 * 3600000L, item)
    }
    fun cached(context: Context, server: String, owner: String?, url: String): SocialLink? =
        SocialLinkReadStore.get(context, server, owner, url)
    fun parse(original: String, value: JSONObject): SocialLink? {
        val item = SocialLinkPolicy.link(original) ?: return null
        val expected = SocialLinkReadIdentity.identity(original) ?: return null
        if (SocialLinkReadIdentity.identity(value.optString("url")) != expected || !value.optBoolean("article")) return null
        if (value.optInt("schema", 1) != 1 || value.optString("original", original) != original) return null
        val title = value.optString("title").trim().take(160)
        if (SocialLinkShareText.isGeneric(title, item.site)) return null
        if (Regex("^(?:Log in|Sign in|Access Denied|Just a moment|Page not found|Something went wrong|内容已删除)", RegexOption.IGNORE_CASE).containsMatchIn(title)) return null
        val image = SocialLinkReadIdentity.image(value.optString("image"), expected.substringBefore(':'))
        return item.copy(title = title, author = value.optString("author").trim().take(80), image = image, ready = true, summary = value.optString("description").trim().take(300))
    }
    /** Only the fields the adapter contract defines are forwarded to the server. */
    fun reportable(value: JSONObject): JSONObject = JSONObject().put("schema", 1).put("original", value.optString("original"))
        .put("url", value.optString("url")).put("article", true).put("title", value.optString("title")).put("author", value.optString("author"))
        .put("description", value.optString("description")).put("image", value.optString("image").ifBlank { null })
    fun capture(web: WebView, original: String, server: String, owner: String?, complete: () -> Unit = {}) {
        if (SocialLinkReadIdentity.identity(original) == null) { complete(); return }
        val script = web.context.assets.open("social_link_read_adapter.js").bufferedReader().use { it.readText() }
        web.evaluateJavascript("$script.read(${JSONObject.quote(original)})") { encoded ->
            val app = web.context.applicationContext
            if (AuthManager.userId(app) != owner || ServerUrlManager.getActive(app) != server) { complete(); return@evaluateJavascript }
            runCatching {
                if (encoded.length > 16384) return@runCatching
                val value = JSONObject(encoded)
                if (SocialLinkReadIdentity.identity(web.url.orEmpty()) != SocialLinkReadIdentity.identity(original)) return@runCatching
                parse(original, value)?.let {
                    remember(server, owner, it)
                    SocialLinkReadStore.put(app, server, owner, value)
                    val read = reportable(value)
                    SocialLinkPreviewApi.loader.execute { SocialLinkPreviewApi.report(app, server, original, read) }
                }
            }
            complete()
        }
    }
}
