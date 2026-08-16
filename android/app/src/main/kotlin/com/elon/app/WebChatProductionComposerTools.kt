package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.bottomsheet.BottomSheetDialog

internal data class WebChatProductionComposerTool(
    val id: String,
    val label: String,
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
) {
    private var requestEpoch = 0
    private var activeSheet: BottomSheetDialog? = null

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.COMPOSER_TOOLS)) return
        val port = consumerPort() ?: return showUnavailable()
        val epoch = requestEpoch
        val cached = readTools(port)
        if (cached.isNotEmpty()) {
            showToolDialog(provider, port, cached)
            return
        }
        val requested = port.requestComposerOptions(TOOLS_SECTION)
        if (!requested.accepted) {
            showToolDialog(provider, port, emptyList())
            return
        }
        Toast.makeText(activity, "正在读取网页工具...", Toast.LENGTH_SHORT).show()
        pollTools(provider, port, epoch, attempt = 0)
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

    private fun pollTools(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val tools = readTools(port)
        if (tools.isNotEmpty()) {
            showToolDialog(provider, port, tools)
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) {
            showToolDialog(provider, port, emptyList())
            return
        }
        host.postDelayed(
            { pollTools(provider, port, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readTools(port: WebChatConsumerPort): List<WebChatProductionComposerTool> =
        WebChatProductionComposerToolParser.parse(
            port.state().composerSections[TOOLS_SECTION].orEmpty(),
        )

    private fun showToolDialog(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        tools: List<WebChatProductionComposerTool>,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        val state = port.state()
        val commands = WebChatProductionComposerCommandCatalog.resolve(
            provider = provider,
            streaming = state.streaming,
            dictationActive = state.dictationActive,
        )
        if (commands.isEmpty() && tools.isEmpty()) {
            showUnavailable()
            return
        }
        val commandById = commands.associateBy { "command:${it.action}" }
        val toolById = tools.associateBy { "tool:${it.id}" }
        val items = commands.map { command ->
            WebChatActionSheetItem(
                id = "command:${command.action}",
                title = command.label,
                subtitle = "当前会话操作",
                contentDescription = command.nativeSelector,
            )
        } + tools.map { tool ->
            WebChatActionSheetItem(
                id = "tool:${tool.id}",
                title = tool.label,
                subtitle = if (tool.selected) "已启用" else null,
                selected = tool.selected,
                contentDescription = tool.nativeSelector,
            )
        }
        val sheet = WebChatActionSheet.show(
            activity = activity,
            title = "网页功能",
            items = items,
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网完整功能",
                    contentDescription = "web-chat-composer-tools-official:${provider.id.wireValue}",
                    action = openOfficialFallback,
                ),
            ),
        ) { item ->
            if (activeProvider() != provider.id) return@show
            commandById[item.id]?.let { executeCommand(provider, port, it); return@show }
            toolById[item.id]?.let { selectTool(provider, port, it) }
        }
        activeSheet = sheet
        sheet?.setOnDismissListener {
            if (activeSheet === sheet) activeSheet = null
        }
    }

    private fun executeCommand(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        command: WebChatProductionComposerCommand,
    ): Boolean {
        if (command.action == REALTIME_VOICE_ACTION) {
            openOfficialFallback()
            return true
        }
        val result = port.executeSessionCommand(command.action)
        val accepted = result.accepted
        if (!accepted) {
            showCommandError(result.error.orEmpty())
        } else {
            WebChatConsumerComposerOperationPolicy.commandAccepted(provider, command.action)
                ?.let(onOperationFeedback)
        }
        return accepted
    }

    private fun selectTool(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        tool: WebChatProductionComposerTool,
    ) {
        val result = port.selectComposerOption(TOOLS_SECTION, tool.id)
        if (!result.accepted) {
            Toast.makeText(activity, "网页工具状态已变化，请重试", Toast.LENGTH_SHORT).show()
        } else {
            onOperationFeedback(WebChatConsumerComposerOperationPolicy.toolAccepted(provider, tool.label))
        }
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

    private fun showCommandError(error: String) {
        val message = when (error) {
            "dictation_unavailable" -> "当前网页暂不支持听写"
            "realtime_voice_unavailable" -> "当前网页暂不支持实时语音"
            "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
            else -> "网页功能状态已变化，请重试"
        }
        Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
    }

    private companion object {
        const val TOOLS_SECTION = "tools"
        const val REALTIME_VOICE_ACTION = "chatgpt_start_realtime_voice"
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
    }
}
