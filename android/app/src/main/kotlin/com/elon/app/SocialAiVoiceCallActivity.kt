package com.elon.app

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.Space
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope

class SocialAiVoiceCallActivity : AppCompatActivity() {

    companion object {
        private const val EXTRA_SERVER_URL = "server_url"
        private const val EXTRA_USER_ID = "user_id"

        fun createIntent(context: Context, serverUrl: String, userId: String): Intent =
            Intent(context, SocialAiVoiceCallActivity::class.java)
                .putExtra(EXTRA_SERVER_URL, serverUrl)
                .putExtra(EXTRA_USER_ID, userId)
    }

    private enum class CallState {
        Idle,
        Listening,
        Processing
    }

    private val bgColor = Color.parseColor("#0B1118")
    private val cardColor = Color.parseColor("#0E1116")
    private val subtleColor = Color.parseColor("#080B0F")
    private val borderColor = Color.parseColor("#667B8793")
    private val primaryTextColor = Color.parseColor("#F8F7F4")
    private val secondaryTextColor = Color.parseColor("#B3DDDBD5")
    private val tertiaryTextColor = Color.parseColor("#80BEBEBA")
    private val primaryActionColor = Color.parseColor("#F8F7F4")
    private val primaryActionTextColor = Color.parseColor("#0B1118")
    private val secondaryActionColor = Color.parseColor("#20262E")
    private val secondaryActionTextColor = Color.parseColor("#F8F7F4")
    private val infoBadgeColor = Color.parseColor("#151A20")
    private val infoTextColor = Color.parseColor("#67BEA0")

    private lateinit var serverUrl: String
    private lateinit var userId: String
    private lateinit var player: RealtimePcmPlayer
    private lateinit var statusText: TextView
    private lateinit var userTranscript: TextView
    private lateinit var aiTranscript: TextView
    private lateinit var micButton: TextView
    private lateinit var speakerButton: ImageButton
    private lateinit var endButton: ImageButton
    private lateinit var pulseBars: List<View>
    private lateinit var ttsSpeaker: VoiceSpeaker

    private var controller: RealtimeVoiceController? = null
    private var callState = CallState.Idle
    private var lastAiMessage = ""
    private var audioOutputEnabled = true
    private var usingTranscribeFallback = false

    private val recordAudioPermission = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            startListening()
        } else {
            Toast.makeText(this, "需要麦克风权限才能实时语音", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        serverUrl = intent.getStringExtra(EXTRA_SERVER_URL)?.takeIf { it.isNotBlank() }
            ?: ServerUrlManager.getActive(this)
        userId = intent.getStringExtra(EXTRA_USER_ID)?.takeIf { it.isNotBlank() }
            ?: AuthManager.effectiveUserId(this)
        player = RealtimePcmPlayer().also { it.outputEnabled = audioOutputEnabled }
        ttsSpeaker = VoiceSpeaker(this, respectUserToggle = false)
        applySystemBars()
        setContentView(buildContentView())
        setCallState(CallState.Idle)
        updateSpeakerButton()
        window.decorView.post { ensurePermissionAndStart() }
    }

    override fun onDestroy() {
        controller?.shutdown()
        controller = null
        player.release()
        ttsSpeaker.release()
        super.onDestroy()
    }

    private fun buildContentView(): View {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(bgColor)
            fitsSystemWindows = true
        }
        root.addView(toolbar(), LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            dp(50)
        ))
        root.addView(callSurface(), LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            0,
            1f
        ))
        root.addView(bottomControls(), LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ))
        return root
    }

    private fun toolbar(): View {
        return FrameLayout(this).apply {
            setBackgroundColor(bgColor)
            addView(TextView(context).apply {
                text = "‹"
                textSize = 30f
                setTextColor(primaryTextColor)
                gravity = Gravity.CENTER
                setOnClickListener { finish() }
            }, FrameLayout.LayoutParams(dp(56), ViewGroup.LayoutParams.MATCH_PARENT, Gravity.START))
            addView(TextView(context).apply {
                text = "一龙AI"
                textSize = 17f
                gravity = Gravity.CENTER
                includeFontPadding = false
                setTextColor(primaryTextColor)
            }, FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
                Gravity.CENTER
            ))
        }
    }

    private fun callSurface(): View {
        return ScrollView(this).apply {
            isFillViewport = true
            setBackgroundColor(bgColor)
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(dp(24), dp(28), dp(24), dp(24))
                addView(voiceAvatar())
                addView(statusBlock())
                addView(transcriptPanel(), LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(28) })
            }, FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun voiceAvatar(): View {
        return FrameLayout(this).apply {
            background = oval(cardColor, borderColor)
            addView(TextView(context).apply {
                text = "一龙AI"
                textSize = 22f
                typeface = Typeface.DEFAULT_BOLD
                gravity = Gravity.CENTER
                setTextColor(primaryTextColor)
            }, FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            ))
            val bars = LinearLayout(context).apply {
                gravity = Gravity.CENTER
                orientation = LinearLayout.HORIZONTAL
            }
            pulseBars = listOf(18, 30, 42, 30, 18).map { height ->
                View(context).apply {
                    background = rounded(primaryActionColor, 3)
                    alpha = 0.32f
                    bars.addView(this, LinearLayout.LayoutParams(dp(5), dp(height)).apply {
                        leftMargin = dp(3)
                        rightMargin = dp(3)
                    })
                }
            }
            addView(bars, FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                dp(52),
                Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
            ).apply { bottomMargin = dp(18) })
        }.also {
            it.layoutParams = LinearLayout.LayoutParams(dp(164), dp(164))
        }
    }

    private fun statusBlock(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(0, dp(22), 0, 0)
            statusText = TextView(context).apply {
                textSize = 18f
                typeface = Typeface.DEFAULT_BOLD
                gravity = Gravity.CENTER
                setTextColor(primaryTextColor)
            }
            addView(statusText, LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ))
            addView(TextView(context).apply {
                text = "轻声说，我在听"
                textSize = 13f
                gravity = Gravity.CENTER
                setTextColor(secondaryTextColor)
            }, LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(8) })
        }
    }

    private fun transcriptPanel(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = rounded(cardColor, 16, borderColor)
            setPadding(dp(18), dp(16), dp(18), dp(18))
            addView(transcriptLabel("你"))
            userTranscript = transcriptText("还没有开口")
            addView(userTranscript)
            addView(Space(context), LinearLayout.LayoutParams(1, dp(18)))
            addView(transcriptLabel("一龙AI"))
            aiTranscript = transcriptText("我在这儿")
            addView(aiTranscript)
        }
    }

    private fun transcriptLabel(text: String): TextView =
        TextView(this).apply {
            this.text = text
            textSize = 12f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(infoTextColor)
        }

    private fun transcriptText(text: String): TextView =
        TextView(this).apply {
            this.text = text
            textSize = 15f
            setLineSpacing(dp(3).toFloat(), 1.0f)
            setTextColor(secondaryTextColor)
            setPadding(0, dp(6), 0, 0)
        }

    private fun bottomControls(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(14), dp(18), dp(22))
            setBackgroundColor(bgColor)
            val row = LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER
            }
            speakerButton = iconControlButton(R.drawable.ic_voice_call_speaker_off, secondary = true).apply {
                contentDescription = "切换扬声器"
                setOnClickListener { toggleSpeaker() }
            }
            micButton = controlButton("开始说话", secondary = false).apply {
                setOnClickListener { handleMicTap() }
            }
            endButton = iconControlButton(R.drawable.ic_voice_call_hangup, secondary = true).apply {
                contentDescription = "结束通话"
                background = rounded(Color.parseColor("#3A2024"), 22, Color.parseColor("#553038"))
                setOnClickListener { finish() }
            }
            row.addView(speakerButton, LinearLayout.LayoutParams(0, dp(52), 0.86f))
            row.addView(Space(context), LinearLayout.LayoutParams(dp(10), 1))
            row.addView(micButton, LinearLayout.LayoutParams(0, dp(52), 1.28f))
            row.addView(Space(context), LinearLayout.LayoutParams(dp(10), 1))
            row.addView(endButton, LinearLayout.LayoutParams(0, dp(52), 0.72f))
            addView(row)
        }
    }

    private fun controlButton(text: String, secondary: Boolean): TextView =
        TextView(this).apply {
            this.text = text
            gravity = Gravity.CENTER
            textSize = 15f
            typeface = Typeface.DEFAULT_BOLD
            includeFontPadding = false
            background = if (secondary) {
                rounded(secondaryActionColor, 22)
            } else {
                rounded(primaryActionColor, 22)
            }
            setTextColor(if (secondary) secondaryActionTextColor else primaryActionTextColor)
        }

    private fun iconControlButton(iconRes: Int, secondary: Boolean): ImageButton =
        ImageButton(this).apply {
            setImageResource(iconRes)
            scaleType = ImageView.ScaleType.CENTER
            setPadding(dp(12), dp(12), dp(12), dp(12))
            background = if (secondary) {
                rounded(secondaryActionColor, 22)
            } else {
                rounded(primaryActionColor, 22)
            }
        }

    private fun handleMicTap() {
        when (callState) {
            CallState.Idle -> ensurePermissionAndStart()
            CallState.Listening -> Toast.makeText(this, "我一直听着，直接说就好", Toast.LENGTH_SHORT).show()
            CallState.Processing -> Toast.makeText(this, "可以直接插话，我会听见", Toast.LENGTH_SHORT).show()
        }
    }

    private fun ensurePermissionAndStart() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            startListening()
        } else {
            recordAudioPermission.launch(Manifest.permission.RECORD_AUDIO)
        }
    }

    private fun startListening() {
        controller?.let {
            it.resumeAutoListening()
            setCallState(CallState.Listening)
            statusText.text = "我在听，继续说"
            return
        }
        if (!player.start()) {
            Toast.makeText(this, "无法启动通话音频播放", Toast.LENGTH_SHORT).show()
        }
        player.clear()
        ttsSpeaker.stop()
        userTranscript.text = "正在听你说"
        aiTranscript.text = "我在这儿"
        setCallState(CallState.Listening)
        usingTranscribeFallback = false
        startVoiceSession(RealtimeVoiceWsClient.Mode.RealtimeChat)
    }

    private fun startVoiceSession(mode: RealtimeVoiceWsClient.Mode) {
        controller = RealtimeVoiceController(
            context = this,
            baseHttpUrl = serverUrl,
            userId = userId,
            mode = mode,
            target = RealtimeVoiceWsClient.Target.SocialAiDirect,
            continuousAutoCommit = mode == RealtimeVoiceWsClient.Mode.Transcribe,
            onTranscriptDelta = { text ->
                runOnUiThread {
                    if (text.isNotBlank()) userTranscript.text = text
                }
            },
            onTranscriptFinal = { text ->
                runOnUiThread {
                    if (text.isNotBlank()) userTranscript.text = text
                    if (callState == CallState.Listening) statusText.text = "我听见了"
                }
            },
            onAiProgress = { text ->
                runOnUiThread {
                    if (text.isNotBlank()) aiTranscript.text = text
                    setCallState(CallState.Processing)
                    statusText.text = "一龙AI 正在说"
                }
            },
            onAiDone = { message, _ ->
                runOnUiThread {
                    val reply = message.trim().ifBlank { "我在呢。" }
                    lastAiMessage = reply
                    aiTranscript.text = reply
                    if (usingTranscribeFallback) {
                        playFallbackReply(reply)
                    } else {
                        setCallState(CallState.Listening)
                        statusText.text = "我在听，继续说"
                    }
                }
            },
            onAiError = { message ->
                runOnUiThread {
                    aiTranscript.text = "一龙AI 出错：${message.take(60)}"
                    controller?.resumeAutoListening()
                    setCallState(CallState.Listening)
                    statusText.text = "我在听，继续说"
                }
            },
            onRealtimeAudio = { chunk ->
                player.play(chunk)
            },
            onRealtimeSpeechStarted = {
                runOnUiThread {
                    player.clear()
                    ttsSpeaker.stop()
                    setCallState(CallState.Listening)
                    statusText.text = "我在听，你说"
                }
            },
            onRealtimeSpeechStopped = {
                runOnUiThread {
                    statusText.text = "我听见了"
                }
            },
            onRealtimeResponseDone = {
                runOnUiThread {
                    setCallState(CallState.Listening)
                    statusText.text = "我在听，继续说"
                }
            },
            onError = { message ->
                runOnUiThread {
                    if (shouldFallbackToTranscribe(message)) {
                        switchToTranscribeFallback()
                    } else {
                        showFailure("语音失败：${message.take(60)}")
                    }
                }
            },
        )
        controller?.start(lifecycleScope)
    }

    private fun shouldFallbackToTranscribe(message: String): Boolean =
        SocialAiVoiceFallbackPolicy.shouldFallbackToTranscribe(
            message = message,
            alreadyUsingFallback = usingTranscribeFallback
        )

    private fun switchToTranscribeFallback() {
        controller?.shutdown()
        controller = null
        player.clear()
        usingTranscribeFallback = true
        userTranscript.text = "正在听你说"
        aiTranscript.text = "实时通话已自动切到稳定语音模式"
        setCallState(CallState.Listening)
        statusText.text = "我在听，继续说"
        startVoiceSession(RealtimeVoiceWsClient.Mode.Transcribe)
    }

    private fun playFallbackReply(reply: String) {
        setCallState(CallState.Processing)
        statusText.text = "一龙AI 正在说"
        if (!audioOutputEnabled) {
            controller?.resumeAutoListening()
            setCallState(CallState.Listening)
            statusText.text = "我在听，继续说"
            return
        }
        ttsSpeaker.speak(reply, onDone = {
            runOnUiThread {
                controller?.resumeAutoListening()
                setCallState(CallState.Listening)
                statusText.text = "我在听，继续说"
            }
        })
    }

    private fun finishTurn(nextStatus: String) {
        controller?.shutdown()
        controller = null
        player.clear()
        ttsSpeaker.stop()
        setCallState(CallState.Idle)
        statusText.text = nextStatus
    }

    private fun showFailure(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
        aiTranscript.text = message
        finishTurn("我在这儿，慢慢说")
    }

    private fun toggleSpeaker() {
        audioOutputEnabled = !audioOutputEnabled
        player.outputEnabled = audioOutputEnabled
        if (!audioOutputEnabled) {
            player.clear()
            ttsSpeaker.stop()
        }
        updateSpeakerButton()
    }

    private fun updateSpeakerButton() {
        val enabled = audioOutputEnabled
        speakerButton.setImageResource(
            if (enabled) R.drawable.ic_voice_call_speaker_on else R.drawable.ic_voice_call_speaker_off
        )
        speakerButton.contentDescription = if (enabled) "关闭扬声器" else "打开扬声器"
    }

    private fun setCallState(state: CallState) {
        callState = state
        when (state) {
            CallState.Idle -> {
                statusText.text = "我在这儿，慢慢说"
                micButton.text = "开始说话"
                micButton.isEnabled = true
                micButton.background = rounded(primaryActionColor, 22)
                micButton.setTextColor(primaryActionTextColor)
                setPulse(false)
            }
            CallState.Listening -> {
                statusText.text = "我在听，继续说"
                micButton.text = "畅聊中"
                micButton.isEnabled = false
                micButton.background = rounded(primaryActionColor, 22)
                micButton.setTextColor(primaryActionTextColor)
                setPulse(true)
            }
            CallState.Processing -> {
                statusText.text = "我在想"
                micButton.text = "一龙AI 处理中"
                micButton.isEnabled = false
                micButton.background = rounded(infoBadgeColor, 22)
                micButton.setTextColor(infoTextColor)
                setPulse(false)
            }
        }
    }

    private fun setPulse(active: Boolean) {
        pulseBars.forEachIndexed { index, view ->
            view.alpha = if (active) 0.46f + index.coerceAtMost(4).toFloat() * 0.08f else 0.32f
        }
    }

    private fun rounded(color: Int, radiusDp: Int, strokeColor: Int? = null): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = dp(radiusDp).toFloat()
            setColor(color)
            if (strokeColor != null) setStroke(dp(1), strokeColor)
        }

    private fun oval(color: Int, strokeColor: Int): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(color)
            setStroke(dp(1), strokeColor)
        }

    private fun applySystemBars() {
        window.statusBarColor = bgColor
        window.navigationBarColor = bgColor
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            window.isNavigationBarContrastEnforced = false
        }
    }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density + 0.5f).toInt()
}
