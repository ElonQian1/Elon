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

internal class WebChatProductionComposerToolsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
    private val onOperationFeedback: (WebChatConsumerComposerFeedback) -> Unit,
    private val onQuickActionChanged: (WebChatProductionQuickComposerAction?) -> Unit,
    private val interactionCache: WebChatProductionInteractionCache,
) {
    private var requestEpoch = 0
    private var activeSheet: WebChatActionSheetHandle? = null

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

    private fun changeQuickAction(
        provider: WebChatProviderIdentity,
        action: WebChatProductionQuickComposerAction,
        desiredActive: Boolean,
    ): Boolean {
        cancelPending()
        if (activeProvider() != provider.id || action !in quickActions(provider)) return false
        val port = consumerPort() ?: run {
            showQuickActionUnavailable(action)
            return false
        }
        val requested = port.requestComposerOptions(TOOLS_SECTION)
        if (!requested.accepted) {
            showQuickActionUnavailable(action)
            return false
        }
        val epoch = requestEpoch
        host.postDelayed(
            { pollQuickAction(provider, port, action, desiredActive, epoch, attempt = 0) },
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
        activeSheet?.dismiss()
        activeSheet = null
    }

    private fun pollQuickAction(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        action: WebChatProductionQuickComposerAction,
        desiredActive: Boolean,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val observed = observedToolOptions(port)
        if (observed.isNotEmpty()) {
            interactionCache.composerOptions(provider.id, TOOLS_SECTION, observed)
            val tools = WebChatProductionComposerToolParser.parse(observed)
            val target = WebChatProductionQuickComposerActionResolver.find(action, tools)
            if (target != null) {
                if (target.selected == desiredActive) {
                    port.dismissComposerOptions()
                    onQuickActionChanged(action.takeIf { desiredActive })
                } else if (applyTool(provider, port, target)) {
                    onQuickActionChanged(action.takeIf { desiredActive })
                }
                return
            }
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            port.dismissComposerOptions()
            showQuickActionUnavailable(action)
            return
        }
        host.postDelayed(
            { pollQuickAction(provider, port, action, desiredActive, epoch, attempt + 1) },
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

    private fun showQuickActionUnavailable(action: WebChatProductionQuickComposerAction) {
        Toast.makeText(
            activity,
            "当前官网暂未提供${action.label}，可在官网完整功能中查看",
            Toast.LENGTH_SHORT,
        ).show()
    }

    private companion object {
        const val TOOLS_SECTION = "tools"
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
        const val QUICK_ACTION_INITIAL_DELAY_MS = 180L
    }
}
