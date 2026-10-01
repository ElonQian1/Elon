package com.elon.app.scan

import java.net.URI
import java.net.URLEncoder

internal data class ScanPayload(val raw: String, val kind: String, val title: String, val target: String = "")

internal object ScanPayloadParser {
    private val account = Regex("(?:[a-f0-9]{32}|[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}|usr_[a-z0-9_-]{1,96})", RegexOption.IGNORE_CASE)
    private val unsafe = Regex("[\\u0000-\\u001f\\u007f\\u202a-\\u202e\\u2066-\\u2069]")
    fun friendQr(id: String): String { require(account.matches(id)); return "yilong://friend/v1/$id" }
    fun parse(value: String): ScanPayload {
        val raw = value.trim()
        require(raw.isNotEmpty()) { "二维码内容为空" }
        require(raw.length <= 8192) { "二维码内容过长" }
        val friend = Regex("yilong://friend/v1/([^/?#]+)", RegexOption.IGNORE_CASE).matchEntire(raw)?.groupValues?.get(1)
        if (account.matches(raw)) return ScanPayload(raw, "friend", "一龙账号", raw)
        if (friend != null && account.matches(friend)) return ScanPayload(raw, "friend", "一龙账号", friend)
        if (raw.startsWith("WIFI:", true)) return ScanPayload(raw, "wifi", "Wi-Fi 信息")
        if (Regex("^BEGIN:VCARD\\r?\\n", RegexOption.IGNORE_CASE).containsMatchIn(raw) || raw.startsWith("MECARD:", true)) return ScanPayload(raw, "contact", "联系人")
        if (!unsafe.containsMatchIn(raw) && !raw.contains('\\')) {
            if (Regex("https?://.*", RegexOption.IGNORE_CASE).matches(raw) && !raw.any(Char::isWhitespace)) {
                val uri = runCatching { URI(raw) }.getOrNull()
                if (!uri?.host.isNullOrEmpty() && uri?.userInfo == null) return ScanPayload(raw, "url", "网页链接", raw)
            }
            if (Regex("tel:\\+?[\\d ()-]{3,40}", RegexOption.IGNORE_CASE).matches(raw)) return ScanPayload(raw, "phone", "电话号码", "tel:" + raw.substring(4).replace(Regex("[ ()-]"), ""))
            if (Regex("(?:sms|smsto):\\+?[\\d ()-]{3,40}(?::[^\\r\\n]*)?", RegexOption.IGNORE_CASE).matches(raw)) {
                val parts = raw.substringAfter(':').split(':', limit = 2)
                val target = "sms:" + parts[0].replace(Regex("[ ()-]"), "") + if (parts.size > 1) "?body=" + URLEncoder.encode(parts[1], "UTF-8").replace("+", "%20") else ""
                return ScanPayload(raw, "sms", "短信", target)
            }
            if (Regex("mailto:[^\\s@?]+@[^\\s@?]+", RegexOption.IGNORE_CASE).matches(raw)) return ScanPayload(raw, "email", "电子邮件", raw)
            if (Regex("geo:-?\\d+(?:\\.\\d+)?,-?\\d+(?:\\.\\d+)?", RegexOption.IGNORE_CASE).matches(raw)) {
                val point = raw.substring(4).split(',').map(String::toDouble)
                if (kotlin.math.abs(point[0]) <= 90 && kotlin.math.abs(point[1]) <= 180) return ScanPayload(raw, "geo", "地理位置", raw)
            }
        }
        return ScanPayload(raw, "text", "文本")
    }
}
