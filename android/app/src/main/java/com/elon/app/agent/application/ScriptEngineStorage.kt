package com.elon.app.agent.application

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import com.elon.app.agent.AgentService
import com.elon.app.agent.application.executor.*
import com.elon.app.agent.domain.execution.ExecutionConfig
import com.elon.app.agent.domain.execution.ExecutionMode
import com.elon.app.agent.domain.execution.ExecutionState
import com.elon.app.agent.domain.execution.ExecutionStateManager
import com.elon.app.agent.domain.screen.ScreenCaptureMode
import com.elon.app.agent.domain.script.*
import com.elon.app.agent.infrastructure.ai.AIClientFactory
import com.elon.app.agent.infrastructure.debug.DebugInterface
import com.elon.app.agent.infrastructure.popup.PopupDismisser
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.*
import java.io.File
import java.util.UUID

// 本地常量(与companion object保持一致)
private const val TAG = "ScriptEngine"
private const val SCRIPTS_DIR = "scripts"
private const val MAX_IMPROVE_ATTEMPTS = 3

// ===== [ScriptEngineStorage.kt] =====
// ========== 脚本存储 ==========

internal fun ScriptEngine.saveScript(script: Script) {
    scriptsCache[script.id] = script
    
    try {
        val scriptsDir = File(service.filesDir, SCRIPTS_DIR)
        if (!scriptsDir.exists()) scriptsDir.mkdirs()
        
        val file = File(scriptsDir, "${script.id}.json")
        file.writeText(gson.toJson(script))
        log("💾 脚本已保存: ${script.name}")
    } catch (e: Exception) {
        Log.e(TAG, "Failed to save script", e)
    }
}

internal fun ScriptEngine.loadScript(scriptId: String): Script? {
    scriptsCache[scriptId]?.let { return it }
    
    try {
        val file = File(service.filesDir, "$SCRIPTS_DIR/$scriptId.json")
        if (file.exists()) {
            val script = gson.fromJson(file.readText(), Script::class.java)
            scriptsCache[scriptId] = script
            return script
        }
    } catch (e: Exception) {
        Log.e(TAG, "Failed to load script", e)
    }
    
    return null
}

internal fun ScriptEngine.listScripts(): List<Script> {
    val scripts = mutableListOf<Script>()
    
    try {
        val scriptsDir = File(service.filesDir, SCRIPTS_DIR)
        if (scriptsDir.exists()) {
            scriptsDir.listFiles()?.forEach { file ->
                if (file.extension == "json") {
                    try {
                        val script = gson.fromJson(file.readText(), Script::class.java)
                        scripts.add(script)
                    } catch (e: Exception) {
                        Log.e(TAG, "Failed to parse script: ${file.name}", e)
                    }
                }
            }
        }
    } catch (e: Exception) {
        Log.e(TAG, "Failed to list scripts", e)
    }
    
    return scripts
}

internal fun ScriptEngine.deleteScript(scriptId: String): Boolean {
    scriptsCache.remove(scriptId)
    
    try {
        val file = File(service.filesDir, "$SCRIPTS_DIR/$scriptId.json")
        return file.delete()
    } catch (e: Exception) {
        return false
    }
}
