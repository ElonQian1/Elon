package com.elon.app.chatgptweb

import com.elon.app.WebChatConsumerCommandResult
import com.elon.app.WebChatConsumerCommandRequest
import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConsumerControlDescriptor
import com.elon.app.WebChatConsumerControlMutation
import com.elon.app.WebChatConsumerControlPresentation
import com.elon.app.WebChatConsumerFeature
import com.elon.app.WebChatConsumerOption
import com.elon.app.WebChatConsumerPort
import com.elon.app.WebChatConsumerState
import org.json.JSONObject

internal class ChatGptWebConsumerPortAdapter(
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val uiManifest: () -> ChatGptWebUiManifest?,
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
                        parentId = option.parentId,
                        parentLabel = option.parentLabel,
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
        val controls = uiManifest()
            .takeIf { observed.adapterCurrent }
            ?.let { manifest ->
                val presentations = ChatGptNativeControlPresentation.describe(manifest.controls)
                manifest.controls.map { control ->
                    val coverage = presentations[control.id]
                    WebChatConsumerControlDescriptor(
                        control = control,
                        requiresUserConfirmation = ChatGptWebControlInvocationPolicy.risk(control) ==
                            ChatGptWebControlInvocationPolicy.Risk.USER_CONFIRMATION,
                        presentation = coverage?.kind.toConsumerPresentation(),
                        nativeSelector = coverage?.nativeSelector,
                        pageActionPlacement = ChatGptNativeControlPresentation
                            .pageActionPlacement(control),
                    )
                }
            }
            .orEmpty()
        return WebChatConsumerState(
            streaming = current?.streaming == true,
            dictationActive = current?.dictationActive == true,
            composerSections = composerSections,
            pageKind = current?.pageKind ?: "unknown",
            pageUrl = current?.url.orEmpty(),
            features = features,
            controls = controls,
            commandRequests = observed.commandRequests.map { request ->
                WebChatConsumerCommandRequest(
                    id = request.id,
                    status = request.status.toConsumerStatus(),
                )
            },
            adapterCurrent = observed.adapterCurrent,
            dictationCaptureActive = current?.dictationCaptureActive == true,
            dictationCapturePending = current?.dictationCapturePending == true,
            draftPresent = current?.draft?.isNotBlank() == true,
            privateReadAloudReady = current?.privateReadAloudReady == true,
            privateReadAloudState = current?.privateReadAloudState ?: "idle",
            privateReadAloudContextId = current?.privateReadAloudContextId.orEmpty(),
        )
    }

    override fun requestComposerOptions(section: String): WebChatConsumerCommandResult =
        execute(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", section))

    override fun dismissComposerOptions(): WebChatConsumerCommandResult =
        execute(JSONObject().put("action", "chatgpt_dismiss_composer_options"))

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

    override fun requestControls(): WebChatConsumerCommandResult =
        execute(JSONObject().put("action", "chatgpt_refresh_controls"))

    override fun revealProjectChoice(label: String): WebChatConsumerCommandResult =
        execute(JSONObject()
            .put("action", "chatgpt_reveal_project_choice")
            .put("project_title", label))

    override fun invokeControl(
        controlId: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult = execute(JSONObject()
        .put("action", "chatgpt_invoke_control")
        .put("control_id", controlId)
        .put("user_confirmed", userConfirmed))

    override fun invokeControlAfterTouchMiss(
        controlId: String,
        userConfirmed: Boolean,
    ): WebChatConsumerCommandResult = execute(JSONObject()
        .put("action", "chatgpt_invoke_control")
        .put("control_id", controlId)
        .put("user_confirmed", userConfirmed)
        .put("after_touch_miss", true))

    override fun toggleOfficialReadAloud(contextId: String): WebChatConsumerCommandResult =
        execute(JSONObject()
            .put("action", "chatgpt_toggle_private_read_aloud")
            .put("context_id", contextId))

    override fun updateControl(
        controlId: String,
        mutation: WebChatConsumerControlMutation,
    ): WebChatConsumerCommandResult = execute(JSONObject()
        .put("control_id", controlId)
        .apply {
            when (mutation) {
                is WebChatConsumerControlMutation.Text ->
                    put("action", "chatgpt_set_control_text").put("text", mutation.value)
                is WebChatConsumerControlMutation.Choice ->
                    put("action", "chatgpt_select_control_choice").put("choice_index", mutation.index)
                is WebChatConsumerControlMutation.Slider ->
                    put("action", "chatgpt_set_control_slider").put("value", mutation.value)
                is WebChatConsumerControlMutation.Selected ->
                    put("action", "chatgpt_set_control_selected").put("selected", mutation.value)
                is WebChatConsumerControlMutation.Expanded ->
                    put("action", "chatgpt_set_control_expanded").put("expanded", mutation.value)
            }
        })

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

    private fun ChatGptNativeControlPresentation.Kind?.toConsumerPresentation() = when (this) {
        ChatGptNativeControlPresentation.Kind.DIRECT -> WebChatConsumerControlPresentation.DIRECT
        ChatGptNativeControlPresentation.Kind.DEDICATED -> WebChatConsumerControlPresentation.DEDICATED
        ChatGptNativeControlPresentation.Kind.MENU -> WebChatConsumerControlPresentation.MENU
        ChatGptNativeControlPresentation.Kind.METADATA -> WebChatConsumerControlPresentation.METADATA
        ChatGptNativeControlPresentation.Kind.OFFICIAL_FALLBACK,
        null -> WebChatConsumerControlPresentation.OFFICIAL_FALLBACK
    }

    private companion object {
        val SESSION_COMMANDS = setOf(
            "chatgpt_stop_generation",
            "chatgpt_start_dictation",
            "chatgpt_cancel_dictation",
            "chatgpt_prepare_realtime_voice",
            "chatgpt_start_realtime_voice",
            "chatgpt_submit_dictation",
            "chatgpt_regenerate_response",
        )
    }
}
