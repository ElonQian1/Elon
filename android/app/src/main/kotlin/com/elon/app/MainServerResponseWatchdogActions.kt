package com.elon.app

import com.elon.app.databinding.ActivityMainBinding

internal class MainServerResponseWatchdogActions(
    private val binding: ActivityMainBinding,
    private val taskResponseTokens: MutableMap<String, Int>,
    private val taskForTrace: (String) -> ConversationTaskState?,
    private val activeConversationTask: () -> ConversationTaskState?,
    private val getCurrentStage: () -> String,
    private val getActiveRequestIsDevelopment: () -> Boolean,
    private val refreshActiveTaskState: () -> Unit,
    private val updateStage: (String, String) -> Unit,
    private val addProjectEvent: (String) -> Unit,
    private val startTaskWorkService: (String, String?, Boolean, String?) -> Boolean
) {
    fun scheduleFirstServerResponseWatchdog(traceId: String, token: Int) {
        binding.root.postDelayed({
            if (taskResponseTokens[traceId] != token) return@postDelayed
            val task = taskForTrace(traceId) ?: return@postDelayed
            task.pendingReconnect = true
            refreshActiveTaskState()
            if (task.isDevelopment && activeConversationTask()?.traceId == traceId) {
                updateStage(getCurrentStage(), "暂时没有收到服务器进度，正在自动恢复连接。")
                addProjectEvent("服务端暂未返回进度，自动恢复连接")
            }
            startTaskWorkService(
                TaskWorkService.ACTION_RESUME_PENDING,
                null,
                getActiveRequestIsDevelopment(),
                traceId
            )
        }, 20_000L)
    }
}
