package com.elon.app.chatgptweb

import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatImageButton
import com.elon.app.R

internal class ChatGptNativeOverlayControlsController(
    private val activity: AppCompatActivity,
    private val headerActionsScroll: HorizontalScrollView,
    private val headerActions: LinearLayout,
    private val onInvoke: (String) -> Unit,
    private val onSetText: (String, String) -> Unit,
    private val onSelectChoice: (String, Int) -> Unit,
    private val onSetSlider: (String, Double) -> Unit,
) {
    private var dialog: androidx.appcompat.app.AlertDialog? = null
    private var controls: List<ChatGptWebUiControl> = emptyList()
    private var contextLabel: String? = null

    fun render(value: ChatGptWebUiManifest) {
        val nextContextLabel = value.controls.firstOrNull {
            it.region == ChatGptWebUiRegion.OVERLAY && it.semantic == "timestamp"
        }?.label
        val nextControls = ChatGptNativeControlPresentation.pageActions(value.controls)
        val controlsChanged = nextControls.map(::revision) != controls.map(::revision)
        controls = nextControls
        contextLabel = nextContextLabel
        if (controlsChanged && dialog?.isShowing == true) {
            dialog?.dismiss()
            dialog = null
        }
        if (controls.isEmpty()) {
            dialog?.dismiss()
            dialog = null
            return
        }
        headerActions.addView(createTrigger())
        headerActionsScroll.visibility = View.VISIBLE
    }

    fun dispose() {
        dialog?.dismiss()
        dialog = null
    }

    private fun createTrigger(): AppCompatImageButton = AppCompatImageButton(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(44), dp(44))
        background = null
        setImageResource(R.drawable.ic_more_horizontal)
        imageTintList = activity.getColorStateList(R.color.elon_icon_primary)
        setPadding(dp(10), dp(10), dp(10), dp(10))
        contentDescription = controls.firstNotNullOfOrNull(ChatGptWebUiControl::contextId)?.let { contextId ->
            ChatGptNativeControlPresentation.messageOverlayActionsSelector(contextId, controls.size)
        } ?: ChatGptNativeControlPresentation.pageActionsSelector(controls)
        tooltipText = activity.getString(R.string.chatgpt_official_page_actions)
        setOnClickListener { showActions() }
    }

    private fun showActions() {
        val current = controls
        if (current.isEmpty() || activity.isFinishing || activity.isDestroyed) return
        dialog?.dismiss()
        val title = contextLabel?.takeIf(String::isNotBlank)?.let { label ->
            activity.getString(R.string.chatgpt_official_page_actions) + " · " + label
        } ?: activity.getString(R.string.chatgpt_official_page_actions)
        dialog = ChatGptNativeControlDialog.show(
            context = activity,
            title = title,
            controls = current,
            onSelected = { control ->
                if (control.supportsTextEntry) {
                    dialog = ChatGptNativeFormControlDialog.show(
                        context = activity,
                        control = control,
                        onSubmit = onSetText,
                    )
                } else if (control.supportsChoiceSelection) {
                    dialog = ChatGptNativeChoiceControlDialog.show(
                        context = activity,
                        control = control,
                        onSelected = onSelectChoice,
                    )
                } else if (control.supportsSliderValue) {
                    dialog = ChatGptNativeSliderControlDialog.show(
                        context = activity,
                        control = control,
                        onSubmit = onSetSlider,
                    )
                } else {
                    onInvoke(control.id)
                }
            },
        )
    }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private fun revision(control: ChatGptWebUiControl): String = listOf(
        control.id,
        control.label,
        control.selected.toString(),
        control.selectedChoiceIndex?.toString().orEmpty(),
        control.choiceLabels.joinToString("\u001f"),
        control.slider?.value?.toString().orEmpty(),
    ).joinToString("\u001e")

}
