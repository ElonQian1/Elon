package com.elon.app.chatgptweb

import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatImageButton
import com.elon.app.R

internal class ChatGptNativeOverlayControlsController(
    private val activity: AppCompatActivity,
    private val headerActionsScroll: HorizontalScrollView,
    private val headerActions: LinearLayout,
    private val shouldPresent: () -> Boolean,
    private val onInvoke: (String) -> Unit,
) {
    private var dialog: AlertDialog? = null
    private var controls: List<ChatGptWebUiControl> = emptyList()
    private var contextLabel: String? = null

    fun render(value: ChatGptWebUiManifest) {
        val nextContextLabel = value.controls.firstOrNull {
            it.region == ChatGptWebUiRegion.OVERLAY && it.semantic == "timestamp"
        }?.label
        val nextControls = value.controls.asSequence()
            .filter { it.region == ChatGptWebUiRegion.OVERLAY && it.enabled }
            .filterNot { it.semantic == "timestamp" }
            .filterNot { it.semantic in DEDICATED_NAVIGATION_SEMANTICS }
            .distinctBy(ChatGptWebUiControl::id)
            .take(MAX_OVERLAY_ACTIONS)
            .toList()
        val controlsChanged = nextControls.map(ChatGptWebUiControl::id) !=
            controls.map(ChatGptWebUiControl::id)
        controls = nextControls
        contextLabel = nextContextLabel
        if (controls.isEmpty()) {
            dialog?.dismiss()
            dialog = null
            return
        }
        headerActions.addView(createTrigger())
        headerActionsScroll.visibility = View.VISIBLE
        if (controlsChanged && shouldPresent()) showActions()
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
        contentDescription = "chatgpt-overlay-actions:${controls.size}"
        tooltipText = activity.getString(R.string.chatgpt_official_page_actions)
        setOnClickListener { showActions() }
    }

    private fun showActions() {
        val current = controls
        if (current.isEmpty() || activity.isFinishing || activity.isDestroyed) return
        dialog?.dismiss()
        val builder = AlertDialog.Builder(activity)
            .setTitle(R.string.chatgpt_official_page_actions)
        contextLabel?.takeIf(String::isNotBlank)?.let(builder::setMessage)
        dialog = builder
            .setItems(current.map(ChatGptWebUiControl::label).toTypedArray()) { opened, which ->
                opened.dismiss()
                onInvoke(current[which].id)
            }
            .setNegativeButton(android.R.string.cancel, null)
            .create()
            .also(AlertDialog::show)
    }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val MAX_OVERLAY_ACTIONS = 40
        val DEDICATED_NAVIGATION_SEMANTICS = setOf("conversation", "project")
    }
}
