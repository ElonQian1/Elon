package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import okhttp3.Call
import okhttp3.Callback
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit

/** 从服务器 `/api/voice/tts/catalog` 获取并缓存女声目录，带 5 分钟 TTL。 */
internal data class TtsCatalogResult(
    /** 服务器返回的声线列表；若请求失败则等于 [VoiceTtsVoiceCatalog.presetVoices]。 */
    val voices: List<VoiceTtsVoiceOption>,
    /** 服务器是否已配置 TTS Worker（若为 false，选任何服务器声线都会静默回退到系统 TTS）。 */
    val workerConfigured: Boolean,
    /** 服务器使用的默认 TTS 引擎名称（如 "index_tts2"、"cosyvoice3"）。 */
    val defaultProvider: String,
    /** 此结果是否来自本地缓存/回退（true = 未能联系服务器）。 */
    val isFallback: Boolean = false,
)

internal object VoiceTtsCatalogFetcher {
    private const val TAG = "VoiceTtsCatalog"
    private const val CACHE_TTL_MS = 5 * 60 * 1000L

    private val http = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    @Volatile private var lastFetchMs = 0L
    @Volatile private var cachedResult: TtsCatalogResult? = null
    @Volatile private var activeCall: Call? = null

    /** 返回上次成功缓存的结果，若从未获取过则返回 null。 */
    fun getCachedOrNull(): TtsCatalogResult? = cachedResult

    /**
     * 若缓存超时则向服务器重新拉取；否则直接回调缓存。
     * [onResult] 可能在任意线程被回调，调用方需自行切换到主线程。
     */
    fun fetchIfStale(context: Context, onResult: (TtsCatalogResult) -> Unit) {
        val cached = cachedResult
        val now = System.currentTimeMillis()
        if (cached != null && now - lastFetchMs < CACHE_TTL_MS) {
            onResult(cached)
            return
        }
        fetch(context, onResult)
    }

    /** 无论缓存是否新鲜都强制重新拉取。 */
    fun fetch(context: Context, onResult: (TtsCatalogResult) -> Unit) {
        activeCall?.cancel()
        val url = try {
            ServerUrlManager.getActive(context).trimEnd('/') + "/api/voice/tts/catalog"
        } catch (e: Exception) {
            Log.w(TAG, "ServerUrlManager 异常: ${e.message}")
            onResult(fallback())
            return
        }
        val request = try {
            AuthManager.applyAuth(context, Request.Builder().url(url).get()).build()
        } catch (e: Exception) {
            Log.w(TAG, "构建请求失败: ${e.message}")
            onResult(fallback())
            return
        }
        val call = http.newCall(request)
        activeCall = call
        call.enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) {
                Log.w(TAG, "catalog 拉取失败: ${e.message}")
                onResult(fallback())
            }

            override fun onResponse(call: Call, response: Response) {
                response.use { resp ->
                    if (!resp.isSuccessful) {
                        Log.w(TAG, "catalog 返回 HTTP ${resp.code}")
                        onResult(fallback())
                        return
                    }
                    val body = resp.body?.string() ?: run {
                        onResult(fallback())
                        return
                    }
                    try {
                        val result = parse(body)
                        cachedResult = result
                        lastFetchMs = System.currentTimeMillis()
                        onResult(result)
                    } catch (e: Exception) {
                        Log.w(TAG, "catalog 解析失败: ${e.message}")
                        onResult(fallback())
                    }
                }
            }
        })
    }

    private fun parse(body: String): TtsCatalogResult {
        val json = JSONObject(body)
        val workerConfigured = json.optBoolean("workerConfigured", false)
        val defaultProvider = json.optString("defaultProvider", "auto")
        val arr = json.optJSONArray("voices")
        val voices = mutableListOf<VoiceTtsVoiceOption>()
        if (arr != null) {
            for (i in 0 until arr.length()) {
                val v = arr.optJSONObject(i) ?: continue
                val id = v.optString("id").trim()
                val label = v.optString("label").trim()
                val desc = v.optString("description").trim()
                if (id.isNotEmpty() && label.isNotEmpty()) {
                    voices += VoiceTtsVoiceOption(
                        id = id,
                        displayName = label,
                        description = desc.ifEmpty { label },
                        usesServerTts = true,
                    )
                }
            }
        }
        return TtsCatalogResult(
            voices = voices.ifEmpty { VoiceTtsVoiceCatalog.presetVoices },
            workerConfigured = workerConfigured,
            defaultProvider = defaultProvider,
            isFallback = false,
        )
    }

    private fun fallback(): TtsCatalogResult {
        val cached = cachedResult
        return if (cached != null) {
            cached.copy(isFallback = true)
        } else {
            TtsCatalogResult(
                voices = VoiceTtsVoiceCatalog.presetVoices,
                workerConfigured = false,
                defaultProvider = "auto",
                isFallback = true,
            )
        }
    }

    /** 供测试或离线模式强制注入结果。 */
    fun injectForTest(result: TtsCatalogResult) {
        cachedResult = result
        lastFetchMs = System.currentTimeMillis()
    }
}
