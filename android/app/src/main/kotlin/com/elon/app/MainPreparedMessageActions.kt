package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.google.gson.JsonArray
import com.google.gson.JsonObject
import java.util.Locale
import java.util.UUID

internal class MainPreparedMessageActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val restoreSendTarget: (SendTarget) -> Boolean,
    private val isConversationTaskRunning: (SendTarget) -> Boolean,
    private val setSendEnabled: (Boolean) -> Unit,
    private val userId: () -> String,
    private val projectIconDataUrl: (String) -> String?,
    private val selectedAgentForRequest: () -> String?,
    private val selectedRuntimeRouteForRequest: () -> String?,
    private val appendMessage: (ChatMessage) -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val looksLikeDevelopmentRequest: (String) -> Boolean,
    private val looksLikeDirectImageRequest: (String) -> Boolean,
    private val rememberConversationTask: (SendTarget, String, String, Boolean) -> Unit,
    private val setActiveRequestIsDevelopment: (Boolean) -> Unit,
    private val setActiveRequestIsPlanning: (Boolean) -> Unit,
    private val resetRequestState: () -> Unit,
    private val acceptDevelopmentRequest: (String) -> Unit,
    private val updateProjectViews: (String) -> Unit,
    private val nextServerResponseToken: () -> Int,
    private val putTaskResponseToken: (String, Int) -> Unit,
    private val startTaskWorkService: (String, String?, Boolean, String?) -> Boolean,
    private val ensureBackgroundKeepAlive: (Boolean) -> Unit,
    private val markTaskPendingReconnect: (SendTarget) -> Unit,
    private val refreshActiveTaskState: () -> Unit,
    private val persistActiveWork: () -> Unit,
    private val updateStage: (String, String) -> Unit,
    private val scheduleFirstServerResponseWatchdog: (String, Int) -> Unit,
    private val clearPendingAttachments: () -> Unit
) {
    fun startPreparedMessage(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: JsonArray,
        target: SendTarget,
        chatAttachments: List<ChatAttachment>,
        executionMode: ProjectRequestExecutionMode = ProjectRequestExecutionMode.Execute
    ) = startPreparedMessageInternal(
        visibleText,
        outgoingText,
        attachmentRefs,
        target,
        chatAttachments,
        appendUserBubble = true,
        executionMode = executionMode
    )

    fun startPreparedMessageAfterUserBubble(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: JsonArray,
        target: SendTarget,
        chatAttachments: List<ChatAttachment>,
        executionMode: ProjectRequestExecutionMode = ProjectRequestExecutionMode.Execute
    ) = startPreparedMessageInternal(
        visibleText,
        outgoingText,
        attachmentRefs,
        target,
        chatAttachments,
        appendUserBubble = false,
        executionMode = executionMode
    )

    private fun startPreparedMessageInternal(
        visibleText: String,
        outgoingText: String,
        attachmentRefs: JsonArray,
        target: SendTarget,
        chatAttachments: List<ChatAttachment>,
        appendUserBubble: Boolean,
        executionMode: ProjectRequestExecutionMode
    ) {
        if (!restoreSendTarget(target)) {
            Toast.makeText(activity, "Target conversation no longer exists.", Toast.LENGTH_LONG).show()
            setSendEnabled(true)
            return
        }
        if (isConversationTaskRunning(target)) {
            Toast.makeText(activity, "这个会话正在工作中，请换一个会话并行开发。", Toast.LENGTH_LONG).show()
            setSendEnabled(true)
            return
        }

        val traceId = "ui_${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(8)}"
        val payload = buildPayload(traceId, target, outgoingText, attachmentRefs, executionMode)
        val payloadJson = payload.toString()
        val requestIsDevelopment = shouldUseDevelopmentPresentation(outgoingText, executionMode)

        if (appendUserBubble) appendUserMessage(visibleText, chatAttachments)
        recordSendTrace(traceId, target, outgoingText, attachmentRefs, executionMode)
        binding.inputEdit.text.clear()
        collapseInputComposer()

        rememberConversationTask(target, traceId, payloadJson, requestIsDevelopment)
        setSendEnabled(false)
        setActiveRequestIsDevelopment(requestIsDevelopment)
        resetRequestState()
        setActiveRequestIsPlanning(executionMode.isPlan)
        updateRequestPresentation(visibleText, outgoingText, requestIsDevelopment, executionMode, attachmentRefs)
        ensureBackgroundKeepAlive(requestIsDevelopment)

        val responseToken = nextServerResponseToken()
        putTaskResponseToken(traceId, responseToken)
        startForegroundWork(target, payloadJson, requestIsDevelopment, traceId, responseToken)
    }

    private fun buildPayload(
        traceId: String,
        target: SendTarget,
        outgoingText: String,
        attachmentRefs: JsonArray,
        executionMode: ProjectRequestExecutionMode
    ): JsonObject {
        return JsonObject().apply {
            addProperty("trace_id", traceId)
            addProperty("client_request_id", traceId)
            addProperty("user_id", userId())
            addProperty("project_id", target.projectId)
            addProperty("project_title", target.projectTitle)
            projectIconDataUrl(target.projectId)
                ?.takeIf { it.isNotBlank() }
                ?.let { addProperty("project_icon_data_url", it) }
            addProperty("conversation_id", target.conversationId)
            addProperty("conversation_title", target.conversationTitle)
            addProperty("message", outgoingText)
            addProperty("execution_mode", executionMode.wireValue)
            addProperty("plan_mode", executionMode.isPlan)
            selectedAgentForRequest()?.let { addProperty("agent", it) }
            selectedRuntimeRouteForRequest()?.let { addProperty("runtimeRoute", it) }
            if (attachmentRefs.size() > 0) add("attachments", attachmentRefs)
        }
    }

    private fun appendUserMessage(visibleText: String, chatAttachments: List<ChatAttachment>) {
        appendMessage(
            ChatMessage(
                role = "user",
                content = visibleText,
                attachments = chatAttachments.takeIf { it.isNotEmpty() }
            )
        )
    }

    private fun recordSendTrace(
        traceId: String,
        target: SendTarget,
        outgoingText: String,
        attachmentRefs: JsonArray,
        executionMode: ProjectRequestExecutionMode
    ) {
        DebugTraceStore.record(
            "ui_chat_send",
            mapOf(
                "trace_id" to traceId,
                "project_id" to target.projectId,
                "conversation_id" to target.conversationId,
                "chars" to outgoingText.length,
                "execution_mode" to executionMode.wireValue,
                "attachment_refs" to attachmentRefs.size()
            )
        )
    }

    private fun updateRequestPresentation(
        visibleText: String,
        outgoingText: String,
        requestIsDevelopment: Boolean,
        executionMode: ProjectRequestExecutionMode,
        attachmentRefs: JsonArray
    ) {
        if (requestIsDevelopment) {
            acceptDevelopmentRequest(visibleText)
            appendMessage(
                CodexInteractionPresentation.intentMessage(
                    visibleText = visibleText,
                    outgoingText = outgoingText,
                    isDevelopment = true,
                    executionMode = executionMode,
                    hasAttachments = attachmentRefs.size() > 0
                )
            )
        }
        appendMessage(ChatMessage("ai-working", initialWorkflowMessage(requestIsDevelopment)))
    }

    private fun shouldUseDevelopmentPresentation(
        outgoingText: String,
        executionMode: ProjectRequestExecutionMode
    ): Boolean {
        if (executionMode.isPlan) return true
        if (looksLikeDirectImageRequest(outgoingText)) return false

        val runtimeRoute = selectedRuntimeRouteForRequest()
            ?.trim()
            ?.lowercase(Locale.US)
        if (runtimeRoute == AiRuntimeRoute.MyKey.wireValue) {
            return looksLikeDevelopmentRequest(outgoingText)
        }

        return true
    }

    private fun startForegroundWork(
        target: SendTarget,
        payloadJson: String,
        requestIsDevelopment: Boolean,
        traceId: String,
        responseToken: Int
    ) {
        if (!startTaskWorkService(TaskWorkService.ACTION_START_WORK, payloadJson, requestIsDevelopment, traceId)) {
            markTaskPendingReconnect(target)
            refreshActiveTaskState()
            persistActiveWork()
            if (requestIsDevelopment) {
                updateStage("连接恢复", "任务请求已保留，正在重新连接服务器。")
            }
        } else {
            clearPendingAttachments()
        }
        scheduleFirstServerResponseWatchdog(traceId, responseToken)
    }
}
