package com.elon.app

import android.view.View
import com.elon.app.chatgptweb.ChatGptWebConversation

internal class WebChatConversationProjectMoveReadTransition(
    private val host: View,
    private val isCurrent: (Int) -> Boolean,
) {
    fun invoke(
        port: WebChatConsumerPort,
        control: WebChatConsumerControlDescriptor,
        userConfirmed: Boolean,
        epoch: Int,
        afterTouchMiss: Boolean = false,
        onAccepted: () -> Unit,
        onRejected: () -> Unit,
    ) {
        val result = if (afterTouchMiss) {
            port.invokeControlAfterTouchMiss(control.control.id, userConfirmed)
        } else {
            port.invokeControl(control.control.id, userConfirmed)
        }
        if (!result.accepted) {
            onRejected()
            return
        }
        // The official menu is mounted asynchronously after the accepted read transition.
        host.postDelayed({
            if (isCurrent(epoch)) onAccepted()
        }, WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS)
    }

    fun refreshControls(
        port: WebChatConsumerPort,
        epoch: Int,
        continuation: () -> Unit,
    ) {
        port.requestControls()
        host.postDelayed({
            if (isCurrent(epoch)) continuation()
        }, WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS)
    }

    fun waitForMoveTrigger(
        conversation: ChatGptWebConversation,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
        optionsOpenRetries: Int = 0,
        onProgress: (String) -> Unit,
        onReady: () -> Unit,
        onFailure: (String) -> Unit,
    ) {
        if (!isCurrent(epoch)) return
        val state = port.state()
        WebChatConversationProjectMoveDiagnostics.recordControls(
            "move_trigger",
            attempt,
            state,
            conversation,
        )
        val trigger = WebChatConversationProjectMovePolicy.moveTrigger(state, conversation)
        if (trigger != null) {
            onProgress("正在打开项目列表")
            invoke(
                port = port,
                control = trigger,
                userConfirmed = true,
                epoch = epoch,
                onAccepted = onReady,
                onRejected = { onFailure("无法打开项目列表") },
            )
            return
        }
        if (WebChatConversationProjectMoveTiming.shouldRefreshControls(attempt)) {
            refreshControls(port, epoch) {
                waitForMoveTrigger(
                    conversation,
                    port,
                    epoch,
                    attempt + 1,
                    optionsOpenRetries,
                    onProgress,
                    onReady,
                    onFailure,
                )
            }
            return
        }
        if (
            WebChatConversationProjectMoveTiming.shouldRetryConversationOptions(
                attempt,
                optionsOpenRetries,
            )
        ) {
            val retryControl = WebChatConversationProjectMovePolicy.retryableConversationOptions(
                state,
                conversation,
            )
            if (retryControl != null) {
                onProgress("正在重试会话设置")
                invoke(
                    port = port,
                    control = retryControl,
                    userConfirmed = false,
                    epoch = epoch,
                    afterTouchMiss = true,
                    onAccepted = {
                        waitForMoveTrigger(
                            conversation,
                            port,
                            epoch,
                            attempt = 0,
                            optionsOpenRetries = optionsOpenRetries + 1,
                            onProgress,
                            onReady,
                            onFailure,
                        )
                    },
                    onRejected = { onFailure("无法重新打开会话设置") },
                )
                return
            }
        }
        if (attempt >= WebChatConversationProjectMoveTiming.CONTROL_POLL_LIMIT) {
            onFailure("官网项目入口暂不可用")
            return
        }
        host.postDelayed({
            waitForMoveTrigger(
                conversation,
                port,
                epoch,
                attempt + 1,
                optionsOpenRetries,
                onProgress,
                onReady,
                onFailure,
            )
        }, WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS)
    }
}
