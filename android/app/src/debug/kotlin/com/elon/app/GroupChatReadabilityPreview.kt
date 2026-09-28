package com.elon.app

import android.content.Context
import android.view.ContextThemeWrapper
import android.view.LayoutInflater
import android.view.View
import android.widget.LinearLayout
import androidx.recyclerview.widget.LinearLayoutManager
import com.elon.app.databinding.ActivityMainBinding
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

/** Offline fixtures, using production message binding, composer and inset handling. Never sends data. */
internal fun groupChatReadabilityPreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.group.readability"
    override val supportedScenarios = setOf("answer", "input")

    override fun createView(context: Context, request: UiRuntimePreviewRequest): View =
        GroupChatReadabilityPreview(ContextThemeWrapper(context, R.style.Theme_ElonApp), request.scenario).binding.root
            .uiNode("group.readability.root")
}

internal class GroupChatReadabilityPreview(context: Context, scenario: String) {
    val binding = ActivityMainBinding.inflate(LayoutInflater.from(context))
    private val dp: (Int) -> Int = { (it * context.resources.displayMetrics.density).toInt() }
    private val keyboard = MainKeyboardInsetsAnimationActions(binding)
    val composer = MainInputComposerSetup(
        activity = context, binding = binding, dp = dp, currentModelLabel = { "档位 3" },
        isVoiceMode = { false }, shouldAnimateInputFocus = { false },
        isAttachmentPanelOpen = { false }, isEmojiPanelOpen = { false }, toggleVoiceMode = {},
        focusInputComposer = { binding.inputEdit.requestFocus() },
        startSpeechToText = {}, stopSpeechToText = {}, cancelSpeechToText = {},
        showModelPopupOrLoad = {}, togglePlanMode = {}, sendMessage = {},
        toggleAttachmentPanel = {}, toggleEmojiPanel = {},
        buildAttachmentPanel = { LinearLayout(context).apply { visibility = View.GONE } },
        buildEmojiPanel = { LinearLayout(context).apply { visibility = View.GONE } },
        collapseAttachmentPanel = {}, collapseEmojiPanel = {}, collapseInputComposer = {},
        updateCollapsedInputPreview = {}, updateSendButtonVisual = {}, updateAdaptiveInputHeight = {},
        onInputTextChanged = {}, selectRunningInputMode = {}, showFullScreenEditor = {},
    ).setup()
    val account = GroupChatGptAccountView(context) {}.apply { render(connected = true) }

    init {
        for (index in 0 until binding.contentContainer.childCount) {
            binding.contentContainer.getChildAt(index).visibility = View.GONE
        }
        for (index in 0 until binding.bottomBarContainer.childCount) {
            binding.bottomBarContainer.getChildAt(index).visibility = View.GONE
        }
        binding.chatPage.visibility = View.VISIBLE
        binding.stageHintBar.visibility = View.GONE
        binding.inputLayout.visibility = View.VISIBLE
        binding.backButton.visibility = View.VISIBLE
        binding.moreButton.visibility = View.VISIBLE
        binding.addButton.visibility = View.GONE
        binding.searchButton.visibility = View.GONE
        binding.topTitleText.text = "群聊布局示例"
        binding.topTitleText.contentDescription = "离线群聊预览，不连接真实群"
        composer.modelButtonShell.layoutParams = composer.modelButtonShell.layoutParams.apply { width = dp(142) }
        composer.modeButtonRow.layoutParams = composer.modeButtonRow.layoutParams.apply { height = -2 }
        composer.planModeButton.visibility = View.GONE
        composer.modeButtonRow.addView(account, LinearLayout.LayoutParams(0, -2, 1f))
        binding.modelButton.text = "档位 3"
        composer.modelButtonShell.contentDescription = "离线模型选择示例"
        binding.sendButton.visibility = View.GONE
        composer.inputModeButton.visibility = View.VISIBLE
        binding.chatList.layoutManager = LinearLayoutManager(context)
        binding.chatList.addOnChildAttachStateChangeListener(object : androidx.recyclerview.widget.RecyclerView.OnChildAttachStateChangeListener {
            override fun onChildViewAttachedToWindow(view: View) {
                val holder = binding.chatList.getChildViewHolder(view) as ChatAdapter.VH
                if (holder.bindingAdapterPosition == 1) holder.text.uiNode("group.readability.answer")
            }
            override fun onChildViewDetachedFromWindow(view: View) = Unit
        })
        binding.chatList.adapter = ChatAdapter(mutableListOf(
            ChatMessage("user", "请整理这张图片中的信息。", id = "preview-request", createdAtMs = 0),
            ChatMessage("ai", "图片信息整理：\n\n先列出能够确认的内容，再标注尚待核对的细节。文字、引用来源和后续讨论入口应该清楚可读。\n\n这是离线布局示例，没有发送任何群消息。",
                id = "preview-answer", senderLabel = "一龙 AI", createdAtMs = 0,
                groupAiReply = """{"schema":1,"provider":"chatgpt_web","requester_id":"offline","source_count":1,"previews":[{"sender_name":"示例成员","text":"图片与提问"}]}"""),
            ChatMessage("friend", "下一条群消息也应保持可读，不与底部账号入口叠在一起。", id = "preview-next",
                senderLabel = "示例成员", createdAtMs = 0),
        )).apply { onGroupAiReplyAction = { _, _ -> } }
        binding.inputLayout.uiNode("group.readability.composer")
        account.uiNode("group.readability.account")
        keyboard.install()
        if (scenario == "input") {
            binding.inputEdit.setText("尚未发送的群聊草稿\n检查输入区展开后的布局")
            composer.inputComposerMotion.updateExpandedTextHeight(dp(132), animate = false)
            composer.inputComposerMotion.expandForTextInput(animate = false)
        }
        binding.root.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(view: View) = Unit
            override fun onViewDetachedFromWindow(view: View) { binding.chatList.adapter = null }
        })
    }
}
