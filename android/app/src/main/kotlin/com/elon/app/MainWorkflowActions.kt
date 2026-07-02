package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainWorkflowActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val chatAdapter: () -> ChatAdapter,
    private val activeRequestIsDevelopment: () -> Boolean,
    private val setActiveRequestIsDevelopment: (Boolean) -> Unit,
    private val activeRequestIsPlanning: () -> Boolean,
    private val setActiveRequestIsPlanning: (Boolean) -> Unit,
    private val setWaitingForReply: (Boolean) -> Unit,
    private val preparePlanImplementationPrompt: () -> Unit,
    private val clearPendingRequestPayload: () -> Unit,
    private val clearPendingReconnectForActiveWork: () -> Unit,
    private val resetReconnectAttempts: () -> Unit,
    private val incrementServerResponseToken: () -> Unit,
    private val currentTimeText: () -> String,
    private val taskResponseTokens: MutableMap<String, Int>,
    private val runningTraceToConversation: Map<String, String>,
    private val runningConversationTasks: Map<String, ConversationTaskState>,
    private val projectStateActions: () -> MainProjectStateActions,
    private val projectViewActions: () -> MainProjectViewActions,
    private val projectRecordActions: () -> MainProjectRecordActions,
    private val conversationPreviewActions: () -> MainConversationPreviewActions,
    private val conversationTaskRegistryActions: () -> MainConversationTaskRegistryActions,
    private val taskWorkServiceActions: () -> MainTaskWorkServiceActions,
    private val sendEnabledActions: () -> MainSendEnabledActions
) {
    val evidenceActions: MainEvidenceActions by lazy {
        MainEvidenceActions(
            activeConversation = projectStateActions()::activeConversation,
            chatAdapter = chatAdapter,
            saveConversations = projectStateActions()::saveConversations,
            assistantEvidenceRoles = MainWorkflowRoles.assistantEvidence
        )
    }

    val foldedCliLogActions: MainFoldedCliLogActions by lazy {
        MainFoldedCliLogActions(
            currentStage = { projectStateActions().currentStage },
            updateStage = projectViewActions()::updateStage,
            maybeAppendVisibleCliSignal = { category, line ->
                progressNarrativeActions.maybeAppendVisibleCliSignal(category, line)
            },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment()) evidenceActions.recordEvidence(kind, detail)
            }
        )
    }

    val progressNarrativeActions: MainProgressNarrativeActions by lazy {
        MainProgressNarrativeActions(
            isDevelopmentRequest = activeRequestIsDevelopment,
            finalizeEvidenceForLatestAssistant = { evidenceActions.finalizeEvidenceForLatestAssistant() },
            appendMessage = messageAppendActions::appendMessage,
            attachEvidenceToLatestAi = { evidenceActions.attachEvidenceToLatestAi() }
        )
    }

    val projectHygieneActions: MainProjectHygieneActions by lazy {
        MainProjectHygieneActions(
            timeText = currentTimeText,
            removeLeakedAndRoutineWorkflowMessages = { messages ->
                workflowMessageCompactor.removeLeakedAndRoutineWorkflowMessages(messages)
            },
            compactWorkflowStatusMessages = { messages ->
                workflowMessageCompactor.compactWorkflowStatusMessages(messages)
            },
            closeStaleWorkflowMessages = { messages ->
                workflowMessageCompactor.closeStaleWorkflowMessages(messages)
            }
        )
    }

    val workflowMessageCompactor: MainWorkflowMessageCompactor by lazy {
        MainWorkflowMessageCompactor(
            staleWorkflowRoles = MainWorkflowRoles.staleWorkflow,
            workflowHistoryStatusRoles = MainWorkflowRoles.historyStatus,
            workflowTerminalRoles = MainWorkflowRoles.terminal
        )
    }

    val serverResponseWatchdogActions: MainServerResponseWatchdogActions by lazy {
        MainServerResponseWatchdogActions(
            binding = binding,
            taskResponseTokens = taskResponseTokens,
            taskForTrace = { traceId -> runningTraceToConversation[traceId]?.let { runningConversationTasks[it] } },
            activeConversationTask = conversationTaskRegistryActions()::activeConversationTask,
            getCurrentStage = { projectStateActions().currentStage },
            getActiveRequestIsDevelopment = activeRequestIsDevelopment,
            refreshActiveTaskState = conversationTaskRegistryActions()::refreshActiveTaskState,
            updateStage = projectViewActions()::updateStage,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            startTaskWorkService = taskWorkServiceActions()::startTaskWorkService
        )
    }

    val assistantRawMessageActions: MainAssistantRawMessageActions by lazy {
        MainAssistantRawMessageActions(
            activity = activity,
            assistantStreamEvents = { assistantStreamEvents },
            assistantTerminalActions = { assistantTerminalActions },
            incrementServerResponseToken = incrementServerResponseToken,
            appendMessage = messageAppendActions::appendMessage,
            isDevelopmentRequest = activeRequestIsDevelopment,
            streamAppendChunk = { sid, chunk ->
                activity.runOnUiThread { chatAdapter().streamAppendChunk(sid, chunk) }
            }
        )
    }

    val messageAppendActions: MainMessageAppendActions by lazy {
        MainMessageAppendActions(
            binding = binding,
            chatAdapter = chatAdapter,
            activeConversation = projectStateActions()::activeConversation,
            workflowMessageCompactor = { workflowMessageCompactor },
            updateActiveConversationPreview = { message ->
                conversationPreviewActions().updateActiveConversationPreview(message)
            },
            saveConversations = projectStateActions()::saveConversations,
            workflowTerminalRoles = MainWorkflowRoles.terminal
        )
    }

    val assistantStreamEvents: MainAssistantStreamEvents by lazy {
        MainAssistantStreamEvents(
            handleTaskEvent = { event, taskId, content ->
                workflowStageActions.handleTaskEvent(event, taskId, content)
            },
            maybeAppendTaskEventNarrative = { event, content ->
                progressNarrativeActions.maybeAppendTaskEventNarrative(event, content)
            },
            maybeAppendWorkflowProgressNarrative = { content ->
                progressNarrativeActions.maybeAppendWorkflowProgressNarrative(content)
            },
            maybeAppendToolCallNarrative = { tool ->
                progressNarrativeActions.maybeAppendToolCallNarrative(tool)
            },
            handleProgress = { content, recordProgressEvidence ->
                workflowStageActions.handleProgress(content, recordProgressEvidence)
            },
            handleFoldedCliOutput = { content -> foldedCliLogActions.handleFoldedCliOutput(content) },
            markToolCallStarted = { tool -> workflowStageActions.handleToolCall(tool) },
            markToolResult = { workflowStageActions.markToolResult(it) },
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment()) evidenceActions.recordEvidence(kind, detail)
            },
            isDevelopmentRequest = activeRequestIsDevelopment,
            addProjectEvent = projectRecordActions()::addProjectEvent
        )
    }

    val assistantTerminalActions: MainAssistantTerminalActions by lazy {
        MainAssistantTerminalActions(
            getActiveRequestIsDevelopment = activeRequestIsDevelopment,
            setActiveRequestIsDevelopment = setActiveRequestIsDevelopment,
            getActiveRequestIsPlanning = activeRequestIsPlanning,
            setActiveRequestIsPlanning = setActiveRequestIsPlanning,
            setWaitingForReply = setWaitingForReply,
            setSendEnabled = sendEnabledActions()::setSendEnabled,
            clearPendingRequestPayload = clearPendingRequestPayload,
            clearPendingReconnectForActiveWork = clearPendingReconnectForActiveWork,
            resetReconnectAttempts = resetReconnectAttempts,
            clearPersistedActiveWork = conversationTaskRegistryActions()::clearPersistedActiveWork,
            updateStage = projectViewActions()::updateStage,
            updateProjectViews = projectViewActions()::updateProjectViews,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment()) evidenceActions.recordEvidence(kind, detail)
            },
            stopWorkingEvidenceForActiveConversation = {
                evidenceActions.stopWorkingEvidenceForActiveConversation()
            },
            clearCurrentEvidence = { evidenceActions.clearCurrentEvidence() },
            resetFoldedCliLog = { foldedCliLogActions.reset() },
            promoteLatestAssistantReplyWithCurrentEvidence =
                evidenceActions::promoteLatestAssistantReplyWithCurrentEvidence,
            aiMessageWithCurrentEvidence = evidenceActions::aiMessageWithCurrentEvidence,
            appendMessage = messageAppendActions::appendMessage,
            preparePlanImplementationPrompt = preparePlanImplementationPrompt,
            workflowStoppedMessage = { reason ->
                mainWorkflowStoppedMessage(reason, activeRequestIsDevelopment())
            }
        )
    }

    val workflowStageActions: MainWorkflowStageActions by lazy {
        MainWorkflowStageActions(
            currentStage = { projectStateActions().currentStage },
            updateStage = projectViewActions()::updateStage,
            addProjectEvent = projectRecordActions()::addProjectEvent,
            recordEvidence = { kind, detail ->
                if (activeRequestIsDevelopment()) evidenceActions.recordEvidence(kind, detail)
            }
        )
    }
}
