package com.elon.app

import com.elon.app.databinding.ActivityMainBinding

internal class MainActiveWorkControlActions(
    private val binding: ActivityMainBinding,
    private val activeConversationTask: () -> ConversationTaskState?,
    private val removeConversationTask: (String?, String?, String?) -> ConversationTaskState?,
    private val resetReconnectAttempts: () -> Unit,
    private val incrementReconnectAttempts: () -> Int,
    private val taskForTrace: (String) -> ConversationTaskState?,
    private val isBackendConnected: () -> Boolean,
    private val getActiveRequestIsDevelopment: () -> Boolean,
    private val setActiveRequestIsDevelopment: (Boolean) -> Unit,
    private val getCurrentStage: () -> String,
    private val getPendingRequestPayload: () -> String?,
    private val setPendingReconnectForActiveWork: (Boolean) -> Unit,
    private val setWaitingForReply: (Boolean) -> Unit,
    private val persistActiveWork: () -> Unit,
    private val clearPersistedActiveWork: () -> Unit,
    private val refreshActiveTaskState: () -> Unit,
    private val stopWorkingEvidenceForActiveConversation: () -> Unit,
    private val clearCurrentEvidence: () -> Unit,
    private val setSendEnabled: (Boolean) -> Unit,
    private val updateFirstConversationStatus: (String) -> Unit,
    private val updateStage: (String, String) -> Unit,
    private val updateProjectViews: (String) -> Unit,
    private val addProjectEvent: (String) -> Unit,
    private val recordEvidence: (String, String) -> Unit,
    private val appendMessage: (ChatMessage) -> Unit,
    private val workflowStoppedMessage: (String, Boolean) -> String,
    private val startTaskWorkService: (String, String?, Boolean, String?) -> Boolean,
    private val nextServerResponseToken: () -> Int,
    private val scheduleFirstServerResponseWatchdog: (String, Int) -> Unit
) {
    fun pauseCurrentWork() {
        val task = activeConversationTask() ?: return
        val wasDevelopment = task.isDevelopment
        removeConversationTask(task.traceId, task.projectId, task.conversationId)
        resetReconnectAttempts()
        persistActiveWork()
        stopWorkingEvidenceForActiveConversation()
        clearCurrentEvidence()
        setSendEnabled(true)
        if (wasDevelopment) {
            updateStage("工作暂停", "你已暂停当前任务，可以调整需求后继续发送。")
            addProjectEvent("暂停当前工作")
        } else {
            updateProjectViews("当前回复已暂停，你可以继续输入新的消息。")
        }
        appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("你已暂停当前工作。", wasDevelopment)))
        startTaskWorkService(TaskWorkService.ACTION_PAUSE, null, getActiveRequestIsDevelopment(), task.traceId)
    }

    fun handleActiveWorkDisconnected(task: ConversationTaskState) {
        task.pendingReconnect = true
        refreshActiveTaskState()
        persistActiveWork()
        setSendEnabled(false)
        updateFirstConversationStatus("连接恢复中 · 回来后继续")
        if (getActiveRequestIsDevelopment()) {
            updateStage(getCurrentStage(), "连接暂时断开，正在保留本轮任务并准备自动恢复。")
            recordEvidence("connection", "连接暂时断开，正在自动恢复任务")
        }

        scheduleReconnectForActiveWork(task.traceId)
    }

    fun scheduleReconnectForActiveWork(traceId: String? = activeConversationTask()?.traceId) {
        val taskTraceId = traceId ?: return
        val task = taskForTrace(taskTraceId) ?: return
        if (!task.pendingReconnect) return
        val delay = (800L * incrementReconnectAttempts()).coerceAtMost(5_000L)
        binding.root.postDelayed({
            val current = taskForTrace(taskTraceId) ?: return@postDelayed
            if (!current.pendingReconnect || isBackendConnected()) return@postDelayed
            startTaskWorkService(
                TaskWorkService.ACTION_RESUME_PENDING,
                null,
                getActiveRequestIsDevelopment(),
                current.traceId
            )
        }, delay)
    }

    fun resumePendingWorkAfterReconnect() {
        val payload = getPendingRequestPayload()
        if (payload.isNullOrBlank()) {
            setPendingReconnectForActiveWork(false)
            setWaitingForReply(false)
            setActiveRequestIsDevelopment(false)
            stopWorkingEvidenceForActiveConversation()
            clearPersistedActiveWork()
            setSendEnabled(true)
            appendMessage(
                ChatMessage(
                    "ai-stopped",
                    workflowStoppedMessage("连接已恢复，但没有找到可继续的请求。请重新发送一次。", false)
                )
            )
            return
        }

        setPendingReconnectForActiveWork(false)
        recordEvidence("connection", "连接已恢复，已自动继续上次任务")
        if (getActiveRequestIsDevelopment()) {
            updateStage(getCurrentStage(), "连接已恢复，正在继续本轮开发任务。")
            addProjectEvent("连接恢复，自动继续任务")
        }

        val responseToken = nextServerResponseToken()
        if (!startTaskWorkService(TaskWorkService.ACTION_RESUME_PENDING, null, getActiveRequestIsDevelopment(), null)) {
            setPendingReconnectForActiveWork(true)
            persistActiveWork()
            scheduleReconnectForActiveWork()
        } else {
            activeConversationTask()?.let { scheduleFirstServerResponseWatchdog(it.traceId, responseToken) }
        }
    }
}
