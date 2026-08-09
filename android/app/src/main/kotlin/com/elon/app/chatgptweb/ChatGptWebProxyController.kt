package com.elon.app.chatgptweb

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.core.content.ContextCompat
import androidx.webkit.ProxyConfig
import androidx.webkit.ProxyController
import androidx.webkit.WebViewFeature
import java.net.URI

internal data class ChatGptWebProxyStatus(
    val label: String,
    val manualEndpoint: String? = null,
    val error: String? = null,
)

internal class ChatGptWebProxyController(context: Context) {
    private val appContext = context.applicationContext
    private val preferences = appContext.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
    private val connectivityManager =
        appContext.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    private val mainExecutor = ContextCompat.getMainExecutor(appContext)

    fun prepare(onReady: (ChatGptWebProxyStatus) -> Unit) {
        val manualEndpoint = savedManualEndpoint()
        if (manualEndpoint != null) {
            applyManualEndpoint(manualEndpoint, persist = false, onReady = onReady)
        } else {
            clearOverride(onReady)
        }
    }

    fun setManualProxy(
        rawEndpoint: String,
        onReady: (ChatGptWebProxyStatus) -> Unit,
    ): String? {
        val endpoint = normalizeEndpoint(rawEndpoint)
            ?: return "请输入有效的 HTTP 代理，例如 192.168.1.2:7890"
        if (!supportsOverride()) return "当前 Android WebView 不支持独立代理配置"
        applyManualEndpoint(endpoint, persist = true, onReady = onReady)
        return null
    }

    fun useSystemNetwork(onReady: (ChatGptWebProxyStatus) -> Unit) {
        clearOverride { status ->
            if (status.error == null) preferences.edit().remove(KEY_MANUAL_ENDPOINT).apply()
            onReady(status)
        }
    }

    fun currentStatus(): ChatGptWebProxyStatus =
        savedManualEndpoint()?.let { ChatGptWebProxyStatus("手动代理 ${displayEndpoint(it)}", it) }
            ?: systemNetworkStatus()

    fun savedManualEndpoint(): String? =
        preferences.getString(KEY_MANUAL_ENDPOINT, null)?.takeIf { normalizeEndpoint(it) == it }

    private fun applyManualEndpoint(
        endpoint: String,
        persist: Boolean,
        onReady: (ChatGptWebProxyStatus) -> Unit,
    ) {
        if (!supportsOverride()) {
            onReady(
                systemNetworkStatus().copy(
                    error = "当前 Android WebView 不支持已保存的独立代理，已改用手机网络",
                ),
            )
            return
        }
        try {
            val config = ProxyConfig.Builder().addProxyRule(toProxyRule(endpoint)).build()
            ProxyController.getInstance().setProxyOverride(config, mainExecutor) {
                if (persist) preferences.edit().putString(KEY_MANUAL_ENDPOINT, endpoint).apply()
                onReady(ChatGptWebProxyStatus("手动代理 ${displayEndpoint(endpoint)}", endpoint))
            }
        } catch (error: RuntimeException) {
            onReady(systemNetworkStatus().copy(error = "代理应用失败：${error.message ?: "配置无效"}"))
        }
    }

    private fun clearOverride(onReady: (ChatGptWebProxyStatus) -> Unit) {
        if (!supportsOverride()) {
            onReady(systemNetworkStatus())
            return
        }
        try {
            ProxyController.getInstance().clearProxyOverride(mainExecutor) {
                onReady(systemNetworkStatus())
            }
        } catch (error: RuntimeException) {
            onReady(systemNetworkStatus().copy(error = "恢复手机网络失败：${error.message ?: "未知错误"}"))
        }
    }

    private fun systemNetworkStatus(): ChatGptWebProxyStatus {
        val proxy = connectivityManager.defaultProxy
        if (proxy != null) {
            val host = proxy.host.orEmpty()
            if (host.isNotBlank() && proxy.port in 1..65535) {
                return ChatGptWebProxyStatus("系统代理 $host:${proxy.port}")
            }
            if (proxy.pacFileUrl.toString().isNotBlank()) {
                return ChatGptWebProxyStatus("系统 PAC 代理")
            }
        }
        val capabilities = connectivityManager.activeNetwork?.let(connectivityManager::getNetworkCapabilities)
        return if (capabilities?.hasTransport(NetworkCapabilities.TRANSPORT_VPN) == true) {
            ChatGptWebProxyStatus("手机 VPN")
        } else {
            ChatGptWebProxyStatus("手机直连")
        }
    }

    private fun supportsOverride(): Boolean =
        WebViewFeature.isFeatureSupported(WebViewFeature.PROXY_OVERRIDE)

    internal companion object {
        private const val PREFERENCES_NAME = "chatgpt_web_proxy"
        private const val KEY_MANUAL_ENDPOINT = "manual_http_proxy"

        fun normalizeEndpoint(rawEndpoint: String): String? {
            val raw = rawEndpoint.trim()
            if (raw.isBlank() || raw.any(Char::isWhitespace)) return null
            val candidate = if (raw.contains("://")) raw else "http://$raw"
            val uri = runCatching { URI(candidate) }.getOrNull() ?: return null
            if (!uri.scheme.equals("http", ignoreCase = true)) return null
            if (uri.userInfo != null || uri.rawQuery != null || uri.rawFragment != null) return null
            if (uri.rawPath?.let { it.isNotBlank() && it != "/" } == true) return null
            val host = uri.host?.trim()?.takeIf(String::isNotBlank) ?: return null
            if (uri.port !in 1..65535) return null
            val bareHost = host.removeSurrounding("[", "]")
            val normalizedHost = if (bareHost.contains(':')) "[$bareHost]" else bareHost.lowercase()
            return "http://$normalizedHost:${uri.port}"
        }

        fun displayEndpoint(endpoint: String): String = endpoint.removePrefix("http://")

        fun toProxyRule(endpoint: String): String = endpoint.removePrefix("http://")
    }
}
