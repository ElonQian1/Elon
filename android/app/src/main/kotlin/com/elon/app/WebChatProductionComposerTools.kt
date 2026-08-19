package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal data class WebChatProductionComposerTool(
    val id: String,
    val label: String,
    val semantic: String,
    val selected: Boolean,
    val nativeSelector: String,
)

internal object WebChatProductionComposerToolParser {
    fun parse(options: List<WebChatConsumerOption>): List<WebChatProductionComposerTool> {
        return options.mapNotNull { option ->
            val id = option.id.trim()
            val label = option.label.trim()
            if (id.isBlank() || label.isBlank()) return@mapNotNull null
            WebChatProductionComposerTool(
                id = id,
                label = label,
                semantic = option.semantic.trim(),
                selected = option.selected,
                nativeSelector = option.nativeSelector
                    .trim()
                    .ifBlank { "web-chat-composer-tool:$id" },
            )
        }
            .distinctBy(WebChatProductionComposerTool::id)
    }
}

internal enum class WebChatProductionQuickActionSyncOutcome {
    KEEP_WAITING,
    RETRY_DISCOVERY,
    RETRY_LATER,
}

internal object WebChatProductionQuickActionSyncPolicy {
    fun resolve(
        requestStatus: WebChatConsumerCommandStatus?,
        attemptsExhausted: Boolean,
        discoveryRound: Int,
        maxDiscoveryRounds: Int,
    ): WebChatProductionQuickActionSyncOutcome = when {
        requestStatus == WebChatConsumerCommandStatus.FAILED ||
            requestStatus == WebChatConsumerCommandStatus.TIMED_OUT ->
            WebChatProductionQuickActionSyncOutcome.RETRY_LATER
        !attemptsExhausted -> WebChatProductionQuickActionSyncOutcome.KEEP_WAITING
        requestStatus == WebChatConsumerCommandStatus.SUCCEEDED &&
            discoveryRound < maxDiscoveryRounds ->
            WebChatProductionQuickActionSyncOutcome.RETRY_DISCOVERY
        else -> WebChatProductionQuickActionSyncOutcome.RETRY_LATER
    }
}

internal class WebChatProductionComposerToolsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
    private val onOperationFeedback: (WebChatConsumerComposerFeedback) -> Unit,
    private val onQuickActionChanged: (WebChatProductionQuickComposerAction?) -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
    private val sessionReady: () -> Boolean,
    private val requestSessionRecovery: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null
    private var pendingQuickAction: PendingQuickAction? = null

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        val actions = quickActions(provider)
        if (actions.isEmpty()) return showUnavailable()
        val selected = selectedQuickAction(provider)
        val actionById = actions.associateBy { "quick:${it.semantic}" }
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "工具",
            items = actions.map { action ->
                WebChatActionSheetItem(
                    id = "quick:${action.semantic}",
                    title = action.label,
                    subtitle = if (action == selected) "已启用" else null,
                    selected = action == selected,
                    contentDescription = "web-chat-composer-tool:${provider.id.wireValue}:${action.semantic}",
                )
            },
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网完整功能",
                    contentDescription = "web-chat-composer-tools-official:${provider.id.wireValue}",
                    action = openOfficialFallback,
                ),
            ),
            onCancelled = { requestEpoch += 1 },
            onDismissed = { activeSheet = null },
        ) { item ->
            if (activeProvider() != provider.id) return@showUpdatable
            actionById[item.id]?.let { selectQuickAction(provider, it) }
        }
    }

    fun quickActions(provider: WebChatProviderIdentity): List<WebChatProductionQuickComposerAction> =
        WebChatProductionQuickComposerActionCatalog.availableFor(provider)

    fun selectedQuickAction(provider: WebChatProviderIdentity): WebChatProductionQuickComposerAction? {
        if (activeProvider() != provider.id) return null
        return consumerPort()
            ?.let(::observedToolOptions)
            ?.let(WebChatProductionComposerToolParser::parse)
            ?.asSequence()
            ?.filter(WebChatProductionComposerTool::selected)
            ?.mapNotNull(WebChatProductionQuickComposerActionResolver::actionFor)
            ?.firstOrNull()
    }

    fun selectQuickAction(
        provider: WebChatProviderIdentity,
        action: WebChatProductionQuickComposerAction,
    ): Boolean = changeQuickAction(provider, action, desiredActive = true)

    fun clearQuickAction(
        provider: WebChatProviderIdentity,
        action: WebChatProductionQuickComposerAction,
    ): Boolean = changeQuickAction(provider, action, desiredActive = false)

    fun onSessionStateChanged(provider: WebChatProviderIdentity) {
        val pending = pendingQuickAction ?: return
        if (pending.providerId != provider.id || activeProvider() != provider.id) return
        if (!sessionReady() || pending.requestInFlight) return
        startPendingRequest(pending)
    }

    private fun changeQuickAction(
        provider: WebChatProviderIdentity,
        action: WebChatProductionQuickComposerAction,
        desiredActive: Boolean,
    ): Boolean {
        cancelPending()
        if (activeProvider() != provider.id || action !in quickActions(provider)) return false
        val pending = PendingQuickAction(
            providerId = provider.id,
            action = action,
            desiredActive = desiredActive,
            epoch = requestEpoch,
        )
        pendingQuickAction = pending
        if (!sessionReady()) {
            requestSessionRecovery()
            showQuickActionQueued(action)
            return true
        }
        return startPendingRequest(pending)
    }

    private fun startPendingRequest(
        pending: PendingQuickAction,
    ): Boolean {
        if (pending.epoch != requestEpoch || pendingQuickAction != pending) return false
        val port = consumerPort() ?: run {
            failPendingQuickAction(pending)
            return false
        }
        val requested = port.requestComposerOptions(TOOLS_SECTION)
        if (!requested.accepted) {
            failPendingQuickAction(pending)
            return false
        }
        pendingQuickAction = pending.copy(requestInFlight = true)
        host.postDelayed(
            {
                pollQuickAction(
                    port,
                    pending.copy(requestInFlight = true),
                    requested.requestId,
                    attempt = 0,
                )
            },
            QUICK_ACTION_INITIAL_DELAY_MS,
        )
        return true
    }

    fun startRealtimeVoice(provider: WebChatProviderIdentity): Boolean {
        cancelPending()
        if (activeProvider() != provider.id) return false
        openOfficialFallback()
        return true
    }

    fun cancelPending() {
        requestEpoch += 1
        pendingQuickAction = null
        activeSheet?.dismiss()
        activeSheet = null
    }

    private fun pollQuickAction(
        port: WebChatConsumerPort,
        pending: PendingQuickAction,
        requestId: String?,
        attempt: Int,
    ) {
        if (
            pending.epoch != requestEpoch ||
            activeProvider() != pending.providerId ||
            pendingQuickAction != pending
        ) return
        if (!sessionReady()) {
            port.dismissComposerOptions()
            pendingQuickAction = pending.copy(requestInFlight = false)
            requestSessionRecovery()
            return
        }
        val observed = observedToolOptions(port)
        if (observed.isNotEmpty()) {
            interactionCache.composerOptions(pending.providerId, TOOLS_SECTION, observed)
            val tools = WebChatProductionComposerToolParser.parse(observed)
            val target = WebChatProductionQuickComposerActionResolver.find(pending.action, tools)
            if (target != null) {
                if (target.selected == pending.desiredActive) {
                    port.dismissComposerOptions()
                    completePendingQuickAction(pending)
                } else if (applyTool(
                        WebChatProviderRegistry.get(pending.providerId),
                        port,
                        target,
                    )) {
                    completePendingQuickAction(pending)
                } else {
                    pendingQuickAction = null
                }
                return
            }
        }
        val requestStatus = requestId?.let { id ->
            port.state().commandRequests.firstOrNull { it.id == id }?.status
        }
        when (WebChatProductionQuickActionSyncPolicy.resolve(
            requestStatus = requestStatus,
            attemptsExhausted = attempt >= MAX_POLL_ATTEMPTS,
            discoveryRound = pending.discoveryRound,
            maxDiscoveryRounds = MAX_DISCOVERY_ROUNDS,
        )) {
            WebChatProductionQuickActionSyncOutcome.KEEP_WAITING -> Unit
            WebChatProductionQuickActionSyncOutcome.RETRY_DISCOVERY -> {
                port.dismissComposerOptions()
                val next = pending.copy(
                    discoveryRound = pending.discoveryRound + 1,
                    requestInFlight = false,
                )
                pendingQuickAction = next
                host.postDelayed(
                    { if (sessionReady()) startPendingRequest(next) },
                    DISCOVERY_RETRY_DELAY_MS,
                )
                return
            }
            WebChatProductionQuickActionSyncOutcome.RETRY_LATER -> {
                port.dismissComposerOptions()
                failPendingQuickAction(pending)
                return
            }
        }
        host.postDelayed(
            {
                pollQuickAction(
                    port,
                    pending,
                    requestId,
                    attempt + 1,
                )
            },
            POLL_INTERVAL_MS,
        )
    }

    private fun observedToolOptions(port: WebChatConsumerPort): List<WebChatConsumerOption> =
        port.state().composerSections[TOOLS_SECTION].orEmpty()

    private fun applyTool(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        tool: WebChatProductionComposerTool,
    ): Boolean {
        val result = port.selectComposerOption(TOOLS_SECTION, tool.id)
        if (!result.accepted) {
            Toast.makeText(activity, "网页工具状态已变化，请重试", Toast.LENGTH_SHORT).show()
        } else {
            onOperationFeedback(WebChatConsumerComposerOperationPolicy.toolAccepted(provider, tool.label))
        }
        return result.accepted
    }

    private fun showUnavailable() {
        if (activity.isFinishing || activity.isDestroyed) return
        androidx.appcompat.app.AlertDialog.Builder(activity)
            .setTitle("网页工具")
            .setMessage("当前网页没有返回可用工具，可在官方页面继续。")
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun showQuickActionQueued(action: WebChatProductionQuickComposerAction) {
        Toast.makeText(
            activity,
            "已保留${action.label}选择，网页连接后自动开启",
            Toast.LENGTH_SHORT,
        ).show()
    }

    private fun completePendingQuickAction(pending: PendingQuickAction) {
        if (pendingQuickAction != pending) return
        pendingQuickAction = null
        onQuickActionChanged(pending.action.takeIf { pending.desiredActive })
    }

    private fun failPendingQuickAction(pending: PendingQuickAction) {
        if (pending.epoch != requestEpoch) return
        pendingQuickAction = null
        showQuickActionRetry(pending.action)
    }

    private fun showQuickActionRetry(action: WebChatProductionQuickComposerAction) {
        Toast.makeText(
            activity,
            "${action.label}暂时未能启用，请稍后重试",
            Toast.LENGTH_SHORT,
        ).show()
    }

    private data class PendingQuickAction(
        val providerId: WebChatProviderId,
        val action: WebChatProductionQuickComposerAction,
        val desiredActive: Boolean,
        val epoch: Int,
        val discoveryRound: Int = 0,
        val requestInFlight: Boolean = false,
    )

    private companion object {
        const val TOOLS_SECTION = "tools"
        const val MAX_POLL_ATTEMPTS = 8
        const val MAX_DISCOVERY_ROUNDS = 2
        const val POLL_INTERVAL_MS = 250L
        const val QUICK_ACTION_INITIAL_DELAY_MS = 180L
        const val DISCOVERY_RETRY_DELAY_MS = 600L
    }
}
