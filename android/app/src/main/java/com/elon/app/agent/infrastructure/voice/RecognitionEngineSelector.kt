// infrastructure/voice/RecognitionEngineSelector.kt
// module: infrastructure/voice | layer: infrastructure | role: engine-selector
// summary: 枚举手机所有 RecognitionService 引擎，按优先级排序并维护失败黑名单。

package com.elon.app.agent.infrastructure.voice

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.speech.RecognitionService
import android.util.Log

/**
 * 一个候选 ASR 引擎。
 *
 * @param component null 表示"使用系统默认（让 createSpeechRecognizer(context) 自己挑）"
 */
data class RecognitionEngine(
    val component: ComponentName?,
    val packageName: String,
    val label: String,
) {
    fun key(): String = component?.flattenToShortString() ?: "<system-default>"
}

/**
 * 引擎选择器：
 *  1. 枚举所有声明 [RecognitionService] 的包；
 *  2. 排除已知坏引擎（运行时被加入黑名单的，会写入 SharedPreferences 持久化）；
 *  3. 按优先级排序：Google > 厂商语音 > 其他；
 *  4. 始终把"系统默认"放在最前面作为兜底选项（除非默认那个已被黑名单）。
 *
 * 用法：
 * ```
 * val candidates = RecognitionEngineSelector.list(context)
 * for (engine in candidates) {
 *     // 设置 StreamingASR.engineComponent = engine.component 后启动
 *     if (success) break
 *     RecognitionEngineSelector.blacklist(context, engine, reason)
 * }
 * ```
 */
object RecognitionEngineSelector {

    private const val TAG = "AsrEngineSelector"
    private const val PREFS = "asr_engine_prefs"
    private const val KEY_BLACKLIST = "engine_blacklist"

    /** 已知优先级越大越优先；只用来做稳定排序。 */
    private val PACKAGE_PRIORITY: Map<String, Int> = mapOf(
        "com.google.android.googlequicksearchbox" to 100,    // Google App / Now
        "com.google.android.tts" to 90,                       // Google TTS（也提供识别）
        "com.huawei.hiai" to 70,                              // 华为 HiAI
        "com.hihonor.magicvoice" to 65,                       // 荣耀 MagicVoice
        "com.xiaomi.aiasst.vision" to 60,                     // 小米
        "com.miui.voiceassist" to 55,
        "com.iflytek.speechcloud" to 40,
        "com.baidu.input" to 35,
    )

    /**
     * 返回排序后的候选引擎列表。
     *
     * 列表头部总是 [RecognitionEngine] with component=null（系统默认）；
     * 后面跟随能直接指定 ComponentName 的所有可用引擎；
     * 全部已被黑名单的引擎会被过滤掉。
     */
    fun list(context: Context): List<RecognitionEngine> {
        val pm = context.packageManager
        val intent = Intent(RecognitionService.SERVICE_INTERFACE)
        val resolves = try {
            @Suppress("DEPRECATION")
            pm.queryIntentServices(intent, 0)
        } catch (t: Throwable) {
            Log.w(TAG, "queryIntentServices 失败: ${t.message}")
            emptyList()
        }
        val blacklist = loadBlacklist(context)
        val seen = HashSet<String>()
        val result = ArrayList<RecognitionEngine>()

        // 1. 系统默认入口（component=null）始终保留 — 它对应当前系统设置里选中的那个，
        //    用户/厂商可能已经把它配成自己可用的版本。
        result += RecognitionEngine(
            component = null,
            packageName = "<system-default>",
            label = "系统默认",
        )

        // 2. 列出所有可解析的服务，按优先级排序
        val engines = resolves.mapNotNull { ri ->
            val si: ServiceInfo = ri.serviceInfo ?: return@mapNotNull null
            val pkg = si.packageName ?: return@mapNotNull null
            val cls = si.name ?: return@mapNotNull null
            val component = ComponentName(pkg, cls)
            val key = component.flattenToShortString()
            if (!seen.add(key)) return@mapNotNull null
            val label = try {
                si.loadLabel(pm)?.toString() ?: pkg
            } catch (_: Throwable) {
                pkg
            }
            RecognitionEngine(component = component, packageName = pkg, label = label)
        }.sortedByDescending { PACKAGE_PRIORITY[it.packageName] ?: 0 }

        for (e in engines) {
            if (blacklist.contains(e.key())) {
                Log.i(TAG, "跳过黑名单引擎: ${e.key()}")
                continue
            }
            result += e
        }

        Log.i(TAG, "候选引擎: ${result.joinToString { "${it.label}(${it.packageName})" }}")
        return result
    }

    /** 把一个引擎写入黑名单（持久化）。系统默认那一项不能被加入黑名单。 */
    fun blacklist(context: Context, engine: RecognitionEngine, reason: String) {
        if (engine.component == null) {
            Log.w(TAG, "拒绝把'系统默认'放入黑名单 ($reason)")
            return
        }
        val key = engine.key()
        val cur = loadBlacklist(context).toMutableSet()
        if (cur.add(key)) {
            context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
                .edit()
                .putStringSet(KEY_BLACKLIST, cur)
                .apply()
            Log.i(TAG, "已加入引擎黑名单: $key ($reason)")
        }
    }

    /** 清空黑名单（用户切回手动模式或重置时调用）。 */
    fun resetBlacklist(context: Context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .remove(KEY_BLACKLIST)
            .apply()
        Log.i(TAG, "引擎黑名单已清空")
    }

    private fun loadBlacklist(context: Context): Set<String> {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getStringSet(KEY_BLACKLIST, emptySet()) ?: emptySet()
    }
}
