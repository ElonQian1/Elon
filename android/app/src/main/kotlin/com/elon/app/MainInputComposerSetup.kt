package com.elon.app

import android.annotation.SuppressLint
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal data class MainInputComposerViews(
    val inputModeButton: ImageButton,
    val attachmentButton: ImageButton,
    val emojiButton: ImageButton,
    val voiceHoldButton: TextView,
    val inputBarContainer: LinearLayout,
    val inputCenterContainer: FrameLayout,
    val expandedInputContainer: FrameLayout,
    val collapsedInputPreview: TextView,
    val modelButtonShell: FrameLayout,
    val modelChevron: ImageView,
    val planModeButton: TextView,
    val inputRightControls: FrameLayout,
    val inputComposerMotion: InputComposerMotion,
    val attachmentPanel: LinearLayout,
    val emojiPanel: LinearLayout,
    val runtimeInputModeStrip: RuntimeInputModeStrip,
    val expandEditorButton: ImageButton,
    val ttsSpeakerButton: ImageButton
)

internal class MainInputComposerSetup(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val currentModelLabel: () -> String,
    private val isVoiceMode: () -> Boolean,
    private val shouldAnimateInputFocus: () -> Boolean,
    private val isAttachmentPanelOpen: () -> Boolean,
    private val isEmojiPanelOpen: () -> Boolean,
    private val toggleVoiceMode: () -> Unit,
    private val focusInputComposer: () -> Unit,
    private val startSpeechToText: () -> Unit,
    private val stopSpeechToText: () -> Unit,
    private val cancelSpeechToText: () -> Unit,
    private val onVoiceTouchMove: (Float, Float) -> Unit = { _, _ -> },
    private val showModelPopupOrLoad: () -> Unit,
    private val togglePlanMode: () -> Unit,
    private val sendMessage: () -> Unit,
    private val toggleAttachmentPanel: () -> Unit,
    private val toggleEmojiPanel: () -> Unit,
    private val buildAttachmentPanel: () -> LinearLayout,
    private val buildEmojiPanel: () -> LinearLayout,
    private val collapseAttachmentPanel: () -> Unit,
    private val collapseEmojiPanel: () -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val updateCollapsedInputPreview: () -> Unit,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit,
    private val selectRunningInputMode: (RunningInputMode) -> Unit,
    private val showFullScreenEditor: () -> Unit
) {
    @SuppressLint("ClickableViewAccessibility")
    fun setup(): MainInputComposerViews {
        val root = binding.inputLayout
        val inputEdit = binding.inputEdit
        val modelButton = binding.modelButton
        val sendButton = binding.sendButton
        val bottomMenuHeight = activity.resources.getDimensionPixelSize(R.dimen.main_bottom_menu_height)

        inputEdit.detachFromParent()
        modelButton.detachFromParent()
        sendButton.detachFromParent()
        root.removeAllViews()
        root.orientation = LinearLayout.VERTICAL
        root.minimumHeight = bottomMenuHeight
        root.setPadding(0, 0, 0, 0)
        root.setBackgroundColor(activity.getColor(R.color.elon_nav_bg))

        val expandedInputContainer = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0
            ).apply {
                marginStart = dp(24)
                marginEnd = dp(24)
            }
        }

        val inputBarContainer = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                bottomMenuHeight
            )
            minimumHeight = bottomMenuHeight
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(10), dp(6), dp(10), dp(6))
        }

        val inputModeButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                marginEnd = dp(8)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_circle)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "切换语音输入"
            setOnClickListener { toggleVoiceMode() }
        }

        lateinit var inputComposerMotion: InputComposerMotion
        val openCollapsedInputComposer = {
            if (!isVoiceMode()) {
                inputComposerMotion.expandForTextInput(animate = true)
                updateAdaptiveInputHeight()
                focusInputComposer()
            }
        }
        val collapsedInputTouchListener = View.OnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_DOWN) {
                openCollapsedInputComposer()
            }
            false
        }

        val inputCenterContainer = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                dp(40),
                1f
            )
            setBackgroundResource(R.drawable.bg_input_pill)
            minimumHeight = dp(40)
            isClickable = true
            isFocusable = false
            setOnClickListener { openCollapsedInputComposer() }
            setOnTouchListener(collapsedInputTouchListener)
        }

        val collapsedInputPreview = TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(18), 0, dp(18), 0)
            text = "文本内容在此输入。"
            setTextColor(Color.parseColor("#A8D0D0D0"))
            textSize = 15f
            isClickable = true
            isFocusable = false
            isFocusableInTouchMode = false
            setOnClickListener { openCollapsedInputComposer() }
            setOnTouchListener(collapsedInputTouchListener)
        }

        val expandEditorButton = ImageButton(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(34), dp(34), Gravity.TOP or Gravity.END).apply {
                topMargin = dp(3)
                marginEnd = dp(2)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_expand_editor)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(5), dp(5), dp(5), dp(5))
            contentDescription = "全屏编辑"
            setOnClickListener { showFullScreenEditor() }
        }

        inputEdit.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            background = ColorDrawable(Color.TRANSPARENT)
            hint = "文本内容在此输入。"
            minLines = 1
            maxLines = 4
            setSingleLine(false)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            isVerticalScrollBarEnabled = false
            includeFontPadding = true
            setHorizontallyScrolling(false)
            setPadding(0, dp(8), dp(36), dp(6))
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#A8D0D0D0"))
            textSize = 15f
            setOnFocusChangeListener { _, hasFocus ->
                if (!isVoiceMode()) {
                    if (!hasFocus || !inputComposerMotion.isKeyboardSynchronizedExpansionPending) {
                        inputComposerMotion.setExpanded(
                            hasFocus,
                            animate = shouldAnimateInputFocus(),
                            animateLayoutHeight = !hasFocus
                        )
                    }
                    updateSendButtonVisual()
                    updateAdaptiveInputHeight()
                }
            }
        }

        val voiceHoldButton = TextView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "按住 说话"
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 15f
            visibility = View.GONE
            setOnTouchListener { _, event ->
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        startSpeechToText()
                        onVoiceTouchMove(event.rawX, event.rawY)
                        true
                    }
                    MotionEvent.ACTION_MOVE -> {
                        onVoiceTouchMove(event.rawX, event.rawY)
                        true
                    }
                    MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                        if (event.action == MotionEvent.ACTION_UP) stopSpeechToText() else cancelSpeechToText()
                        true
                    }
                    else -> true
                }
            }
        }

        val modelButtonShell = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(76), dp(32)).apply {
                marginEnd = dp(8)
            }
            background = activity.getDrawable(R.drawable.bg_model_pill_light)
            alpha = 0f
            visibility = View.GONE
            isClickable = true
            isFocusable = false
            isFocusableInTouchMode = false
            contentDescription = "选择模型：${currentModelLabel()}"
            setOnClickListener { showModelPopupOrLoad() }
        }

        modelButton.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(14), 0, dp(27), 0)
            setCompoundDrawablesRelativeWithIntrinsicBounds(0, 0, 0, 0)
            setTextColor(Color.parseColor("#2D2D2D"))
            textSize = 12.5f
            setOnClickListener { showModelPopupOrLoad() }
        }

        val modelChevron = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(13), dp(13), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(11)
            }
            setImageResource(R.drawable.ic_input_model_chevron)
            scaleType = ImageView.ScaleType.CENTER
            alpha = 0.9f
            rotation = 0f
            isClickable = false
            isFocusable = false
        }
        modelButtonShell.addView(modelButton)
        modelButtonShell.addView(modelChevron)

        val planModeButton = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(64), dp(32)).apply {
                marginEnd = dp(6)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = "先规划"
            textSize = 12.5f
            alpha = 0f
            visibility = View.GONE
            isClickable = true
            isFocusable = true
            contentDescription = "开启先规划"
            setOnClickListener { togglePlanMode() }
        }

        val emojiButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                marginEnd = dp(6)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_emoji_circle)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "打开表情"
            setOnClickListener { toggleEmojiPanel() }
        }

        inputCenterContainer.addView(collapsedInputPreview)
        expandedInputContainer.addView(inputEdit)
        expandedInputContainer.addView(voiceHoldButton)
        expandedInputContainer.addView(expandEditorButton)

        val inputRightControls = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42))
        }

        val attachmentButton = ImageButton(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(42), dp(42), Gravity.START or Gravity.CENTER_VERTICAL)
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_add_circle_simple)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "展开更多输入功能"
            setOnClickListener { toggleAttachmentPanel() }
        }

        sendButton.apply {
            layoutParams = FrameLayout.LayoutParams(dp(42), dp(42), Gravity.END or Gravity.CENTER_VERTICAL)
            background = activity.getDrawable(R.drawable.ic_input_send_arrow_circle)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = ""
            setOnClickListener { sendMessage() }
        }

        val ttsSpeakerButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                marginEnd = dp(0)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(
                if (VoiceSpeaker.isTtsEnabled(activity)) R.drawable.ic_input_tts_on_circle
                else R.drawable.ic_input_tts_off_circle
            )
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "切换AI语音朗读"
            visibility = View.GONE
            setOnClickListener {
                val enabled = !VoiceSpeaker.isTtsEnabled(activity)
                VoiceSpeaker.setTtsEnabled(activity, enabled)
                setImageResource(
                    if (enabled) R.drawable.ic_input_tts_on_circle
                    else R.drawable.ic_input_tts_off_circle
                )
                val msg = if (enabled) "AI 回复将自动朗读" else "已关闭语音朗读"
                Toast.makeText(activity, msg, Toast.LENGTH_SHORT).show()
            }
        }

        inputBarContainer.addView(inputModeButton)
        inputBarContainer.addView(ttsSpeakerButton)
        inputBarContainer.addView(modelButtonShell)
        inputBarContainer.addView(emojiButton)
        inputBarContainer.addView(planModeButton)
        inputBarContainer.addView(inputCenterContainer)
        inputRightControls.addView(attachmentButton)
        inputRightControls.addView(sendButton)
        inputBarContainer.addView(inputRightControls)

        val attachmentPanel = buildAttachmentPanel()
        val emojiPanel = buildEmojiPanel()
        val runtimeInputModeStrip = RuntimeInputModeStrip(
            activity = activity,
            dp = dp,
            onModeSelected = selectRunningInputMode
        )
        root.addView(expandedInputContainer)
        root.addView(runtimeInputModeStrip.view)
        root.addView(inputBarContainer)
        root.addView(attachmentPanel)
        root.addView(emojiPanel)

        inputComposerMotion = InputComposerMotion(
            expandedInputContainer = expandedInputContainer,
            collapsedInputContainer = inputCenterContainer,
            collapsedText = collapsedInputPreview,
            modelButton = modelButtonShell,
            planModeButton = planModeButton,
            rightControls = inputRightControls
        )
        inputEdit.setOnClickListener {
            if (!isVoiceMode()) {
                inputComposerMotion.expandForTextInput(animate = true)
                updateAdaptiveInputHeight()
                focusInputComposer()
            }
        }

        inputEdit.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                updateCollapsedInputPreview()
                updateSendButtonVisual()
                updateAdaptiveInputHeight()
            }
            override fun afterTextChanged(s: Editable?) = Unit
        })

        binding.chatList.setOnTouchListener { _, event ->
            if (event.action == MotionEvent.ACTION_DOWN) {
                if (isAttachmentPanelOpen()) {
                    collapseAttachmentPanel()
                }
                if (isEmojiPanelOpen()) {
                    collapseEmojiPanel()
                }
                collapseInputComposer()
            }
            false
        }
        binding.stageHintText.setOnClickListener {
            collapseAttachmentPanel()
            collapseEmojiPanel()
            collapseInputComposer()
        }

        return MainInputComposerViews(
            inputModeButton = inputModeButton,
            attachmentButton = attachmentButton,
            emojiButton = emojiButton,
            voiceHoldButton = voiceHoldButton,
            inputBarContainer = inputBarContainer,
            inputCenterContainer = inputCenterContainer,
            expandedInputContainer = expandedInputContainer,
            collapsedInputPreview = collapsedInputPreview,
            modelButtonShell = modelButtonShell,
            modelChevron = modelChevron,
            planModeButton = planModeButton,
            inputRightControls = inputRightControls,
            inputComposerMotion = inputComposerMotion,
            attachmentPanel = attachmentPanel,
            emojiPanel = emojiPanel,
            runtimeInputModeStrip = runtimeInputModeStrip,
            expandEditorButton = expandEditorButton,
            ttsSpeakerButton = ttsSpeakerButton
        )
    }

    private fun View.detachFromParent() {
        (parent as? ViewGroup)?.removeView(this)
    }
}
