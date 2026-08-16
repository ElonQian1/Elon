package com.elon.app

import android.view.View
import android.view.LayoutInflater
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.ArrayAdapter
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptMessageClipboard
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import org.json.JSONArray
import org.json.JSONObject

internal object WebChatProductionMessageActionBinder {
    fun bind(
        itemView: View,
        message: ChatMessage,
        onAction: ((ChatMessage, WebChatMessageAction) -> Unit)?,
    ) {
        val row = itemView.findViewById<LinearLayout>(R.id.webChatMessageActionBar) ?: return
        val metadata = message.webChatMessage
        val actions = metadata?.actions.orEmpty()
        val visible = metadata != null && actions.isNotEmpty() && onAction != null
        row.visibility = if (visible) View.VISIBLE else View.GONE
        if (!visible || metadata == null) {
            row.contentDescription = null
            return
        }
        val actionHandler = onAction ?: return

        row.contentDescription = "web-chat-message-actions:${selectorId(metadata)}"
        bindButton(
            itemView,
            R.id.webChatMessageCopy,
            message,
            metadata,
            WebChatMessageAction.COPY,
            actions,
            actionHandler,
        )
        bindButton(
            itemView,
            R.id.webChatMessageRegenerate,
            message,
            metadata,
            WebChatMessageAction.REGENERATE,
            actions,
            actionHandler,
        )
        bindButton(
            itemView,
            R.id.webChatMessageMore,
            message,
            metadata,
            WebChatMessageAction.MORE,
            actions,
            actionHandler,
        )
    }

    private fun bindButton(
        itemView: View,
        id: Int,
        message: ChatMessage,
        metadata: WebChatProductionMessage,
        action: WebChatMessageAction,
        actions: Set<WebChatMessageAction>,
        onAction: ((ChatMessage, WebChatMessageAction) -> Unit),
    ) {
        val button = itemView.findViewById<ImageButton>(id) ?: return
        val available = action in actions
        button.visibility = if (available) View.VISIBLE else View.GONE
        button.contentDescription = "web-chat-message-action:${selectorId(metadata)}:${action.wireValue}"
        button.setOnClickListener(if (available) View.OnClickListener { onAction(message, action) } else null)
    }

    private fun selectorId(message: WebChatProductionMessage): String =
        "${message.providerWireValue}:${ChatGptNativeControlPresentation.stableContextId(message.sourceMessageId)}"
}

internal data class WebChatContextAction(
    val controlId: String,
    val label: String,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String,
)

internal object WebChatProductionMessageActionJson {
    fun messageContextIds(state: JSONObject): Set<String> {
        val controls = state.optJSONObject("ui_manifest")?.optJSONArray("controls") ?: return emptySet()
        return buildSet {
            controls.forEachObject { control ->
                if (
                    control.optString("region") == "message" &&
                    control.optBoolean("enabled", false) &&
                    !isPrimaryCopy(control)
                ) {
                    control.optString("context_id").takeIf(String::isNotBlank)?.let(::add)
                }
            }
        }
    }

    fun contextActions(response: JSONObject): List<WebChatContextAction> {
        val controls = response.optJSONArray("controls") ?: return emptyList()
        return buildList {
            controls.forEachObject { control ->
                val id = control.optString("control_id")
                val label = control.optString("label").trim()
                if (
                    id.isNotBlank() &&
                    label.isNotBlank() &&
                    control.optBoolean("enabled", false) &&
                    !isPrimaryCopy(control) &&
                    control.optString("native_presentation") != "official_fallback"
                ) {
                    add(
                        WebChatContextAction(
                            controlId = id,
                            label = label,
                            requiresUserConfirmation = control.optBoolean("requires_user_confirmation", false),
                            nativeSelector = "web-chat-message-context-action:" +
                                ChatGptNativeControlPresentation.stableContextId(id),
                        ),
                    )
                }
            }
        }.distinctBy(WebChatContextAction::controlId)
    }

    private fun isPrimaryCopy(control: JSONObject): Boolean =
        control.optString("semantic") == "copy" ||
            control.optString("label").trim() in setOf("复制", "Copy")

    private inline fun JSONArray.forEachObject(block: (JSONObject) -> Unit) {
        for (index in 0 until length()) optJSONObject(index)?.let(block)
    }
}

internal class WebChatProductionMessageActionCoordinator(
    private val activity: AppCompatActivity,
    private val mcpPort: () -> WebChatSocialMcpPort?,
    private val openOfficialFallback: () -> Unit,
) {
    private val clipboard = ChatGptMessageClipboard(activity)

    fun handle(message: ChatMessage, action: WebChatMessageAction) {
        val metadata = message.webChatMessage ?: return
        if (action !in metadata.actions) return
        when (action) {
            WebChatMessageAction.COPY -> clipboard.copy(message.content)
            WebChatMessageAction.REGENERATE -> dispatch(
                JSONObject().put("action", "chatgpt_regenerate_response"),
                failureMessage = "当前回答暂时不能重新生成",
            )
            WebChatMessageAction.MORE -> showMore(metadata)
        }
    }

    private fun showMore(message: WebChatProductionMessage) {
        val port = mcpPort() ?: return openOfficialFallback()
        val contextId = ChatGptNativeControlPresentation.stableContextId(message.sourceMessageId)
        val response = port.control(
            JSONObject()
                .put("action", "chatgpt_find_controls")
                .put("region", "message")
                .put("context_id", contextId)
                .put("limit", 50),
        )
        val actions = WebChatProductionMessageActionJson.contextActions(response)
        if (actions.isEmpty()) {
            Toast.makeText(activity, "当前消息没有更多可用操作", Toast.LENGTH_SHORT).show()
            return
        }
        val adapter = ContextActionAdapter(activity, actions)
        AlertDialog.Builder(activity)
            .setTitle("消息操作")
            .setAdapter(adapter) { opened, index ->
                opened.dismiss()
                actions.getOrNull(index)?.let(::confirmAndInvoke)
            }
            .setNeutralButton("官网功能") { _, _ -> openOfficialFallback() }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private class ContextActionAdapter(
        activity: AppCompatActivity,
        private val actions: List<WebChatContextAction>,
    ) : ArrayAdapter<WebChatContextAction>(activity, android.R.layout.simple_list_item_1, actions) {
        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
            val textView = (convertView ?: LayoutInflater.from(context)
                .inflate(android.R.layout.simple_list_item_1, parent, false)) as TextView
            val action = actions[position]
            textView.text = action.label
            textView.contentDescription = action.nativeSelector
            return textView
        }
    }

    private fun confirmAndInvoke(action: WebChatContextAction) {
        if (!action.requiresUserConfirmation) {
            invoke(action, userConfirmed = false)
            return
        }
        AlertDialog.Builder(activity)
            .setTitle(action.label)
            .setMessage("确认执行这个网页操作？")
            .setPositiveButton(android.R.string.ok) { _, _ -> invoke(action, userConfirmed = true) }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun invoke(action: WebChatContextAction, userConfirmed: Boolean) {
        dispatch(
            JSONObject()
                .put("action", "chatgpt_invoke_control")
                .put("control_id", action.controlId)
                .put("user_confirmed", userConfirmed),
            failureMessage = "网页操作执行失败",
        )
    }

    private fun dispatch(args: JSONObject, failureMessage: String) {
        val result = mcpPort()?.control(args)
        if (result?.optBoolean("control_ok", false) == true) return
        Toast.makeText(activity, failureMessage, Toast.LENGTH_SHORT).show()
    }
}
