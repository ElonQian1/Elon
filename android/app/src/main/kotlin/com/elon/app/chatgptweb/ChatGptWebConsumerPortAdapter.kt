package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerCommandResult
import com.elon.app.WebChatConsumerCommandRequest
import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConsumerFeature
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
        val features = if (observed.adapterCurrent) {
            observed.features.map { feature ->
                WebChatConsumerFeature(
                    id = feature.id,
                    label = feature.label,
                    kind = feature.kind,
                    selected = feature.selected,
                    requiresUserConfirmation = ChatGptWebProductCapabilityCatalog
                        .requiresUserConfirmation(feature.kind),
                    nativeSelector = ChatGptNativeNavigationSelector.feature(feature),
                )
            }
        } else {
            emptyList()
        }
        return WebChatConsumerState(
            streaming = current?.streaming == true,
            dictationActive = current?.dictationActive == true,
            composerSections = composerSections,
            pageKind = current?.pageKind ?: "unknown",
            pageUrl = current?.url.orEmpty(),
            features = features,
            commandRequests = observed.commandRequests.map { request ->
                WebChatConsumerCommandRequest(
                    id = request.id,
                    status = request.status.toConsumerStatus(),
                )
            },
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

    override fun requestFeatures(): WebChatConsumerCommandResult =
        execute(JSONObject().put("action", "chatgpt_list_features"))

    override fun selectFeature(
        featureId: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult = execute(JSONObject()
        .put("action", "chatgpt_select_feature")
        .put("feature_id", featureId)
        .put("user_confirmed", userConfirmed))

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

    private fun String.toConsumerStatus(): WebChatConsumerCommandStatus = when (this) {
        ChatGptWebObservedState.CommandRequest.PENDING -> WebChatConsumerCommandStatus.PENDING
        ChatGptWebObservedState.CommandRequest.SUCCEEDED -> WebChatConsumerCommandStatus.SUCCEEDED
        ChatGptWebObservedState.CommandRequest.FAILED -> WebChatConsumerCommandStatus.FAILED
        ChatGptWebObservedState.CommandRequest.TIMED_OUT -> WebChatConsumerCommandStatus.TIMED_OUT
        else -> WebChatConsumerCommandStatus.UNKNOWN
    }

    private companion object {
        val SESSION_COMMANDS = setOf(
            "chatgpt_stop_generation",
            "chatgpt_start_dictation",
            "chatgpt_submit_dictation",
        )
    }
}
