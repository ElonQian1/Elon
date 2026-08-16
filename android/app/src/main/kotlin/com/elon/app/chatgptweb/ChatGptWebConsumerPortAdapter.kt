package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerCommandResult
import com.elon.app.WebChatConsumerOption
import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatConsumerState
import org.json.JSONObject

internal class ChatGptWebConsumerPortAdapter(
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val observedState: () -> ChatGptWebObservedState.Snapshot,
    private val executeControl: (JSONObject) -> JSONObject,
) : WebChatConsumerPort {
    override fun state(): WebChatConsumerState {
        val observed = observedState()
        val current = snapshot().takeIf { observed.adapterCurrent }
        val composerSections = if (observed.adapterCurrent) {
            observed.composerSections.mapValues { (section, options) ->
                options.map { option ->
                    WebChatConsumerOption(
                        id = option.id,
                        label = option.label,
                        selected = option.selected,
                        semantic = option.semantic,
                        opensSubmenu = option.opensSubmenu,
                        nativeSelector = ChatGptNativeNavigationSelector.composerOption(section, option),
                    )
                }
            }
        } else {
            emptyMap()
        }
        return WebChatConsumerState(
            streaming = current?.streaming == true,
            dictationActive = current?.dictationActive == true,
            composerSections = composerSections,
        )
    }

    override fun requestComposerOptions(section: String): WebChatConsumerCommandResult =
        execute(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", section))

    override fun selectComposerOption(
        section: String,
        optionId: String,
    ): WebChatConsumerCommandResult = execute(JSONObject()
        .put("action", "chatgpt_select_composer_option")
        .put("section", section)
        .put("option_id", optionId))

    override fun executeSessionCommand(action: String): WebChatConsumerCommandResult {
        if (action !in SESSION_COMMANDS) {
            return WebChatConsumerCommandResult(
                accepted = false,
                error = "unsupported_consumer_command",
            )
        }
        return execute(JSONObject().put("action", action))
    }

    private fun execute(args: JSONObject): WebChatConsumerCommandResult {
        val response = executeControl(args)
        val receipt = response.optJSONObject("command_receipt")
        return WebChatConsumerCommandResult(
            accepted = response.optBoolean("control_ok"),
            error = response.optString("error")
                .trim()
                .takeIf(String::isNotBlank),
            requestId = receipt?.optString("request_id")
                ?.trim()
                ?.takeIf(String::isNotBlank),
        )
    }

    private companion object {
        val SESSION_COMMANDS = setOf(
            "chatgpt_stop_generation",
            "chatgpt_start_dictation",
            "chatgpt_submit_dictation",
        )
    }
}
