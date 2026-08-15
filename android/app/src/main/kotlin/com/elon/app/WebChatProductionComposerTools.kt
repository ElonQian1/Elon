package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject

internal data class WebChatProductionComposerTool(
    val id: String,
    val label: String,
    val selected: Boolean,
    val nativeSelector: String,
)

internal object WebChatProductionComposerToolParser {
    fun parse(navigation: JSONObject): List<WebChatProductionComposerTool> {
        val options = navigation.optJSONObject("composer_sections")
            ?.optJSONArray(TOOLS_SECTION)
            ?: return emptyList()
        val parsed = mutableListOf<WebChatProductionComposerTool>()
        for (index in 0 until options.length()) {
            val option = options.optJSONObject(index) ?: continue
            val id = option.optString("id").trim()
            val label = option.optString("label").trim()
            if (id.isBlank() || label.isBlank()) continue
            parsed += WebChatProductionComposerTool(
                id = id,
                label = label,
                selected = option.optBoolean("selected"),
                nativeSelector = option.optString("native_adb_content_description")
                    .trim()
                    .ifBlank { "web-chat-composer-tool:$id" },
            )
        }
        return parsed.distinctBy(WebChatProductionComposerTool::id)
    }

    private const val TOOLS_SECTION = "tools"
}

internal class WebChatProductionComposerToolsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val mcpPort: () -> WebChatSocialMcpPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.COMPOSER_TOOLS)) return
        val port = mcpPort() ?: return showUnavailable()
        val epoch = requestEpoch
        val cached = readTools(port)
        if (cached.isNotEmpty()) {
            showToolDialog(provider, port, cached)
            return
        }
        val requested = port.control(JSONObject()
            .put("action", "chatgpt_list_composer_options")
            .put("section", TOOLS_SECTION))
        if (!requested.optBoolean("control_ok")) {
            showUnavailable()
            return
        }
        Toast.makeText(activity, "正在读取网页工具...", Toast.LENGTH_SHORT).show()
        pollTools(provider, port, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
    }

    private fun pollTools(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
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
            showUnavailable()
            return
        }
        host.postDelayed(
            { pollTools(provider, port, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readTools(port: WebChatSocialMcpPort): List<WebChatProductionComposerTool> {
        val navigation = port.control(JSONObject()
            .put("action", "chatgpt_get_navigation")
            .put("section", TOOLS_SECTION))
        if (!navigation.optBoolean("control_ok")) return emptyList()
        return WebChatProductionComposerToolParser.parse(navigation)
    }

    private fun showToolDialog(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        tools: List<WebChatProductionComposerTool>,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        val labels = tools.map { tool ->
            if (tool.selected) "${tool.label}（已选）" else tool.label
        }.toTypedArray()
        val dialog = AlertDialog.Builder(activity)
            .setTitle("网页工具")
            .setItems(labels) { _, index -> selectTool(port, tools[index]) }
            .setNegativeButton("取消", null)
            .create()
        dialog.setOnShowListener {
            dialog.listView?.apply {
                contentDescription = "web-chat-composer-tools:${provider.id.wireValue}"
                post {
                    for (childIndex in 0 until childCount) {
                        val optionIndex = firstVisiblePosition + childIndex
                        if (optionIndex in tools.indices) {
                            getChildAt(childIndex)?.contentDescription = tools[optionIndex].nativeSelector
                        }
                    }
                }
            }
        }
        dialog.show()
    }

    private fun selectTool(port: WebChatSocialMcpPort, tool: WebChatProductionComposerTool) {
        val result = port.control(JSONObject()
            .put("action", "chatgpt_select_composer_option")
            .put("section", TOOLS_SECTION)
            .put("option_id", tool.id))
        if (!result.optBoolean("control_ok")) {
            Toast.makeText(activity, "网页工具状态已变化，请重试", Toast.LENGTH_SHORT).show()
        }
    }

    private fun showUnavailable() {
        if (activity.isFinishing || activity.isDestroyed) return
        AlertDialog.Builder(activity)
            .setTitle("网页工具")
            .setMessage("当前网页没有返回可用工具，可在官方页面继续。")
            .setPositiveButton("打开官方页") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .show()
    }

    private companion object {
        const val TOOLS_SECTION = "tools"
        const val MAX_POLL_ATTEMPTS = 8
        const val POLL_INTERVAL_MS = 250L
    }
}
