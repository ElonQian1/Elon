package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptNativeChoiceControlDialog
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import com.elon.app.chatgptweb.ChatGptNativeFormControlDialog
import com.elon.app.chatgptweb.ChatGptNativeSliderControlDialog
import com.elon.app.chatgptweb.ChatGptWebSlider
import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.json.JSONArray
import org.json.JSONObject

internal object WebChatProductionControlJson {
    fun parse(value: JSONObject): ChatGptWebUiControl? {
        val id = value.optString("control_id").trim()
        val semantic = value.optString("semantic").trim().lowercase()
        val label = value.optString("label").trim()
        val region = value.optString("region").trim().lowercase()
        val role = value.optString("role").trim().lowercase()
        if (id.isBlank() || semantic.isBlank() || label.isBlank() || region.isBlank()) return null
        val choices = value.optJSONArray("choice_labels").strings()
        val choiceIndex = (value.opt("selected_choice_index") as? Number)
            ?.toInt()
            ?.takeIf { it in choices.indices }
        return ChatGptWebUiControl(
            id = id,
            semantic = semantic,
            label = label,
            region = region,
            role = role,
            enabled = value.optBoolean("enabled", false),
            selected = value.optBoolean("selected", false),
            inputKind = value.optionalString("input_kind"),
            writable = value.optBoolean("writable", false),
            stateSettable = value.optBoolean("state_settable", false),
            choiceLabels = choices,
            selectedChoiceIndex = choiceIndex,
            slider = value.optJSONObject("slider")?.let(::parseSlider),
            expanded = value.opt("expanded") as? Boolean,
            expandable = value.optBoolean("expandable", false),
            contextId = value.optionalString("context_id"),
            inViewport = value.optBoolean("in_viewport", true),
            webXRatio = value.optionalFiniteDouble("web_x_ratio"),
            webYRatio = value.optionalFiniteDouble("web_y_ratio"),
        )
    }

    private fun parseSlider(value: JSONObject): ChatGptWebSlider? {
        val min = value.optionalFiniteDouble("min") ?: return null
        val max = value.optionalFiniteDouble("max") ?: return null
        val step = value.optionalFiniteDouble("step") ?: return null
        val current = value.optionalFiniteDouble("value") ?: return null
        val stepCount = (max - min) / step
        if (
            max <= min ||
            step <= 0.0 ||
            current !in min..max ||
            !stepCount.isFinite() ||
            stepCount > MAX_NATIVE_SLIDER_STEPS
        ) return null
        return ChatGptWebSlider(min = min, max = max, step = step, value = current)
    }

    private fun JSONObject.optionalString(name: String): String? {
        if (!has(name) || isNull(name)) return null
        return optString(name).trim().takeIf(String::isNotBlank)
    }

    private fun JSONObject.optionalFiniteDouble(name: String): Double? =
        (opt(name) as? Number)?.toDouble()?.takeIf(Double::isFinite)

    private fun JSONArray?.strings(): List<String> {
        if (this == null) return emptyList()
        return buildList {
            for (index in 0 until length()) {
                if (isNull(index)) continue
                optString(index).trim().takeIf(String::isNotBlank)?.let(::add)
            }
        }
    }

    private const val MAX_NATIVE_SLIDER_STEPS = 10_000.0
}

internal object WebChatProductionAdaptiveControlSelectors {
    fun stateList(controlId: String): String =
        "web-chat-control-state:${stable(controlId)}"

    fun stateValue(controlId: String, value: Boolean): String =
        "web-chat-control-state:${stable(controlId)}:$value"

    fun expansionList(controlId: String): String =
        "web-chat-control-expanded:${stable(controlId)}"

    fun expansionValue(controlId: String, value: Boolean): String =
        "web-chat-control-expanded:${stable(controlId)}:$value"

    private fun stable(value: String): String =
        ChatGptNativeControlPresentation.stableContextId(value)
}

internal class WebChatProductionAdaptiveControlsCoordinator(
    private val activity: AppCompatActivity,
) {
    private var activeDialog: AlertDialog? = null

    fun supports(action: WebChatProductionPageAction): Boolean = action.control.let { control ->
        control.supportsTextEntry ||
            control.supportsChoiceSelection ||
            control.supportsSliderValue ||
            control.supportsSelectedState ||
            control.supportsExpandedState
    }

    fun present(
        port: WebChatSocialMcpPort,
        action: WebChatProductionPageAction,
        onUpdated: () -> Unit,
    ): Boolean {
        val control = action.control
        when {
            control.supportsTextEntry -> showText(port, control, onUpdated)
            control.supportsChoiceSelection -> showChoice(port, control, onUpdated)
            control.supportsSliderValue -> showSlider(port, control, onUpdated)
            control.supportsSelectedState -> showSelectedState(port, control, onUpdated)
            control.supportsExpandedState -> showExpandedState(port, control, onUpdated)
            else -> return false
        }
        return true
    }

    fun cancel() {
        activeDialog?.dismiss()
        activeDialog = null
    }

    private fun showText(
        port: WebChatSocialMcpPort,
        control: ChatGptWebUiControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeFormControlDialog.show(activity, control) { controlId, text ->
            dispatch(port, JSONObject()
                .put("action", "chatgpt_set_control_text")
                .put("control_id", controlId)
                .put("text", text), onUpdated)
        })
    }

    private fun showChoice(
        port: WebChatSocialMcpPort,
        control: ChatGptWebUiControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeChoiceControlDialog.show(activity, control) { controlId, index ->
            dispatch(port, JSONObject()
                .put("action", "chatgpt_select_control_choice")
                .put("control_id", controlId)
                .put("choice_index", index), onUpdated)
        })
    }

    private fun showSlider(
        port: WebChatSocialMcpPort,
        control: ChatGptWebUiControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeSliderControlDialog.show(activity, control) { controlId, value ->
            dispatch(port, JSONObject()
                .put("action", "chatgpt_set_control_slider")
                .put("control_id", controlId)
                .put("value", value), onUpdated)
        })
    }

    private fun showSelectedState(
        port: WebChatSocialMcpPort,
        control: ChatGptWebUiControl,
        onUpdated: () -> Unit,
    ) {
        val values = if (control.role in SINGLE_SELECT_ROLES) listOf(true) else listOf(true, false)
        val labels = if (values.size == 1) arrayOf("选择") else arrayOf("开启", "关闭")
        val checked = values.indexOf(control.selected)
        val dialog = AlertDialog.Builder(activity)
            .setTitle(control.label)
            .setSingleChoiceItems(labels, checked) { opened, index ->
                val selected = values[index]
                dispatch(port, JSONObject()
                    .put("action", "chatgpt_set_control_selected")
                    .put("control_id", control.id)
                    .put("selected", selected), onUpdated)
                opened.dismiss()
            }
            .setNegativeButton(android.R.string.cancel, null)
            .create()
        dialog.setOnShowListener {
            dialog.listView?.apply {
                contentDescription = WebChatProductionAdaptiveControlSelectors.stateList(control.id)
                post {
                    for (index in 0 until childCount) {
                        val value = values.getOrNull(firstVisiblePosition + index) ?: continue
                        getChildAt(index)?.contentDescription =
                            WebChatProductionAdaptiveControlSelectors.stateValue(control.id, value)
                    }
                }
            }
        }
        dialog.show()
        track(dialog)
    }

    private fun showExpandedState(
        port: WebChatSocialMcpPort,
        control: ChatGptWebUiControl,
        onUpdated: () -> Unit,
    ) {
        val values = listOf(true, false)
        val dialog = AlertDialog.Builder(activity)
            .setTitle(control.label)
            .setSingleChoiceItems(arrayOf("展开", "收起"), values.indexOf(control.expanded)) { opened, index ->
                dispatch(port, JSONObject()
                    .put("action", "chatgpt_set_control_expanded")
                    .put("control_id", control.id)
                    .put("expanded", values[index]), onUpdated)
                opened.dismiss()
            }
            .setNegativeButton(android.R.string.cancel, null)
            .create()
        dialog.setOnShowListener {
            dialog.listView?.apply {
                contentDescription = WebChatProductionAdaptiveControlSelectors.expansionList(control.id)
                post {
                    for (index in 0 until childCount) {
                        val value = values.getOrNull(firstVisiblePosition + index) ?: continue
                        getChildAt(index)?.contentDescription =
                            WebChatProductionAdaptiveControlSelectors.expansionValue(control.id, value)
                    }
                }
            }
        }
        dialog.show()
        track(dialog)
    }

    private fun dispatch(
        port: WebChatSocialMcpPort,
        args: JSONObject,
        onUpdated: () -> Unit,
    ) {
        val result = port.control(args)
        if (result.optBoolean("control_ok")) {
            onUpdated()
            return
        }
        val message = when (result.optString("error")) {
            "stale_control_id" -> "官网控件已变化，请重新打开列表"
            "control_not_writable", "control_state_not_settable",
            "control_choices_unavailable", "control_slider_unavailable",
            "control_expansion_unavailable" -> "当前控件不能以原生方式修改"
            "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
            else -> "网页控件更新失败，请重试"
        }
        Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
    }

    private fun track(dialog: AlertDialog) {
        activeDialog?.takeIf { it !== dialog }?.dismiss()
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
    }

    private companion object {
        val SINGLE_SELECT_ROLES = setOf("radio", "menuitemradio", "tab")
    }
}
