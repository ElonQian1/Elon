package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class WebChatProductionPageActionsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
) {
    private var requestEpoch = 0
    private var activeDialog: AlertDialog? = null
    private var activeSheet: WebChatActionSheetHandle? = null
    private var actionById = emptyMap<String, WebChatProductionPageAction>()
    private val adaptiveControls = WebChatProductionAdaptiveControlsCoordinator(activity)

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.PAGE_ACTIONS)) {
            openOfficialFallback()
            return
        }
        val port = consumerPort() ?: return showObservationDialog(
            provider,
            WebChatProductionObservationState.SESSION_RECOVERING,
        )
        val epoch = requestEpoch
        val state = port.state()
        if (!WebChatProductionPageIdentity.from(state).hasConversationTarget) return
        val actions = readActions(provider.id, state)
        val initialObservation = observation(
            provider,
            state,
            actions,
            request = null,
            pollingExhausted = false,
        )
        if (presentActions(provider, port, state, actions, initialObservation)) return
        val requested = port.requestControls()
        if (requested.accepted) {
            pollActions(provider, port, requested, epoch, attempt = 0)
        } else {
            val latestState = port.state()
            presentActions(
                provider,
                port,
                latestState,
                actions,
                observation(provider, latestState, actions, requested, pollingExhausted = false),
            )
        }
    }

    fun cancelPending() {
        requestEpoch += 1
        activeSheet?.dismiss()
        activeSheet = null
        actionById = emptyMap()
        activeDialog?.dismiss()
        activeDialog = null
        adaptiveControls.cancel()
    }

    private fun pollActions(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        request: WebChatConsumerCommandResult,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val state = port.state()
        val pageIdentity = WebChatProductionPageIdentity.from(state)
        val observed = WebChatProductionPageActionParser.parse(
            state.controls.take(MAX_CONTROL_COUNT),
            pageIdentity,
        )
        val actions = readActions(provider.id, state)
        val exhausted = attempt >= MAX_POLL_ATTEMPTS
        val observation = observation(provider, state, actions, request, exhausted)
        val presented = presentActions(provider, port, state, actions, observation)
        if (presented || observed.isNotEmpty() || exhausted || observation in TERMINAL_OBSERVATION_STATES) {
            return
        }
        host.postDelayed(
            { pollActions(provider, port, request, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readActions(
        providerId: WebChatProviderId,
        state: WebChatConsumerState,
    ): List<WebChatProductionPageAction> =
        WebChatProductionPageActionParser.parse(
            interactionCache.controls(providerId, state.copy(
                controls = state.controls.take(MAX_CONTROL_COUNT),
            )),
            WebChatProductionPageIdentity.from(state),
        )

    private fun observation(
        provider: WebChatProviderIdentity,
        state: WebChatConsumerState,
        resolved: List<WebChatProductionPageAction>,
        request: WebChatConsumerCommandResult?,
        pollingExhausted: Boolean,
    ): WebChatProductionObservationState = WebChatProductionCapabilityEvidencePolicy.resolve(
        WebChatProductionCapabilityEvidence(
            declaredSupported = provider.supports(WebChatProviderCapability.PAGE_ACTIONS),
            adapterCurrent = state.adapterCurrent,
            observedCount = WebChatProductionPageActionParser.parse(
                state.controls.take(MAX_CONTROL_COUNT),
                WebChatProductionPageIdentity.from(state),
            ).size,
            cachedCount = if (state.controls.isEmpty()) resolved.size else 0,
            requestAccepted = request?.accepted,
            requestError = request?.error,
            requestStatus = WebChatProductionCapabilityEvidencePolicy.requestStatus(request, state),
            pollingExhausted = pollingExhausted,
        ),
    )

    private fun presentActions(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        state: WebChatConsumerState,
        actions: List<WebChatProductionPageAction>,
        observation: WebChatProductionObservationState,
    ): Boolean {
        val pageIdentity = WebChatProductionPageIdentity.from(state)
        if (pageIdentity.actionPlacement == WebChatConsumerPageActionPlacement.CONVERSATION) {
            val trigger = actions.firstOrNull { it.semantic == "conversation_options" }
            val nested = actions.filterNot { it.semantic == "conversation_options" }
            if (trigger != null && nested.none { it.control.region == "overlay" }) {
                invoke(
                    provider,
                    port,
                    trigger,
                    previousControlIds = actions.mapTo(mutableSetOf(), WebChatProductionPageAction::controlId),
                    userConfirmed = false,
                )
                return true
            }
            if (nested.isNotEmpty()) {
                showActionDialog(provider, port, pageIdentity, nested, observation)
                return true
            }
        }
        showActionDialog(provider, port, pageIdentity, actions, observation)
        return actions.isNotEmpty()
    }

    private fun showActionDialog(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        pageIdentity: WebChatProductionPageIdentity,
        actions: List<WebChatProductionPageAction>,
        observation: WebChatProductionObservationState,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        actionById = actions.associateBy(WebChatProductionPageAction::controlId)
        val previousControlIds = actionById.keys
        val availableItems = actions.map { action ->
            WebChatActionSheetItem(
                id = action.controlId,
                title = action.label,
                subtitle = if (action.officialFallback) "在官网中完成" else null,
                contentDescription = action.nativeSelector,
            )
        }
        val items = availableItems.ifEmpty {
            listOf(WebChatProductionInteractionPlaceholder.item(
                provider.id,
                surface = "page-actions",
                title = pageIdentity.sheetTitle,
                state = observation,
            ))
        }
        activeSheet?.let {
            it.updateItems(items)
            return
        }
        val sheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = pageIdentity.sheetTitle,
            items = items,
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "打开官网",
                    contentDescription = "web-chat-page-actions-official:${provider.id.wireValue}",
                    action = openOfficialFallback,
                ),
            ),
            onCancelled = { requestEpoch += 1 },
            onDismissed = {
                activeSheet = null
                actionById = emptyMap()
            },
        ) { item ->
            if (activeProvider() != provider.id) return@showUpdatable
            actionById[item.id]?.let { action ->
                selectAction(provider, port, action, previousControlIds)
            }
        }
        activeSheet = sheet
    }

    private fun selectAction(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        action: WebChatProductionPageAction,
        previousControlIds: Set<String>,
    ) {
        if (action.officialFallback) {
            openOfficialFallback()
            return
        }
        if (adaptiveControls.supports(action)) {
            val present = {
                adaptiveControls.present(port, action) {
                    refreshAfterAdaptiveMutation(provider, port)
                }
                Unit
            }
            if (action.requiresUserConfirmation) {
                showConfirmation(action, present)
            } else {
                present()
            }
            return
        }
        if (!action.requiresUserConfirmation) {
            invoke(provider, port, action, previousControlIds, userConfirmed = false)
            return
        }
        showConfirmation(action) {
            invoke(provider, port, action, previousControlIds, userConfirmed = true)
        }
    }

    private fun showConfirmation(
        action: WebChatProductionPageAction,
        onConfirmed: () -> Unit,
    ) {
        val dialog = AlertDialog.Builder(activity)
            .setTitle(action.label)
            .setMessage("确认执行这个网页操作？")
            .setPositiveButton("继续") { _, _ -> onConfirmed() }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun refreshAfterAdaptiveMutation(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
    ) {
        val epoch = requestEpoch
        host.postDelayed({
            if (epoch != requestEpoch || activeProvider() != provider.id) return@postDelayed
            val state = port.state()
            val actions = readActions(provider.id, state)
            if (actions.isNotEmpty()) {
                showActionDialog(
                    provider,
                    port,
                    WebChatProductionPageIdentity.from(state),
                    actions,
                    WebChatProductionObservationState.AVAILABLE,
                )
            }
        }, ADAPTIVE_MUTATION_SETTLE_MS)
    }

    private fun invoke(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        action: WebChatProductionPageAction,
        previousControlIds: Set<String>,
        userConfirmed: Boolean,
    ) {
        val result = port.invokeControl(action.controlId, userConfirmed)
        if (!result.accepted) {
            Toast.makeText(activity, errorMessage(result.error.orEmpty()), Toast.LENGTH_SHORT).show()
            return
        }
        if (action.semantic !in NESTED_MENU_SEMANTICS) return
        val epoch = requestEpoch
        host.post {
            if (epoch != requestEpoch || activeProvider() != provider.id) return@post
            showNestedTransition(provider, action.label)
            pollNestedActions(provider, port, epoch, previousControlIds, attempt = 0)
        }
    }

    private fun pollNestedActions(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        epoch: Int,
        previousControlIds: Set<String>,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val nested = readActions(provider.id, port.state())
            .filterNot { it.controlId in previousControlIds }
        if (nested.isNotEmpty()) {
            showActionDialog(
                provider,
                port,
                WebChatProductionPageIdentity.from(port.state()),
                nested,
                WebChatProductionObservationState.AVAILABLE,
            )
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            activeSheet?.updateItems(listOf(WebChatProductionInteractionPlaceholder.item(
                provider.id,
                surface = "nested-actions",
                title = "更多操作",
                state = WebChatProductionObservationState.TEMPORARILY_UNOBSERVED,
            )))
            return
        }
        host.postDelayed({
            pollNestedActions(provider, port, epoch, previousControlIds, attempt + 1)
        }, POLL_INTERVAL_MS)
    }

    private fun showNestedTransition(provider: WebChatProviderIdentity, label: String) {
        actionById = emptyMap()
        val transitionItems = listOf(WebChatProductionInteractionPlaceholder.item(
            provider.id,
            surface = "nested-actions",
            title = label,
        ))
        activeSheet?.let { sheet ->
            sheet.updateItems(transitionItems)
            return
        }
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = label,
            items = transitionItems,
            footerActions = listOf(WebChatActionSheetFooterAction(
                label = "打开官网",
                contentDescription = "web-chat-page-actions-official:${provider.id.wireValue}",
                action = openOfficialFallback,
            )),
            onCancelled = { requestEpoch += 1 },
            onDismissed = {
                activeSheet = null
                actionById = emptyMap()
            },
        ) { item ->
            if (activeProvider() != provider.id) return@showUpdatable
            actionById[item.id]?.let { action ->
                selectAction(provider, consumerPort() ?: return@showUpdatable, action, actionById.keys)
            }
        }
    }

    private fun showObservationDialog(
        provider: WebChatProviderIdentity,
        state: WebChatProductionObservationState,
    ) {
        if (activity.isFinishing || activity.isDestroyed) return
        val dialog = AlertDialog.Builder(activity)
            .setTitle("网页功能")
            .setMessage(WebChatProductionCapabilityEvidencePolicy.subtitle(state))
            .setNeutralButton("重试") { _, _ -> show(provider) }
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun showTracked(dialog: AlertDialog) {
        activeDialog?.dismiss()
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
        dialog.show()
    }

    private fun errorMessage(error: String): String = when (error) {
        "stale_control_id" -> "官网操作已变化，请重新打开列表"
        "user_confirmation_required" -> "需要确认后才能执行此操作"
        "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
        else -> "网页操作执行失败，请重试"
    }

    private companion object {
        const val MAX_CONTROL_COUNT = 80
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
        const val ADAPTIVE_MUTATION_SETTLE_MS = 320L
        val NESTED_MENU_SEMANTICS = setOf(
            "conversation_options",
            "profile",
            "more",
            "sources",
            "rename",
            "save_to_project",
        )
        val TERMINAL_OBSERVATION_STATES = setOf(
            WebChatProductionObservationState.REQUEST_FAILED,
            WebChatProductionObservationState.ADAPTER_UNSUPPORTED,
        )
    }
}
