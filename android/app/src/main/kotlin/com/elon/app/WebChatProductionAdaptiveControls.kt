package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptNativeChoiceControlDialog
import com.elon.app.chatgptweb.ChatGptNativeControlPresentation
import com.elon.app.chatgptweb.ChatGptNativeFormControlDialog
import com.elon.app.chatgptweb.ChatGptNativeSliderControlDialog

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
        port: WebChatConsumerPort,
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
        port: WebChatConsumerPort,
        control: WebChatConsumerControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeFormControlDialog.show(activity, control) { controlId, text ->
            dispatch(port, controlId, WebChatConsumerControlMutation.Text(text), onUpdated)
        })
    }

    private fun showChoice(
        port: WebChatConsumerPort,
        control: WebChatConsumerControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeChoiceControlDialog.show(activity, control) { controlId, index ->
            dispatch(port, controlId, WebChatConsumerControlMutation.Choice(index), onUpdated)
        })
    }

    private fun showSlider(
        port: WebChatConsumerPort,
        control: WebChatConsumerControl,
        onUpdated: () -> Unit,
    ) {
        track(ChatGptNativeSliderControlDialog.show(activity, control) { controlId, value ->
            dispatch(port, controlId, WebChatConsumerControlMutation.Slider(value), onUpdated)
        })
    }

    private fun showSelectedState(
        port: WebChatConsumerPort,
        control: WebChatConsumerControl,
        onUpdated: () -> Unit,
    ) {
        val values = if (control.role in SINGLE_SELECT_ROLES) listOf(true) else listOf(true, false)
        val labels = if (values.size == 1) arrayOf("选择") else arrayOf("开启", "关闭")
        val checked = values.indexOf(control.selected)
        val dialog = AlertDialog.Builder(activity)
            .setTitle(control.label)
            .setSingleChoiceItems(labels, checked) { opened, index ->
                val selected = values[index]
                dispatch(
                    port,
                    control.id,
                    WebChatConsumerControlMutation.Selected(selected),
                    onUpdated,
                )
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
        port: WebChatConsumerPort,
        control: WebChatConsumerControl,
        onUpdated: () -> Unit,
    ) {
        val values = listOf(true, false)
        val dialog = AlertDialog.Builder(activity)
            .setTitle(control.label)
            .setSingleChoiceItems(arrayOf("展开", "收起"), values.indexOf(control.expanded)) { opened, index ->
                dispatch(
                    port,
                    control.id,
                    WebChatConsumerControlMutation.Expanded(values[index]),
                    onUpdated,
                )
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
        port: WebChatConsumerPort,
        controlId: String,
        mutation: WebChatConsumerControlMutation,
        onUpdated: () -> Unit,
    ) {
        val result = port.updateControl(controlId, mutation)
        if (result.accepted) {
            onUpdated()
            return
        }
        val message = when (result.error) {
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
