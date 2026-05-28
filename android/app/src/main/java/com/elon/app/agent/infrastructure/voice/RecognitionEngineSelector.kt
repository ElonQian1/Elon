// infrastructure/voice/RecognitionEngineSelector.kt
// module: infrastructure/voice | layer: infrastructure | role: engine-selector
// summary: 枚举手机所有 RecognitionService 引擎，按优先级排序。失败回退由调用方在内存中跟踪。

package com.elon.app.agent.infrastructure.voice

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.provider.Settings
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
 *  2. 按优先级排序：Google > 厂商语音 > 其他；
 *  3. 始终把"系统默认"放在最前面作为兜底选项。
 *
 * **不做持久化**。是否跳过某个引擎完全由调用方在本次会话内自己记忆
 * （[com.elon.app.AgentVoiceBridge] 用 `candidateIndex` 顺序遍历就够了）。
 * 这样下次 APP 启动会重新从首选引擎开始尝试 —— 用户可能在系统设置里换了
 * 引擎、或者重启了网络，没必要永久封禁。
 */
object RecognitionEngineSelector {

    private const val TAG = "AsrEngineSelector"

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
     * 后面跟随能直接指定 ComponentName 的所有可用引擎。
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
        val seen = HashSet<String>()
        val result = ArrayList<RecognitionEngine>()

        // 1. 列出所有可解析的服务，过滤掉无可用权限/已禁用的，按优先级排序
        val engines = resolves.mapNotNull { ri ->
            val si: ServiceInfo = ri.serviceInfo ?: return@mapNotNull null
            val pkg = si.packageName ?: return@mapNotNull null
            val cls = si.name ?: return@mapNotNull null
            val component = ComponentName(pkg, cls)
            val key = component.flattenToShortString()
            if (!seen.add(key)) return@mapNotNull null
            if (!isUsable(pm, pkg)) {
                Log.i(TAG, "过滤无效引擎: $pkg (未启用或无 RECORD_AUDIO)")
                return@mapNotNull null
            }
            val label = try {
                si.loadLabel(pm)?.toString() ?: pkg
            } catch (_: Throwable) {
                pkg
            }
            RecognitionEngine(component = component, packageName = pkg, label = label)
        }.sortedByDescending { PACKAGE_PRIORITY[it.packageName] ?: 0 }

        // 2. 解析系统默认指向的真实 component，避免走 SpeechRecognizer 的间接层（在某些
        //    国产 ROM 上会稳定报 code=11 SERVER_DISCONNECTED）。如果解析成功且该
        //    component 已在 engines 里，就用它作为首选（移到前面）；否则把它作为额外
        //    候选追加。如果完全解析不到，才回退到 component=null 兜底。
        val systemDefault = resolveSystemDefault(context, pm)
        val enginesMut = engines.toMutableList()
        if (systemDefault != null) {
            val idx = enginesMut.indexOfFirst { it.component == systemDefault }
            if (idx >= 0) {
                val picked = enginesMut.removeAt(idx)
                result += picked.copy(label = "${picked.label}(系统默认)")
                result += enginesMut
            } else {
                val label = try { pm.getApplicationLabel(pm.getApplicationInfo(systemDefault.packageName, 0)).toString() } catch (_: Throwable) { systemDefault.packageName }
                result += RecognitionEngine(systemDefault, systemDefault.packageName, "$label(系统默认)")
                result += enginesMut
            }
        } else {
            // 完全没解析到，把 null 兜底放最前
            result += RecognitionEngine(null, "<system-default>", "系统默认")
            result += enginesMut
        }

        Log.i(TAG, "候选引擎(${result.size}): ${result.joinToString { "${it.label}(${it.packageName})" }}")
        return result
    }

    /**
     * 引擎是否真的可用：
     *  - 包必须 enabled；
     *  - 包必须已被授予 RECORD_AUDIO（很多预装但未被用过的 RecognitionService 没有授权）。
     */
    private fun isUsable(pm: PackageManager, pkg: String): Boolean {
        return try {
            val ai = pm.getApplicationInfo(pkg, 0)
            if (!ai.enabled) return false
            pm.checkPermission(android.Manifest.permission.RECORD_AUDIO, pkg) == PackageManager.PERMISSION_GRANTED
        } catch (_: Throwable) {
            false
        }
    }

    /** 读 Settings.Secure 拿到当前系统默认 ASR 的 ComponentName。 */
    private fun resolveSystemDefault(context: Context, pm: PackageManager): ComponentName? {
        return try {
            val raw = Settings.Secure.getString(context.contentResolver, "voice_recognition_service")
                ?: return null
            val cn = ComponentName.unflattenFromString(raw) ?: return null
            if (!isUsable(pm, cn.packageName)) null else cn
        } catch (_: Throwable) {
            null
        }
    }
}
