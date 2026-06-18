package com.elon.chatvoice

import android.Manifest
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.provider.Settings
import android.speech.RecognitionService
import android.util.Log

data class ChatVoiceRecognitionEngine(
    val component: ComponentName?,
    val packageName: String,
    val label: String,
) {
    fun key(): String = component?.flattenToShortString() ?: SYSTEM_DEFAULT_KEY

    companion object {
        const val SYSTEM_DEFAULT_KEY = "<system-default>"
    }
}

internal object ChatVoiceRecognitionEngineSelector {
    private const val TAG = "ChatVoiceEngineSelector"

    private val packagePriority = mapOf(
        "com.xiaomi.aiasst.vision" to 200,
        "com.miui.voiceassist" to 190,
        "com.xiaomi.mibrain.speech" to 180,
        "com.google.android.googlequicksearchbox" to 100,
        "com.google.android.tts" to 90,
        "com.huawei.hiai" to 70,
        "com.hihonor.magicvoice" to 65,
        "com.iflytek.speechcloud" to 40,
        "com.baidu.input" to 35,
    )

    fun listForUse(context: Context): List<ChatVoiceRecognitionEngine> {
        val appContext = context.applicationContext
        val base = list(appContext)
        val topKey = base.firstOrNull()?.key()
        return base.sortedWith(
            compareBy(
                { engine ->
                    val health = ChatVoiceEngineHealthStore.get(appContext, engine.key())
                    if (engine.key() == topKey && health == ChatVoiceEngineHealth.FAILED) {
                        1
                    } else {
                        when (health) {
                            ChatVoiceEngineHealth.OK -> 0
                            ChatVoiceEngineHealth.UNKNOWN -> 1
                            ChatVoiceEngineHealth.FAILED -> 2
                        }
                    }
                },
                { engine -> -(packagePriority[engine.packageName] ?: 0) },
            )
        )
    }

    private fun list(context: Context): List<ChatVoiceRecognitionEngine> {
        val pm = context.packageManager
        val services = queryRecognitionServices(pm)
        val engines = services
            .mapNotNull { serviceInfoToEngine(pm, it.serviceInfo) }
            .distinctBy { it.key() }
            .sortedByDescending { packagePriority[it.packageName] ?: 0 }
            .toMutableList()
        val result = ArrayList<ChatVoiceRecognitionEngine>()
        val systemDefault = resolveSystemDefault(context, pm)
        if (systemDefault != null) {
            val idx = engines.indexOfFirst { it.component == systemDefault }
            if (idx >= 0) {
                val picked = engines.removeAt(idx)
                result += picked.copy(label = "${picked.label}(系统默认)")
                result += engines
            } else {
                result += engines
            }
        } else {
            result += ChatVoiceRecognitionEngine(null, ChatVoiceRecognitionEngine.SYSTEM_DEFAULT_KEY, "系统默认")
            result += engines
        }
        Log.i(TAG, "候选引擎(${result.size}): ${result.joinToString { "${it.label}(${it.packageName})" }}")
        return result
    }

    private fun queryRecognitionServices(pm: PackageManager) = try {
        @Suppress("DEPRECATION")
        pm.queryIntentServices(Intent(RecognitionService.SERVICE_INTERFACE), 0)
    } catch (error: Throwable) {
        Log.w(TAG, "queryIntentServices failed: ${error.message}")
        emptyList()
    }

    private fun serviceInfoToEngine(
        pm: PackageManager,
        serviceInfo: ServiceInfo?,
    ): ChatVoiceRecognitionEngine? {
        val info = serviceInfo ?: return null
        val pkg = info.packageName ?: return null
        val cls = info.name ?: return null
        if (!isUsable(pm, pkg)) return null
        val label = try {
            info.loadLabel(pm).toString()
        } catch (_: Throwable) {
            pkg
        }
        return ChatVoiceRecognitionEngine(ComponentName(pkg, cls), pkg, label)
    }

    private fun isUsable(pm: PackageManager, pkg: String): Boolean {
        return try {
            val info = pm.getApplicationInfo(pkg, 0)
            if (!info.enabled) return false
            val isSystemApp = (info.flags and ApplicationInfo.FLAG_SYSTEM) != 0
            isSystemApp || pm.checkPermission(Manifest.permission.RECORD_AUDIO, pkg) == PackageManager.PERMISSION_GRANTED
        } catch (_: Throwable) {
            false
        }
    }

    private fun resolveSystemDefault(context: Context, pm: PackageManager): ComponentName? {
        return try {
            val raw = Settings.Secure.getString(context.contentResolver, "voice_recognition_service") ?: return null
            val component = ComponentName.unflattenFromString(raw) ?: return null
            if (isUsable(pm, component.packageName)) component else null
        } catch (_: Throwable) {
            null
        }
    }
}
