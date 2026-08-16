package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebUiControl
import com.google.android.material.bottomsheet.BottomSheetDialog
import org.json.JSONObject

internal data class WebChatProductionPageAction(
    val control: ChatGptWebUiControl,
    val requiresUserConfirmation: Boolean,
    val officialFallback: Boolean,
    val nativeSelector: String,
) {
    val controlId: String get() = control.id
    val label: String get() = control.label
    val semantic: String get() = control.semantic
}

internal object WebChatProductionPageActionParser {
    fun parse(response: JSONObject): List<WebChatProductionPageAction> {
        val controls = response.optJSONArray("controls") ?: return emptyList()
        val parsed = mutableListOf<WebChatProductionPageAction>()
        for (index in 0 until controls.length()) {
            val raw = controls.optJSONObject(index) ?: continue
            val control = WebChatProductionControlJson.parse(raw) ?: continue
            val presentation = raw.optString("native_presentation").trim().lowercase()
            if (
                !control.enabled ||
                control.region !in PAGE_REGIONS ||
                control.semantic in EXCLUDED_SEMANTICS ||
                presentation !in SUPPORTED_PRESENTATIONS
            ) continue
            parsed += WebChatProductionPageAction(
                control = control,
                requiresUserConfirmation = raw.optBoolean("requires_user_confirmation", false),
                officialFallback = presentation == "official_fallback" ||
                    control.semantic in OFFICIAL_COMPLETION_SEMANTICS,
                nativeSelector = raw.optString("native_adb_content_description")
                    .trim()
                    .ifBlank { raw.optString("adb_content_description").trim() }
                    .ifBlank { "web-chat-page-action:${control.semantic}:${control.id}" },
            )
        }
        return parsed.distinctBy(WebChatProductionPageAction::controlId)
    }

    private val PAGE_REGIONS = setOf("header", "content", "suggestions", "overlay")
    private val SUPPORTED_PRESENTATIONS = setOf("direct", "dedicated", "menu", "official_fallback")
    private val EXCLUDED_SEMANTICS = setOf(
        "navigation",
        "title",
        "send",
        "stop",
        "copy",
        "regenerate",
        "model",
        "attachment",
        "dictation",
        "voice_mode",
        "search",
        "conversation",
        "timestamp",
    )
    private val OFFICIAL_COMPLETION_SEMANTICS = setOf(
        "conversation_files",
        "share",
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
    private var activeSheet: BottomSheetDialog? = null
    private val adaptiveControls = WebChatProductionAdaptiveControlsCoordinator(activity)

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
        activeSheet?.dismiss()
        activeSheet = null
        activeDialog?.dismiss()
        activeDialog = null
        adaptiveControls.cancel()
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
        val byId = actions.associateBy(WebChatProductionPageAction::controlId)
        val previousControlIds = byId.keys
        val sheet = WebChatActionSheet.show(
            activity = activity,
            title = "当前网页操作",
            items = actions.map { action ->
                WebChatActionSheetItem(
                    id = action.controlId,
                    title = action.label,
                    subtitle = if (action.officialFallback) "在官网中完成" else null,
                    contentDescription = action.nativeSelector,
                )
            },
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网完整功能",
                    contentDescription = "web-chat-page-actions-official:${provider.id.wireValue}",
                    action = openOfficialFallback,
                ),
            ),
        ) { item ->
            if (activeProvider() != provider.id) return@show
            byId[item.id]?.let { action ->
                selectAction(provider, port, action, previousControlIds)
            }
        }
        activeSheet = sheet
        sheet?.setOnDismissListener {
            if (activeSheet === sheet) activeSheet = null
        }
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
        port: WebChatSocialMcpPort,
    ) {
        val epoch = requestEpoch
        host.postDelayed({
            if (epoch != requestEpoch || activeProvider() != provider.id) return@postDelayed
            val actions = readActions(port)
            if (actions.isNotEmpty()) showActionDialog(provider, port, actions)
        }, ADAPTIVE_MUTATION_SETTLE_MS)
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
        const val ADAPTIVE_MUTATION_SETTLE_MS = 320L
        val NESTED_MENU_SEMANTICS = setOf(
            "conversation_options",
            "profile",
            "more",
            "sources",
            "rename",
            "save_to_project",
        )
    }
}
