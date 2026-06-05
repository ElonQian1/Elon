// infrastructure/floating/FloatingVoiceActivityV2.kt
// module: infrastructure/floating | layer: infrastructure | role: voice-balloon-activity
// summary: 悬浮球语音全双工 Activity - PCM 直流到服务器 Realtime，AI 返回 JSON 脚本则本地执行，返回文字则 TTS 播报

package com.elon.app.agent.infrastructure.floating

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.WindowManager
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.elon.app.AuthManager
import com.elon.app.RealtimePcmPlayer
import com.elon.app.RealtimeVoiceController
import com.elon.app.RealtimeVoiceWsClient
import com.elon.app.ServerUrlManager
import com.elon.app.agent.AgentService
import com.elon.app.agent.application.LocalBasicActionExecutor
import com.elon.app.agent.infrastructure.voice.AndroidTTSService

/**
 * 🎤 悬浮球语音全双工 Activity
 *
 * 升级说明（旧版 → 新版）：
 *  旧版：手机 ASR（文字） → 服务器 Codex（文字） → TTS → 手机执行
 *  新版：手机 PCM 直流 → 服务器 OpenAI Realtime → PCM 音频回流 + AI 文字
 *
 * 新版优势：
 *  - 真正全双工：用户说话和 AI 回复同时进行
 *  - 支持打断：检测到用户说话立刻清空 AI 播放缓冲
 *  - 长时间持续对话：无 ASR 启停开销，连接保持直到用户关闭
 *  - AI 返回 JSON 脚本 → 本地 ScriptEngine 执行手机操控
 *  - AI 返回文字 → PCM 音频实时播放，无需 TTS 转换
 */
class ConversationalVoiceActivity : AppCompatActivity() {

    companion object {
        private const val TAG = "ConversationalVoice"

        /** AI 回复是否为手机控制 JSON 脚本（有 steps 数组）。 */
        private fun isScriptJson(text: String): Boolean {
            val t = text.trim()
            if (!t.startsWith("{")) return false
            return try {
                val obj = org.json.JSONObject(t)
                obj.has("steps")
            } catch (_: Exception) { false }
        }
    }

    // ── UI ──────────────────────────────────────────────────
    private lateinit var statusText: TextView
    private lateinit var userText: TextView
    private lateinit var aiText: TextView
    private lateinit var cancelButton: Button

    // ── 语音核心 ──────────────────────────────────────────────
    private lateinit var player: RealtimePcmPlayer
    private var controller: RealtimeVoiceController? = null

    private val handler = Handler(Looper.getMainLooper())

    private val requestPermission = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) startRealtime()
        else { Toast.makeText(this, "需要麦克风权限", Toast.LENGTH_SHORT).show(); finish() }
    }

    // ── 生命周期 ──────────────────────────────────────────────

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setDimAmount(0.5f)
        }
        player = RealtimePcmPlayer()
        createUI()
        checkPermissionAndStart()
    }

    override fun onNewIntent(intent: android.content.Intent?) {
        super.onNewIntent(intent)
        setIntent(intent)
        Log.i(TAG, "🔁 onNewIntent - controller 已持续运行，无需重启")
        // Realtime 全双工连接持续保持，用户再次点悬浮球不需要重启
        userText.text = ""
        aiText.text = ""
        statusText.text = "在听，说吧"
    }

    override fun onDestroy() {
        super.onDestroy()
        handler.removeCallbacksAndMessages(null)
        controller?.shutdown()
        controller = null
        player.release()
    }

    // ── 权限与启动 ────────────────────────────────────────────

    private fun checkPermissionAndStart() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            == PackageManager.PERMISSION_GRANTED) startRealtime()
        else requestPermission.launch(Manifest.permission.RECORD_AUDIO)
    }

    private fun startRealtime() {
        if (controller != null) {
            // 已有连接，只需恢复监听
            controller?.resumeAutoListening()
            statusText.text = "在听，说吧"
            return
        }

        if (!player.start()) {
            Toast.makeText(this, "无法启动音频播放", Toast.LENGTH_SHORT).show()
        }
        player.clear()

        val serverUrl = ServerUrlManager.getActive(this)
        val userId = AuthManager.effectiveUserId(this)

        statusText.text = "连接中..."

        controller = RealtimeVoiceController(
            context = this,
            baseHttpUrl = serverUrl,
            userId = userId,
            mode = RealtimeVoiceWsClient.Mode.RealtimeChat,
            target = RealtimeVoiceWsClient.Target.PhoneControl,
            continuousAutoCommit = true,    // 检测到静音自动提交，无需用户操作

            onTranscriptDelta = { text ->
                runOnUiThread { if (text.isNotBlank()) userText.text = text }
            },
            onTranscriptFinal = { text ->
                runOnUiThread {
                    if (text.isNotBlank()) userText.text = text
                    statusText.text = "在想..."
                }
            },

            // AI 回复字幕增量：实时展示
            onAiProgress = { text ->
                runOnUiThread {
                    if (text.isNotBlank()) {
                        aiText.text = text
                        statusText.text = "AI 说"
                    }
                }
            },

            // AI 回复完成（对文字 fallback 路径）
            onAiDone = { message, _ ->
                runOnUiThread {
                    val reply = message.trim()
                    if (reply.isNotBlank()) aiText.text = reply
                    maybeExecuteScript(reply)
                    statusText.text = "在听，说吧"
                }
            },

            // Realtime PCM 音频片段 → 直接播放
            onRealtimeAudio = { chunk -> player.play(chunk) },

            // 用户开始说话 → 打断 AI，清空播放缓冲
            onRealtimeSpeechStarted = {
                runOnUiThread {
                    player.clear()
                    statusText.text = "在听..."
                    aiText.text = ""
                }
            },
            onRealtimeSpeechStopped = {
                runOnUiThread { statusText.text = "在想..." }
            },

            // AI 这轮回复结束 → 解析文字，检查是否为脚本
            onRealtimeResponseDone = {
                runOnUiThread {
                    val finalAiText = aiText.text.toString().trim()
                    maybeExecuteScript(finalAiText)
                    statusText.text = "在听，说吧"
                }
            },

            onError = { message ->
                runOnUiThread {
                    Log.w(TAG, "语音错误: $message")
                    statusText.text = "重试中..."
                    // Realtime 模式下 controller 内部会自动恢复，不需要手动重启
                }
            },
        )
        controller?.start(lifecycleScope)
        statusText.text = "在听，说吧"
    }

    /**
     * 检查 AI 回复文字是否为手机自动化 JSON 脚本：
     *  - 是脚本 → AgentService.executeTask 执行
     *  - 是文字 → 已经通过 PCM 音频播放，不做额外处理
     */
    private fun maybeExecuteScript(text: String) {
        if (!isScriptJson(text)) return

        // 三层漏斗：先试本地直控（最快），失败才走 ScriptEngine
        val agentService = AgentService.getInstance()
        if (agentService != null) {
            val localResult = LocalBasicActionExecutor.tryExecute(agentService, text)
            if (localResult is LocalBasicActionExecutor.Result.Handled) {
                Log.i(TAG, "⚡ 本地直控: ${localResult.message}")
                runOnUiThread { aiText.text = localResult.message }
                return
            }
        }

        // 交给 AgentService ScriptEngine 执行
        Log.i(TAG, "📜 执行脚本: ${text.take(60)}")
        runOnUiThread { statusText.text = "执行中..." }
        AgentService.executeTask(text)
    }

    // ── UI 构建 ───────────────────────────────────────────────

    private fun createUI() {
        val density = resources.displayMetrics.density
        fun dp(v: Int) = (v * density).toInt()

        val root = FrameLayout(this).apply {
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { finish() }
        }

        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(dp(28), dp(28), dp(28), dp(22))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#181B20"))
                cornerRadius = dp(24).toFloat()
            }
            elevation = dp(16).toFloat()
            isClickable = true
            isFocusable = true
        }

        // 状态指示
        statusText = TextView(this).apply {
            text = "连接中..."
            textSize = 16f
            setTextColor(Color.parseColor("#58BE6A"))
            gravity = Gravity.CENTER
        }
        card.addView(statusText)

        // 用户说的
        userText = TextView(this).apply {
            text = ""
            textSize = 17f
            setTextColor(Color.parseColor("#6091CF"))
            gravity = Gravity.CENTER
            maxLines = 4
            setPadding(0, dp(12), 0, 0)
        }
        card.addView(userText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))

        // AI 说的
        aiText = TextView(this).apply {
            text = ""
            textSize = 15f
            setTextColor(Color.parseColor("#A6AFBD"))
            gravity = Gravity.CENTER
            maxLines = 5
            setPadding(0, dp(8), 0, 0)
        }
        card.addView(aiText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))

        // 取消按钮
        cancelButton = Button(this).apply {
            text = "关闭"
            textSize = 14f
            setTextColor(Color.parseColor("#F2F5FA"))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#283140"))
                cornerRadius = dp(20).toFloat()
            }
            setPadding(dp(24), dp(8), dp(24), dp(8))
            setOnClickListener { finish() }
        }
        card.addView(cancelButton, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = dp(18)
        })

        card.addView(TextView(this).apply {
            text = "直接说话 · AI 实时回复"
            textSize = 11f
            setTextColor(Color.parseColor("#6F7785"))
            gravity = Gravity.CENTER
            setPadding(0, dp(10), 0, 0)
        })

        root.addView(card, FrameLayout.LayoutParams(dp(300), FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.CENTER))
        setContentView(root)
    }
}
