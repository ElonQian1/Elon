package com.elon.app

internal class MainAssistantTerminalActions(
    private val getActiveRequestIsDevelopment: () -> Boolean,
    private val setActiveRequestIsDevelopment: (Boolean) -> Unit,
    private val getActiveRequestIsPlanning: () -> Boolean,
    private val setActiveRequestIsPlanning: (Boolean) -> Unit,
    private val setWaitingForReply: (Boolean) -> Unit,
    private val setSendEnabled: (Boolean) -> Unit,
    private val clearPendingRequestPayload: () -> Unit,
    private val clearPendingReconnectForActiveWork: () -> Unit,
    private val resetReconnectAttempts: () -> Unit,
    private val clearPersistedActiveWork: () -> Unit,
    private val updateStage: (String, String) -> Unit,
    private val updateProjectViews: (String) -> Unit,
    private val addProjectEvent: (String) -> Unit,
    private val recordEvidence: (String, String) -> Unit,
    private val stopWorkingEvidenceForActiveConversation: () -> Unit,
    private val clearCurrentEvidence: () -> Unit,
    private val resetFoldedCliLog: () -> Unit,
    private val aiMessageWithCurrentEvidence: (String, List<ChatAttachment>) -> ChatMessage,
    private val appendMessage: (ChatMessage) -> Unit,
    private val preparePlanImplementationPrompt: () -> Unit,
    private val workflowStoppedMessage: (String) -> String
) {
    fun handleDone(content: String, apkUrl: String?, imageUrl: String?, modelUsed: String? = null, nodeId: String? = null): ChatMessage? {
        resetPendingRequestState()
        val wasDevelopment = getActiveRequestIsDevelopment()
        val wasPlanning = getActiveRequestIsPlanning()
        if (wasPlanning) {
            updateStage("规划完成", "计划已生成，确认后可开始实现。")
            addProjectEvent("生成开发计划")
            recordEvidence("result", "计划已生成，等待用户确认")
        } else if (wasDevelopment) {
            updateStage(
                "交付完成",
                if (apkUrl != null) "APK 已生成，可以下载安装测试。" else "任务已完成，可以继续提出修改。"
            )
            addProjectEvent(if (apkUrl != null) "生成 APK 下载链接" else "任务完成")
            recordEvidence("result", if (apkUrl != null) "APK 已生成：$apkUrl" else "任务完成")
        } else {
            updateProjectViews("普通消息已回复，开发项目记录保持不变。")
        }
        setActiveRequestIsDevelopment(false)
        setActiveRequestIsPlanning(false)
        stopWorkingEvidenceForActiveConversation()
        resetFoldedCliLog()
        if (wasPlanning) {
            preparePlanImplementationPrompt()
        }
        val visibleApkUrl = if (wasDevelopment && !wasPlanning) apkUrl else null
        // 若服务端已通过 assistant_message 流式推送过 AI 回复（Done.message 为空），
        // 非开发任务无需再额外追加一条"回复已完成"气泡；开发任务仍创建气泡以附上证据。
        if (content.isBlank() && !wasDevelopment && visibleApkUrl == null && imageUrl == null) {
            return null
        }
        return aiMessageWithCurrentEvidence(
            finalReplyMessage(planAwareContent(content, wasPlanning), visibleApkUrl, imageUrl, wasDevelopment),
            chatAttachmentFromImageUrl(imageUrl)
        ).let { msg ->
            var result = if (visibleApkUrl != null) msg.copy(apkUrl = visibleApkUrl) else msg
            if (modelUsed != null) result = result.copy(modelUsed = modelUsed)
            if (nodeId != null) result = result.copy(nodeId = nodeId)
            result
        }
    }

    fun handleError(rawMessage: String, code: String? = null, retryable: Boolean? = null): ChatMessage {
        resetPendingRequestState()
        val error = friendlyErrorMessage(rawMessage, code, retryable)
        val wasDevelopment = getActiveRequestIsDevelopment()
        if (wasDevelopment) {
            updateStage("需要处理", error)
            addProjectEvent("发生错误：${summarize(error, 30)}")
            recordEvidence("result", "发生错误：$error")
        }
        setActiveRequestIsDevelopment(false)
        setActiveRequestIsPlanning(false)
        stopWorkingEvidenceForActiveConversation()
        clearCurrentEvidence()
        return ChatMessage("error", error)
    }

    fun handleMalformedResponse() {
        resetPendingRequestState()
        if (getActiveRequestIsDevelopment()) {
            updateStage("需要处理", "服务端返回内容无法识别。")
            addProjectEvent("服务端返回异常")
        }
        appendMessage(ChatMessage("ai-stopped", workflowStoppedMessage("服务端返回内容无法识别。")))
        setActiveRequestIsDevelopment(false)
        setActiveRequestIsPlanning(false)
        stopWorkingEvidenceForActiveConversation()
        clearCurrentEvidence()
        appendMessage(ChatMessage("error", "服务端返回异常，无法解析。"))
    }

    private fun resetPendingRequestState() {
        setWaitingForReply(false)
        setSendEnabled(true)
        clearPendingRequestPayload()
        clearPendingReconnectForActiveWork()
        resetReconnectAttempts()
        clearPersistedActiveWork()
    }

    private fun planAwareContent(content: String, wasPlanning: Boolean): String {
        if (!wasPlanning) return content
        val trimmed = content.trim()
        val suffix = "下一步我已经帮你填好了：如果计划没问题，直接点发送开始实现；如果想调整，改输入框里的文字再发送。"
        return if (trimmed.isBlank()) suffix else "$trimmed\n\n$suffix"
    }
}
