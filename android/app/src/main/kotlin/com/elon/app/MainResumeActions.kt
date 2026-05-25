package com.elon.app

import android.content.Intent
import android.content.SharedPreferences
import android.view.View
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding

internal class MainResumeActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val prefs: SharedPreferences,
    private val isBindingInitialized: () -> Boolean,
    private val setAppInForeground: (Boolean) -> Unit,
    private val setTaskAppForeground: (Boolean) -> Unit,
    private val drainQueuedTaskEvents: () -> Unit,
    private val loadModelOptions: () -> Unit,
    private val getBackendConnected: () -> Boolean,
    private val getWaitingForReply: () -> Boolean,
    private val getPendingReconnectForActiveWork: () -> Boolean,
    private val setPendingReconnectForActiveWork: (Boolean) -> Unit,
    private val currentStage: () -> String,
    private val updateStage: (String, String) -> Unit,
    private val recordEvidence: (String, String) -> Unit,
    private val startTaskWorkService: (String) -> Boolean,
    private val isActiveConversationWorking: () -> Boolean,
    private val setSendEnabled: (Boolean) -> Unit,
    private val maybePrewarmCodexSession: (String) -> Unit
) {
    fun onResume() {
        setAppInForeground(true)
        setTaskAppForeground(true)
        startMcpDebugKeepAlive()
        drainQueuedTaskEvents()
        clearCompletedTaskBadge(activity, prefs)
        if (!isBindingInitialized()) return
        loadModelOptions()
        if (!getBackendConnected()) {
            handleDisconnectedResume()
        } else if (!isActiveConversationWorking()) {
            setSendEnabled(true)
            if (binding.chatPage.visibility == View.VISIBLE) {
                maybePrewarmCodexSession("resume_chat")
            }
        }
    }

    private fun handleDisconnectedResume() {
        if (getWaitingForReply() && !getPendingReconnectForActiveWork()) {
            setPendingReconnectForActiveWork(true)
            updateStage(currentStage(), "正在恢复连接，回来后会自动继续本轮任务。")
            recordEvidence("connection", "连接恢复中，正在继续上次任务")
        }
        startTaskWorkService(
            if (getWaitingForReply()) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
        )
    }

    private fun startMcpDebugKeepAlive() {
        if (!McpDebugKeepAliveService.shouldAutoStart(activity)) return
        val intent = Intent(activity, McpDebugKeepAliveService::class.java).apply {
            action = McpDebugKeepAliveService.ACTION_START
        }
        runCatching {
            ContextCompat.startForegroundService(activity, intent)
        }.onSuccess {
            DebugTraceStore.record("mcp_keepalive_auto_start_requested")
        }.onFailure { error ->
            DebugTraceStore.record(
                "mcp_keepalive_auto_start_failed",
                mapOf("error" to (error.message ?: error.javaClass.simpleName))
            )
        }
    }
}
