package com.elon.app.sociallinks

import android.webkit.WebView
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import org.json.JSONObject
import org.json.JSONTokener

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
    fun parse(original: String, value: JSONObject): SocialLink? {
        val item = SocialLinkPolicy.link(original)?.takeIf { it.site == "微信公众号" } ?: return null
        val source = SocialLinkPolicy.safeUrl(value.optString("url")) ?: return null
        val expected = SocialLinkPolicy.safeUrl(original) ?: return null
        if (source.host != expected.host || source.path != expected.path || (source.path != "/s" && !source.path.startsWith("/s/")) ||
            (source.path == "/s" && source.rawQuery != expected.rawQuery) || !value.optBoolean("article")) return null
        val title = value.optString("title").trim().take(160)
        if (SocialLinkShareText.isGeneric(title, item.site)) return null
        val image = SocialLinkPolicy.safeUrl(value.optString("image"))?.takeIf {
            it.host == "qpic.cn" || it.host.endsWith(".qpic.cn")
        }?.toString()
        return item.copy(title = title, author = value.optString("author").trim().take(80), image = image, ready = true)
    }
    fun capture(web: WebView, original: String, server: String, owner: String?) {
        if (SocialLinkPolicy.link(original)?.site != "微信公众号") return
        web.evaluateJavascript(SCRIPT) { encoded ->
            val app = web.context.applicationContext
            if (AuthManager.userId(app) != owner || ServerUrlManager.getActive(app) != server) return@evaluateJavascript
            runCatching {
                if (encoded.length > 16384) return@runCatching
                val text = JSONTokener(encoded).nextValue() as? String ?: return@runCatching
                parse(original, JSONObject(text))?.let { remember(server, owner, it) }
            }
        }
    }
    private const val SCRIPT = """(function(){
      if(location.hostname!=='mp.weixin.qq.com')return null;
      var heading=document.querySelector('#activity-name'),body=document.querySelector('#js_content');
      if(!heading||!body||!heading.getClientRects().length)return null;
      var author=document.querySelector('#js_name'),cover=document.querySelector('meta[property="og:image"]');
      var image='';try{var u=new URL(cover?cover.content:'',location.href);if(u.protocol==='http:')u.protocol='https:';image=u.href;}catch(e){}
      return JSON.stringify({url:location.href,article:true,title:heading.textContent.trim().slice(0,160),author:author?author.textContent.trim().slice(0,80):'',image:image});
    })()"""
}
