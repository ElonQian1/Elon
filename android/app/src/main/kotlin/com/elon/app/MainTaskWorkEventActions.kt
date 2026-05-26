package com.elon.app

import android.content.Intent

internal class MainTaskWorkEventActions(
    private val getBackendConnected: () -> Boolean,
    private val setBackendConnected: (Boolean) -> Unit,
    private val getWaitingForReply: () -> Boolean,
    private val resetReconnectAttempts: () -> Unit,
    private val updateFirstConversationStatus: (String) -> Unit,
    private val updateConversationTaskFromService: (String?, String?, String?, Boolean?, Boolean?) -> ConversationTaskState?,
    private val activeConversationTask: () -> ConversationTaskState?,
    private val recordEvidence: (String, String) -> Unit,
    private val setSendEnabled: (Boolean) -> Unit,
    private val isActiveConversationWorking: () -> Boolean,
    private val handleActiveWorkDisconnected: (ConversationTaskState) -> Unit,
    private val updateIdleReadyStatus: () -> Unit,
    private val appendTaskMessage: (String, String?, String?, String?, Boolean?) -> Unit,
    private val removeConversationTask: (String?, String?, String?) -> ConversationTaskState?,
    private val syncActiveTasksFromServiceState: (String?) -> Unit,
    private val clearTaskMaps: () -> Unit,
    private val refreshActiveTaskState: () -> Unit,
    private val navigateToLogin: () -> Unit = {},
) {
    fun handleTaskWorkEvent(intent: Intent) {
        when (intent.action) {
            TaskWorkService.ACTION_EVENT -> handleEvent(intent)
            TaskWorkService.ACTION_STATE -> handleState(intent)
        }
    }

    private fun handleEvent(intent: Intent) {
        setBackendConnected(intent.getBooleanExtra(TaskWorkService.EXTRA_CONNECTED, getBackendConnected()))
        val traceId = intent.getStringExtra(TaskWorkService.EXTRA_TRACE_ID)?.takeIf { it.isNotBlank() }
        val projectId = intent.getStringExtra(TaskWorkService.EXTRA_PROJECT_ID)?.takeIf { it.isNotBlank() }
        val conversationId = intent.getStringExtra(TaskWorkService.EXTRA_CONVERSATION_ID)?.takeIf { it.isNotBlank() }
        val isDevelopment = if (intent.hasExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT)) {
            intent.getBooleanExtra(TaskWorkService.EXTRA_IS_DEVELOPMENT, true)
        } else {
            null
        }
        when (intent.getStringExtra(TaskWorkService.EXTRA_KIND)) {
            "connected" -> handleConnected(traceId, projectId, conversationId, isDevelopment)
            "disconnected" -> handleDisconnected(traceId, projectId, conversationId, isDevelopment)
            "auth_required" -> handleAuthRequired()
            "message" -> {
                intent.getStringExtra(TaskWorkService.EXTRA_RAW_MESSAGE)?.let { raw ->
                    appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
                }
            }
            "paused" -> {
                removeConversationTask(traceId, projectId, conversationId)
                updateIdleReadyStatus()
                setSendEnabled(!isActiveConversationWorking())
            }
        }
    }

    private fun handleConnected(
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?
    ) {
        resetReconnectAttempts()
        updateFirstConversationStatus("已连接 · 点击进入开发会话")
        val task = updateConversationTaskFromService(
            traceId,
            projectId,
            conversationId,
            isDevelopment,
            false
        )
        if (task != null && activeConversationTask()?.traceId == task.traceId) {
            recordEvidence("connection", "连接已恢复，后台任务继续运行")
        }
        setSendEnabled(!isActiveConversationWorking())
    }

    private fun handleDisconnected(
        traceId: String?,
        projectId: String?,
        conversationId: String?,
        isDevelopment: Boolean?
    ) {
        setBackendConnected(false)
        val task = updateConversationTaskFromService(
            traceId,
            projectId,
            conversationId,
            isDevelopment,
            true
        )
        if (task != null && activeConversationTask()?.traceId == task.traceId) {
            handleActiveWorkDisconnected(task)
        } else {
            updateIdleReadyStatus()
            setSendEnabled(!isActiveConversationWorking())
        }
    }

    private fun handleAuthRequired() {
        setBackendConnected(false)
        updateFirstConversationStatus("登录已失效，请重新登录")
        setSendEnabled(false)
        navigateToLogin()
    }

    private fun handleState(intent: Intent) {
        setBackendConnected(intent.getBooleanExtra(TaskWorkService.EXTRA_CONNECTED, getBackendConnected()))
        val serviceWaiting = intent.getBooleanExtra(TaskWorkService.EXTRA_WAITING, getWaitingForReply())
        syncActiveTasksFromServiceState(intent.getStringExtra(TaskWorkService.EXTRA_ACTIVE_TASKS))
        if (!serviceWaiting) {
            clearTaskMaps()
            refreshActiveTaskState()
            updateIdleReadyStatus()
        }
        setSendEnabled(!isActiveConversationWorking())
    }
}
