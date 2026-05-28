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
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal data class MainInputComposerViews(
    val inputModeButton: ImageButton,
    val attachmentButton: ImageButton,
    val voiceHoldButton: TextView,
    val inputBarContainer: LinearLayout,
    val inputCenterContainer: FrameLayout,
    val expandedInputContainer: FrameLayout,
    val collapsedInputPreview: TextView,
    val modelButtonShell: FrameLayout,
    val modelChevron: ImageView,
    val inputRightControls: FrameLayout,
    val inputComposerMotion: InputComposerMotion,
    val attachmentPanel: LinearLayout,
    val runtimeInputModeStrip: RuntimeInputModeStrip,
    val expandEditorButton: ImageButton
)

internal class MainInputComposerSetup(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val currentModelLabel: () -> String,
    private val isVoiceMode: () -> Boolean,
    private val shouldAnimateInputFocus: () -> Boolean,
    private val isAttachmentPanelOpen: () -> Boolean,
    private val toggleVoiceMode: () -> Unit,
    private val focusInputComposer: () -> Unit,
    private val startSpeechToText: () -> Unit,
    private val stopSpeechToText: () -> Unit,
    private val cancelSpeechToText: () -> Unit,
    private val showModelPopupOrLoad: () -> Unit,
    private val sendMessage: () -> Unit,
    private val toggleAttachmentPanel: () -> Unit,
    private val buildAttachmentPanel: () -> LinearLayout,
    private val collapseAttachmentPanel: () -> Unit,
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

        inputEdit.detachFromParent()
        modelButton.detachFromParent()
        sendButton.detachFromParent()
        root.removeAllViews()
        root.orientation = LinearLayout.VERTICAL
        root.setPadding(0, 0, 0, 0)
        root.setBackgroundColor(Color.parseColor("#1E1E1E"))

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
                dp(60)
            )
            minimumHeight = dp(60)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(14), dp(6), dp(14), dp(6))
        }

        val inputModeButton = ImageButton(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                marginEnd = dp(10)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(R.drawable.ic_input_voice_circle)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(6), dp(6), dp(6), dp(6))
            contentDescription = "切换语音输入"
            setOnClickListener { toggleVoiceMode() }
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
            setOnClickListener { focusInputComposer() }
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
            setOnClickListener { focusInputComposer() }
        }

        lateinit var inputComposerMotion: InputComposerMotion
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
                    inputComposerMotion.setExpanded(hasFocus, animate = shouldAnimateInputFocus())
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
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 15f
            visibility = View.GONE
            setOnTouchListener { _, event ->
                when (event.action) {
                    MotionEvent.ACTION_DOWN -> {
                        startSpeechToText()
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
            layoutParams = LinearLayout.LayoutParams(dp(86), dp(32)).apply {
                marginEnd = dp(10)
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
            setPadding(dp(16), 0, dp(30), 0)
            setCompoundDrawablesRelativeWithIntrinsicBounds(0, 0, 0, 0)
            setTextColor(Color.parseColor("#2D2D2D"))
            textSize = 12.5f
            setOnClickListener { showModelPopupOrLoad() }
        }

        val modelChevron = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(13), dp(13), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(13)
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

        inputBarContainer.addView(inputModeButton)
        inputBarContainer.addView(modelButtonShell)
        inputBarContainer.addView(inputCenterContainer)
        inputRightControls.addView(attachmentButton)
        inputRightControls.addView(sendButton)
        inputBarContainer.addView(inputRightControls)

        val attachmentPanel = buildAttachmentPanel()
        val runtimeInputModeStrip = RuntimeInputModeStrip(
            activity = activity,
            dp = dp,
            onModeSelected = selectRunningInputMode
        )
        root.addView(expandedInputContainer)
        root.addView(runtimeInputModeStrip.view)
        root.addView(inputBarContainer)
        root.addView(attachmentPanel)

        inputComposerMotion = InputComposerMotion(
            expandedInputContainer = expandedInputContainer,
            collapsedInputContainer = inputCenterContainer,
            collapsedText = collapsedInputPreview,
            modelButton = modelButtonShell,
            rightControls = inputRightControls
        )
        inputEdit.setOnClickListener {
            if (!inputComposerMotion.isExpanded && !isVoiceMode()) {
                inputComposerMotion.setExpanded(true, animate = true)
            }
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
                collapseInputComposer()
            }
            false
        }
        binding.stageHintText.setOnClickListener {
            collapseAttachmentPanel()
            collapseInputComposer()
        }

        return MainInputComposerViews(
            inputModeButton = inputModeButton,
            attachmentButton = attachmentButton,
            voiceHoldButton = voiceHoldButton,
            inputBarContainer = inputBarContainer,
            inputCenterContainer = inputCenterContainer,
            expandedInputContainer = expandedInputContainer,
            collapsedInputPreview = collapsedInputPreview,
            modelButtonShell = modelButtonShell,
            modelChevron = modelChevron,
            inputRightControls = inputRightControls,
            inputComposerMotion = inputComposerMotion,
            attachmentPanel = attachmentPanel,
            runtimeInputModeStrip = runtimeInputModeStrip,
            expandEditorButton = expandEditorButton
        )
    }

    private fun View.detachFromParent() {
        (parent as? ViewGroup)?.removeView(this)
    }
}
