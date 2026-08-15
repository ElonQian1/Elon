package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject

internal data class WebChatProductionPageAction(
    val controlId: String,
    val label: String,
    val semantic: String,
    val requiresUserConfirmation: Boolean,
    val officialFallback: Boolean,
    val nativeSelector: String,
)

internal object WebChatProductionPageActionParser {
    fun parse(response: JSONObject): List<WebChatProductionPageAction> {
        val controls = response.optJSONArray("controls") ?: return emptyList()
        val parsed = mutableListOf<WebChatProductionPageAction>()
        for (index in 0 until controls.length()) {
            val control = controls.optJSONObject(index) ?: continue
            val controlId = control.optString("control_id").trim()
            val label = control.optString("label").trim()
            val semantic = control.optString("semantic").trim().lowercase()
            val region = control.optString("region").trim().lowercase()
            val presentation = control.optString("native_presentation").trim().lowercase()
            if (
                controlId.isBlank() || label.isBlank() ||
                !control.optBoolean("enabled", false) ||
                region !in PAGE_REGIONS ||
                semantic.isBlank() ||
                semantic in EXCLUDED_SEMANTICS ||
                presentation !in SUPPORTED_PRESENTATIONS
            ) continue
            parsed += WebChatProductionPageAction(
                controlId = controlId,
                label = label,
                semantic = semantic,
                requiresUserConfirmation = control.optBoolean("requires_user_confirmation", false),
                officialFallback = presentation == "official_fallback" ||
                    semantic in OFFICIAL_COMPLETION_SEMANTICS,
                nativeSelector = control.optString("native_adb_content_description")
                    .trim()
                    .ifBlank { control.optString("adb_content_description").trim() }
                    .ifBlank { "web-chat-page-action:$semantic:$controlId" },
            )
        }
        return parsed.distinctBy(WebChatProductionPageAction::controlId)
    }

    private val PAGE_REGIONS = setOf("header", "content", "suggestions", "overlay")
    private val SUPPORTED_PRESENTATIONS = setOf("direct", "dedicated", "menu", "official_fallback")
    private val EXCLUDED_SEMANTICS = setOf(
        "navigation",
        "title",
        "close",
        "confirm",
        "send",
        "stop",
        "copy",
        "regenerate",
        "model",
        "attachment",
        "dictation",
        "voice_mode",
        "text_input",
        "selection",
        "toggle",
        "slider",
        "search",
        "conversation",
        "timestamp",
    )
    private val OFFICIAL_COMPLETION_SEMANTICS = setOf(
        "conversation_files",
        "rename",
        "share",
        "delete",
        "save_to_project",
        "create_asset",
        "open_media",
        "personalization",
        "plan",
        "logout",
    )
}

internal class WebChatProductionPageActionsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val mcpPort: () -> WebChatSocialMcpPort?,
    private val activeProvider: () -> WebChatProviderId?,
    private val openOfficialFallback: () -> Unit,
) {
    private var requestEpoch = 0
    private var activeDialog: AlertDialog? = null

    fun show(provider: WebChatProviderIdentity) {
        cancelPending()
        if (!provider.supports(WebChatProviderCapability.PAGE_ACTIONS)) {
            openOfficialFallback()
            return
        }
        val port = mcpPort() ?: return showUnavailable()
        val epoch = requestEpoch
        val cached = readActions(port)
        if (cached.isNotEmpty()) {
            showActionDialog(provider, port, cached)
            return
        }
        val requested = port.control(JSONObject().put("action", "chatgpt_refresh_controls"))
        if (!requested.optBoolean("control_ok")) return showUnavailable()
        Toast.makeText(activity, "正在读取当前网页操作...", Toast.LENGTH_SHORT).show()
        pollActions(provider, port, epoch, attempt = 0)
    }

    fun cancelPending() {
        requestEpoch += 1
        activeDialog?.dismiss()
        activeDialog = null
    }

    private fun pollActions(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        epoch: Int,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val actions = readActions(port)
        if (actions.isNotEmpty()) {
            showActionDialog(provider, port, actions)
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) return showUnavailable()
        host.postDelayed(
            { pollActions(provider, port, epoch, attempt + 1) },
            POLL_INTERVAL_MS,
        )
    }

    private fun readActions(port: WebChatSocialMcpPort): List<WebChatProductionPageAction> {
        val response = port.control(JSONObject()
            .put("action", "chatgpt_find_controls")
            .put("limit", MAX_CONTROL_COUNT))
        if (!response.optBoolean("control_ok")) return emptyList()
        return WebChatProductionPageActionParser.parse(response)
    }

    private fun showActionDialog(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        actions: List<WebChatProductionPageAction>,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeProvider() != provider.id) return
        if (actions.isEmpty()) return showUnavailable()
        val labels = actions.map { action ->
            if (action.officialFallback) "${action.label}（官网）" else action.label
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("当前网页操作")
            .setItems(labels.toTypedArray()) { _, index ->
                actions.getOrNull(index)?.let { action ->
                    selectAction(provider, port, action, actions.mapTo(mutableSetOf()) { it.controlId })
                }
            }
            .setNeutralButton("官网完整功能") { _, _ -> openOfficialFallback() }
            .setNegativeButton("取消", null)
            .create()
        dialog.setOnShowListener {
            dialog.listView?.apply {
                contentDescription = "web-chat-page-actions:${provider.id.wireValue}"
                post {
                    for (childIndex in 0 until childCount) {
                        val actionIndex = firstVisiblePosition + childIndex
                        getChildAt(childIndex)?.contentDescription =
                            actions.getOrNull(actionIndex)?.nativeSelector
                    }
                }
            }
        }
        showTracked(dialog)
    }

    private fun selectAction(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        action: WebChatProductionPageAction,
        previousControlIds: Set<String>,
    ) {
        if (action.officialFallback) {
            openOfficialFallback()
            return
        }
        if (!action.requiresUserConfirmation) {
            invoke(provider, port, action, previousControlIds, userConfirmed = false)
            return
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(action.label)
            .setMessage("确认执行这个网页操作？")
            .setPositiveButton("继续") { _, _ ->
                invoke(provider, port, action, previousControlIds, userConfirmed = true)
            }
            .setNegativeButton("取消", null)
            .create()
        showTracked(dialog)
    }

    private fun invoke(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        action: WebChatProductionPageAction,
        previousControlIds: Set<String>,
        userConfirmed: Boolean,
    ) {
        val result = port.control(JSONObject()
            .put("action", "chatgpt_invoke_control")
            .put("control_id", action.controlId)
            .put("user_confirmed", userConfirmed))
        if (!result.optBoolean("control_ok")) {
            Toast.makeText(activity, errorMessage(result.optString("error")), Toast.LENGTH_SHORT).show()
            return
        }
        if (action.semantic !in NESTED_MENU_SEMANTICS) return
        val epoch = requestEpoch
        Toast.makeText(activity, "正在读取${action.label}...", Toast.LENGTH_SHORT).show()
        pollNestedActions(provider, port, epoch, previousControlIds, attempt = 0)
    }

    private fun pollNestedActions(
        provider: WebChatProviderIdentity,
        port: WebChatSocialMcpPort,
        epoch: Int,
        previousControlIds: Set<String>,
        attempt: Int,
    ) {
        if (epoch != requestEpoch || activeProvider() != provider.id) return
        val nested = readActions(port).filterNot { it.controlId in previousControlIds }
        if (nested.isNotEmpty()) {
            showActionDialog(provider, port, nested)
            return
        }
        if (attempt >= MAX_POLL_ATTEMPTS) return
        host.postDelayed({
            pollNestedActions(provider, port, epoch, previousControlIds, attempt + 1)
        }, POLL_INTERVAL_MS)
    }

    private fun showUnavailable() {
        if (activity.isFinishing || activity.isDestroyed) return
        val dialog = AlertDialog.Builder(activity)
            .setTitle("当前网页操作")
            .setMessage("当前网页没有返回可用操作，可在官方页面继续。")
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
        val NESTED_MENU_SEMANTICS = setOf("conversation_options", "profile", "more", "sources")
    }
}
