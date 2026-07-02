package com.elon.app

import android.content.SharedPreferences
import androidx.appcompat.app.AppCompatActivity

internal class MainTaskActions(
    private val activity: AppCompatActivity,
    private val prefs: SharedPreferences,
    private val backendConnected: () -> Boolean,
    private val setBackendConnected: (Boolean) -> Unit,
    private val waitingForReply: () -> Boolean,
    private val resetReconnectAttempts: () -> Unit,
    private val taskResponseTokens: MutableMap<String, Int>,
    private val runningTraceToConversation: MutableMap<String, String>,
    private val runningConversationTasks: MutableMap<String, ConversationTaskState>,
    private val activeRequestIsDevelopment: () -> Boolean,
    private val workflowActions: () -> MainWorkflowActions,
    private val conversationPreviewActions: () -> MainConversationPreviewActions,
    private val conversationTaskRegistryActions: () -> MainConversationTaskRegistryActions,
    private val activeWorkControlActions: () -> MainActiveWorkControlActions,
    private val sendEnabledActions: () -> MainSendEnabledActions,
    private val isProjectConversationVisible: () -> Boolean,
    private val drainNextQueuedMessage: (String?, String?) -> Unit
) {
    val taskWorkEventActions: MainTaskWorkEventActions by lazy {
        MainTaskWorkEventActions(
            getBackendConnected = backendConnected,
            setBackendConnected = setBackendConnected,
            getWaitingForReply = waitingForReply,
            resetReconnectAttempts = resetReconnectAttempts,
            updateFirstConversationStatus = { text ->
                conversationPreviewActions().updateFirstConversationStatus(text)
            },
            updateConversationTaskFromService =
                conversationTaskRegistryActions()::updateConversationTaskFromService,
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment()) workflowActions().evidenceActions.recordEvidence(kind, detail)
            },
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            isActiveConversationWorking = conversationTaskRegistryActions()::isActiveConversationWorking,
            handleActiveWorkDisconnected = { task -> activeWorkControlActions().handleActiveWorkDisconnected(task) },
            updateIdleReadyStatus = { conversationPreviewActions().updateIdleReadyStatus() },
            appendTaskMessage = { raw, traceId, projectId, conversationId, isDevelopment ->
                traceId?.let { taskResponseTokens.remove(it) }
                taskMessageRouterActions.appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
            },
            removeConversationTask = conversationTaskRegistryActions()::removeConversationTask,
            syncActiveTasksFromServiceState = { activeTasksJson ->
                conversationTaskRegistryActions().syncActiveTasksFromServiceState(activeTasksJson)
            },
            clearTaskMaps = {
                runningConversationTasks.clear()
                runningTraceToConversation.clear()
                taskResponseTokens.clear()
            },
            refreshActiveTaskState = { conversationTaskRegistryActions().refreshActiveTaskState() },
            navigateToLogin = {
                activity.startActivity(android.content.Intent(activity, LoginActivity::class.java))
            }
        )
    }

    val taskWorkReceiverActions: MainTaskWorkReceiverActions by lazy {
        MainTaskWorkReceiverActions(
            activity = activity,
            handleTaskWorkEvent = { intent -> taskWorkEventActions.handleTaskWorkEvent(intent) }
        )
    }

    val taskMessageRouterActions: MainTaskMessageRouterActions by lazy {
        MainTaskMessageRouterActions(
            keyForTrace = { traceId -> runningTraceToConversation[traceId] },
            conversationTaskKey = conversationTaskRegistryActions()::conversationTaskKey,
            activeConversationTaskKey = conversationTaskRegistryActions()::activeConversationTaskKey,
            taskIsDevelopment = { key -> runningConversationTasks[key]?.isDevelopment },
            isProjectConversationVisible = isProjectConversationVisible,
            appendActiveMessage = { raw -> workflowActions().assistantRawMessageActions.appendMessage(raw) },
            appendBackgroundTaskMessage = { raw, key, isDevelopment ->
                backgroundTaskMessageActions.appendBackgroundTaskMessage(raw, key, isDevelopment)
            },
            removeConversationTask = conversationTaskRegistryActions()::removeConversationTask,
            persistActiveWork = conversationTaskRegistryActions()::persistActiveWork,
            updateConversationTaskFromService =
                conversationTaskRegistryActions()::updateConversationTaskFromService,
            drainNextQueuedMessage = drainNextQueuedMessage,
            markProjectTaskCompleted = { projectId ->
                markProjectTaskCompletionBadge(prefs, projectId)
            }
        )
    }

    val backgroundTaskMessageActions: MainBackgroundTaskMessageActions by lazy {
        MainBackgroundTaskMessageActions(
            activity = activity,
            findConversationLocationByKey = { key -> conversationPreviewActions().ensureConversationLocationByKey(key) },
            appendMessageToConversation = { projectIndex, conversationIndex, message ->
                conversationPreviewActions().appendMessageToConversation(projectIndex, conversationIndex, message)
            },
            appendEvidenceToConversation = { projectIndex, conversationIndex, entry, working ->
                conversationPreviewActions().appendEvidenceToConversation(projectIndex, conversationIndex, entry, working)
            },
            stopEvidenceForConversation = { projectIndex, conversationIndex ->
                conversationPreviewActions().stopEvidenceForConversation(projectIndex, conversationIndex)
            },
            appendStreamChunkToConversation = { projectIndex, conversationIndex, streamId, chunk ->
                conversationPreviewActions().appendStreamChunkToConversation(projectIndex, conversationIndex, streamId, chunk)
            }
        )
    }

    val taskWorkServiceActions: MainTaskWorkServiceActions by lazy {
        MainTaskWorkServiceActions(
            activity = activity,
            prefs = prefs,
            appendTaskMessage = { raw, traceId, projectId, conversationId, isDevelopment ->
                taskMessageRouterActions.appendTaskMessage(raw, traceId, projectId, conversationId, isDevelopment)
            },
            appendRawMessage = { raw -> workflowActions().assistantRawMessageActions.appendMessage(raw) }
        )
    }
}
