package com.elon.app

import android.view.View
import org.json.JSONObject

internal enum class WebChatDictationMcpTarget { START_OR_SUBMIT, CANCEL }

internal data class WebChatDictationMcpSnapshot(
    val phase: String,
    val transport: String?,
    val startOrSubmitSelector: String?,
    val cancelSelector: String?,
)

internal object WebChatDictationMcpPolicy {
    fun snapshot(startOrSubmitSelector: String?, cancelSelector: String?): WebChatDictationMcpSnapshot {
        val dictationCancelSelector = cancelSelector.takeIf { it.isCancelSelector() }
        val dictationStartOrSubmitSelector = startOrSubmitSelector.takeIf {
            it == START_SELECTOR || it == DOM_STARTING_SELECTOR ||
                it == DOM_START_FAILED_SELECTOR || it.isSubmitSelector()
        }
        return WebChatDictationMcpSnapshot(
            phase = when {
                dictationStartOrSubmitSelector == DOM_STARTING_SELECTOR -> "starting"
                dictationStartOrSubmitSelector == DOM_START_FAILED_SELECTOR -> "failed"
                dictationCancelSelector != null -> "active"
                dictationStartOrSubmitSelector.isSubmitSelector() -> "active"
                else -> "idle"
            },
            transport = sequenceOf(dictationCancelSelector, dictationStartOrSubmitSelector)
                .mapNotNull(::transportFromSelector)
                .firstOrNull(),
            startOrSubmitSelector = dictationStartOrSubmitSelector,
            cancelSelector = dictationCancelSelector,
        )
    }

    fun target(action: String, snapshot: WebChatDictationMcpSnapshot): WebChatDictationMcpTarget? =
        when (action) {
            ACTION_START -> WebChatDictationMcpTarget.START_OR_SUBMIT
                .takeIf { snapshot.startOrSubmitSelector == START_SELECTOR }
            ACTION_SUBMIT -> WebChatDictationMcpTarget.START_OR_SUBMIT
                .takeIf { snapshot.startOrSubmitSelector.isSubmitSelector() }
            ACTION_CANCEL -> WebChatDictationMcpTarget.CANCEL
                .takeIf { snapshot.cancelSelector.isCancelSelector() }
            else -> null
        }

    private fun transportFromSelector(selector: String?): String? = when {
        selector == null -> null
        ":private:" in selector -> "private"
        ":shared:" in selector -> "shared"
        ":dom:" in selector || ":chatgpt_web:" in selector -> "official_dom"
        else -> null
    }

    private fun String?.isSubmitSelector(): Boolean = this?.endsWith(":submit-dictation") == true
    private fun String?.isCancelSelector(): Boolean = this?.endsWith(":cancel-dictation") == true

    const val ACTION_START = "start_web_chat_dictation"
    const val ACTION_SUBMIT = "submit_web_chat_dictation"
    const val ACTION_CANCEL = "cancel_web_chat_dictation"
    val ACTIONS = setOf(ACTION_START, ACTION_SUBMIT, ACTION_CANCEL)
    private const val START_SELECTOR = "web-chat-composer-command:start-dictation"
    private const val DOM_STARTING_SELECTOR = "web-chat-composer-command:dom:starting-dictation"
    private const val DOM_START_FAILED_SELECTOR =
        "web-chat-composer-command:dom:start-failed-dictation"
}

/** Semantic MCP control for the exact production composer listeners. */
internal class WebChatDictationMcpActions(
    private val views: () -> MainInputComposerViews?,
) {
    fun stateJson(): JSONObject {
        val currentViews = views()
        val button = currentViews?.webDictationButton
        val snapshot = snapshot(currentViews)
        return JSONObject()
            .put("visible", button?.visibility == View.VISIBLE)
            .put("enabled", button?.isEnabled == true)
            .put("phase", snapshot.phase)
            .put("transport", snapshot.transport ?: JSONObject.NULL)
            .put("start_or_submit_selector", snapshot.startOrSubmitSelector ?: JSONObject.NULL)
            .put("cancel_selector", snapshot.cancelSelector ?: JSONObject.NULL)
    }

    fun control(action: String): Boolean {
        val currentViews = views() ?: return false
        val target = WebChatDictationMcpPolicy.target(action, snapshot(currentViews)) ?: return false
        val view = when (target) {
            WebChatDictationMcpTarget.START_OR_SUBMIT -> currentViews.webDictationButton
            WebChatDictationMcpTarget.CANCEL -> currentViews.inputModeButton
        }
        return view.visibility == View.VISIBLE && view.isEnabled && view.performClick()
    }

    private fun snapshot(currentViews: MainInputComposerViews?) =
        WebChatDictationMcpPolicy.snapshot(
            currentViews?.webDictationButton?.tag?.toString(),
            currentViews?.inputModeButton?.tag?.toString(),
        )
}
