package com.elon.app

import android.annotation.SuppressLint
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.InsetDrawable
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
    val modeButtonRow: LinearLayout,
    val pendingAttachmentHost: LinearLayout,
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
        root.clipChildren = false
        root.clipToPadding = false
        root.setPadding(0, dp(12), 0, dp(6))
        root.setBackgroundColor(activity.getColor(R.color.elon_bg_app))

        val modeButtonRow = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(40)
            ).apply {
                marginStart = dp(20)
                marginEnd = dp(20)
                bottomMargin = dp(6)
            }
            clipChildren = false
            clipToPadding = false
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
        }

        val inputPanelContainer = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(20)
                marginEnd = dp(20)
            }
            clipChildren = false
            clipToPadding = false
            gravity = Gravity.BOTTOM
            orientation = LinearLayout.VERTICAL
            background = activity.getDrawable(R.drawable.bg_bottom_panel_new)
            setPadding(0, 0, 0, 0)
        }

        val pendingAttachmentHost = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            orientation = LinearLayout.VERTICAL
        }

        val expandedInputContainer = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0
            ).apply {
                marginStart = dp(102)
                marginEnd = dp(10)
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
            setPadding(dp(10), dp(4), dp(10), dp(4))
        }

        val inputModeButton = ImageButton(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(38), dp(38), Gravity.END or Gravity.CENTER_VERTICAL)
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_wave_new)
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(dp(4), dp(4), dp(4), dp(4))
            contentDescription = "切换语音输入"
            setOnClickListener { toggleVoiceMode() }
        }

        lateinit var inputComposerMotion: InputComposerMotion
        val openCollapsedInputComposer = {
            if (!isVoiceMode()) {
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
                dp(38),
                1f
            )
            background = ColorDrawable(Color.TRANSPARENT)
            minimumHeight = dp(38)
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
            setPadding(dp(4), 0, dp(4), 0)
            text = "输入内容"
            setTextColor(Color.parseColor("#5E5E5E"))
            textSize = 15f
            isClickable = true
            isFocusable = false
            isFocusableInTouchMode = false
            setOnClickListener { openCollapsedInputComposer() }
            setOnTouchListener(collapsedInputTouchListener)
        }

        val expandEditorButton = ImageButton(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(28), dp(28), Gravity.TOP or Gravity.END).apply {
                topMargin = dp(5)
                marginEnd = 0
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_expand_new)
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(dp(3), dp(3), dp(3), dp(3))
            contentDescription = "全屏编辑"
            setOnClickListener { showFullScreenEditor() }
        }

        inputEdit.apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ).apply {
                topMargin = bottomMenuHeight
            }
            background = ColorDrawable(Color.TRANSPARENT)
            hint = "输入内容"
            minLines = 1
            maxLines = 6
            setSingleLine(false)
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            isVerticalScrollBarEnabled = false
            includeFontPadding = false
            setHorizontallyScrolling(false)
            setPadding(dp(4), 0, dp(38), 0)
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#5E5E5E"))
            textSize = 17f
            setOnFocusChangeListener { _, hasFocus ->
                if (!isVoiceMode()) {
                    if (hasFocus) {
                        inputComposerMotion.expandForTextInput(animate = shouldAnimateInputFocus())
                    } else {
                        inputComposerMotion.setExpanded(
                            expanded = false,
                            animate = shouldAnimateInputFocus()
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
            setTextColor(Color.parseColor("#D6D6D6"))
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
            layoutParams = LinearLayout.LayoutParams(dp(76), dp(36)).apply {
                marginEnd = dp(10)
            }
            background = activity.getDrawable(R.drawable.bg_bottom_mode_pill_new)
            alpha = 1f
            visibility = View.VISIBLE
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
            setPadding(dp(16), 0, dp(24), 0)
            setCompoundDrawablesRelativeWithIntrinsicBounds(0, 0, 0, 0)
            setTextColor(Color.parseColor("#D6D6D6"))
            textSize = 14f
            setOnClickListener { showModelPopupOrLoad() }
        }

        val modelChevron = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(16), dp(16), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(10)
            }
            setImageResource(R.drawable.ic_input_chevron_new)
            scaleType = ImageView.ScaleType.FIT_CENTER
            alpha = 0.9f
            rotation = 0f
            isClickable = false
            isFocusable = false
        }
        modelButtonShell.addView(modelButton)
        modelButtonShell.addView(modelChevron)

        val planModeButton = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(72), dp(36)).apply {
                marginEnd = dp(8)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = "规划"
            textSize = 14f
            alpha = 1f
            visibility = View.VISIBLE
            isClickable = true
            isFocusable = true
            contentDescription = "开启先规划"
            setOnClickListener { togglePlanMode() }
        }

        val emojiButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(38), dp(38)).apply {
                marginEnd = dp(8)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_emoji_new)
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(dp(4), dp(4), dp(4), dp(4))
            contentDescription = "打开表情"
            setOnClickListener { toggleEmojiPanel() }
        }

        inputCenterContainer.addView(collapsedInputPreview)
        expandedInputContainer.addView(inputEdit)
        expandedInputContainer.addView(voiceHoldButton)
        expandedInputContainer.addView(expandEditorButton)

        val inputRightControls = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(38), dp(38))
        }

        val attachmentButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(38), dp(38)).apply {
                marginEnd = dp(8)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_add_new)
            scaleType = ImageView.ScaleType.FIT_CENTER
            setPadding(dp(4), dp(4), dp(4), dp(4))
            contentDescription = "展开更多输入功能"
            setOnClickListener { toggleAttachmentPanel() }
        }

        sendButton.apply {
            layoutParams = FrameLayout.LayoutParams(dp(38), dp(38), Gravity.END or Gravity.CENTER_VERTICAL)
            activity.getDrawable(R.drawable.ic_input_send_new)?.let { background = InsetDrawable(it, dp(3)) }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = ""
            setOnClickListener { sendMessage() }
        }

        val ttsSpeakerButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(38), dp(38)).apply {
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

        modeButtonRow.addView(modelButtonShell)
        modeButtonRow.addView(planModeButton)

        inputBarContainer.addView(attachmentButton)
        inputBarContainer.addView(emojiButton)
        inputBarContainer.addView(ttsSpeakerButton)
        inputBarContainer.addView(inputCenterContainer)
        inputRightControls.addView(inputModeButton)
        inputRightControls.addView(sendButton)
        inputBarContainer.addView(inputRightControls)

        val attachmentPanel = buildAttachmentPanel()
        val emojiPanel = buildEmojiPanel()
        val runtimeInputModeStrip = RuntimeInputModeStrip(
            activity = activity,
            dp = dp,
            onModeSelected = selectRunningInputMode
        )
        inputPanelContainer.addView(pendingAttachmentHost)
        inputPanelContainer.addView(expandedInputContainer)
        inputPanelContainer.addView(inputBarContainer)
        inputPanelContainer.addView(attachmentPanel)
        root.addView(modeButtonRow)
        root.addView(runtimeInputModeStrip.view)
        root.addView(inputPanelContainer)
        root.addView(emojiPanel)

        inputComposerMotion = InputComposerMotion(
            expandedInputContainer = expandedInputContainer,
            inputPanelContainer = inputPanelContainer,
            collapsedInputContainer = inputCenterContainer,
            collapsedText = collapsedInputPreview,
            rightControls = inputRightControls,
            expandedBottomOverlap = bottomMenuHeight
        )
        inputEdit.setOnClickListener {
            if (!isVoiceMode()) {
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
        binding.stageHintBar.setOnClickListener {
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
            modeButtonRow = modeButtonRow,
            pendingAttachmentHost = pendingAttachmentHost,
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
