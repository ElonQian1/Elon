package com.elon.app

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class MainAttachmentPanelActions(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val activeConversation: () -> AppConversation,
    private val attachmentPanel: () -> LinearLayout?,
    private val attachmentButton: () -> ImageButton?,
    private val collapseInputComposer: () -> Unit,
    private val collapseEmojiPanel: () -> Unit,
    private val openCameraAttachment: () -> Unit,
    private val openPhotoAttachment: () -> Unit,
    private val openDocumentAttachment: () -> Unit,
    private val showUiDesignAction: () -> Boolean,
    private val openUiDesignOptions: () -> Unit,
    private val webChatQuickActions: () -> List<WebChatProductionQuickComposerAction>,
    private val selectWebChatQuickAction: (WebChatProductionQuickComposerAction) -> Boolean,
) {
    var isOpen = false
        private set

    private var iconAnimationToken = 0
    private var uiDesignAction: View? = null
    private val webQuickActionViews = mutableMapOf<WebChatProductionQuickComposerAction, View>()

    fun buildAttachmentPanel(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.START
            orientation = LinearLayout.VERTICAL
            setPadding(dp(10), dp(4), dp(10), dp(12))
            visibility = View.GONE

            addView(createAttachmentAction("相机", R.drawable.ic_attach_camera, "attachment-action-camera") {
                openCameraAttachment()
            })
            addView(createAttachmentAction("照片", R.drawable.ic_attach_photos, "attachment-action-photos") {
                openPhotoAttachment()
            })
            addView(createAttachmentAction("文件", R.drawable.ic_attach_files, "attachment-action-files") {
                openDocumentAttachment()
            })
            WebChatProductionQuickComposerAction.entries.forEach { quickAction ->
                val icon = when (quickAction) {
                    WebChatProductionQuickComposerAction.IMAGE_GENERATION -> R.drawable.ic_attach_function
                    WebChatProductionQuickComposerAction.WEB_SEARCH -> R.drawable.ic_search_simple
                }
                val actionView = createAttachmentAction(
                    quickAction.label,
                    icon,
                    "web-chat-quick-action:${quickAction.semantic}",
                ) {
                    selectWebChatQuickAction(quickAction)
                }
                actionView.visibility = View.GONE
                webQuickActionViews[quickAction] = actionView
                addView(actionView)
            }
            val designAction = createAttachmentAction(
                "UI设计",
                R.drawable.ic_attach_function,
                "attachment-action-ui-design",
            ) {
                openUiDesignOptions()
            }
            uiDesignAction = designAction
            addView(designAction)
        }
    }

    fun toggleAttachmentPanel() {
        if (isOpen) collapseAttachmentPanel() else expandAttachmentPanel()
    }

    fun expandAttachmentPanel() {
        if (activeConversation().ended) return
        collapseEmojiPanel()
        collapseInputComposer()
        if (isOpen) return
        val panel = attachmentPanel() ?: return
        uiDesignAction?.visibility = if (showUiDesignAction()) View.VISIBLE else View.GONE
        val availableQuickActions = webChatQuickActions().toSet()
        webQuickActionViews.forEach { (action, view) ->
            view.visibility = if (action in availableQuickActions) View.VISIBLE else View.GONE
        }
        isOpen = true
        applyAttachmentPanelBackground(expanded = true)
        panel.visibility = View.VISIBLE
        animateAttachmentButtonIcon(expanded = true)
    }

    fun collapseAttachmentPanel() {
        val panel = attachmentPanel() ?: return
        val wasOpen = isOpen || panel.visibility == View.VISIBLE
        isOpen = false
        panel.visibility = View.GONE
        applyAttachmentPanelBackground(expanded = false)
        if (wasOpen) {
            animateAttachmentButtonIcon(expanded = false)
        } else {
            updateAttachmentButtonIcon(expanded = false)
        }
    }

    private fun createAttachmentAction(
        label: String,
        iconRes: Int,
        selector: String,
        action: () -> Unit
    ): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(56),
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = selector
            setPadding(dp(8), 0, dp(12), 0)

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                    marginEnd = dp(14)
                }
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(Color.parseColor("#2A2A2A"))
                }
                setImageResource(iconRes)
                scaleType = ImageView.ScaleType.FIT_CENTER
                setPadding(dp(8), dp(8), dp(8), dp(8))
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, dp(48), 1f)
                gravity = Gravity.CENTER_VERTICAL
                includeFontPadding = false
                text = label
                setTextColor(Color.parseColor("#F8F7F4"))
                textSize = 16f
            })
            setOnClickListener {
                collapseAttachmentPanel()
                action()
            }
        }
    }

    private fun applyAttachmentPanelBackground(expanded: Boolean) {
        val panel = attachmentPanel() ?: return
        (panel.parent as? View)?.setBackgroundResource(
            if (expanded) R.drawable.bg_bottom_panel_expanded else R.drawable.bg_bottom_panel_new
        )
    }

    private fun animateAttachmentButtonIcon(expanded: Boolean) {
        val button = attachmentButton() ?: return
        val token = ++iconAnimationToken
        val targetAlpha = if (activeConversation().ended) 0.55f else 1f
        button.animate().cancel()
        button.rotation = 0f
        button.scaleX = 1f
        button.scaleY = 1f
        button.animate()
            .alpha(0.55f)
            .setDuration(70L)
            .withEndAction {
                if (token != iconAnimationToken) return@withEndAction
                updateAttachmentButtonIcon(expanded)
                button.animate()
                    .alpha(targetAlpha)
                    .setDuration(90L)
                    .start()
            }
            .start()
    }

    private fun updateAttachmentButtonIcon(expanded: Boolean) {
        val button = attachmentButton() ?: return
        button.setImageResource(R.drawable.ic_input_add_new)
        button.rotation = if (expanded) 45f else 0f
        button.contentDescription = if (expanded) "收起更多输入功能" else "展开更多输入功能"
    }
}
