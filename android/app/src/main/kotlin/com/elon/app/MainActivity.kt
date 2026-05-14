package com.elon.app

import android.os.Bundle
import android.view.inputmethod.EditorInfo
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import java.util.UUID

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var wsClient: ElonWsClient

    /** 每次安装 APP 生成的唯一用户 ID，存入 SharedPreferences 持久化 */
    private val userId: String by lazy {
        val prefs = getSharedPreferences("elon", MODE_PRIVATE)
        prefs.getString("user_id", null) ?: UUID.randomUUID().toString().replace("-", "").also {
            prefs.edit().putString("user_id", it).apply()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        // 连接 WebSocket
        wsClient = ElonWsClient(
            serverUrl = "ws://182.254.168.75:8080/ws",
            onMessage = { msg -> runOnUiThread { appendMessage(msg) } },
            onConnected = { runOnUiThread { binding.statusText.text = "已连接" } },
            onDisconnected = { runOnUiThread { binding.statusText.text = "未连接，点击重试" } }
        )
        wsClient.connect()

        // 重连按钮
        binding.statusText.setOnClickListener { wsClient.connect() }

        // 发送按钮
        binding.sendButton.setOnClickListener { sendMessage() }

        // 键盘回车发送
        binding.inputEdit.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                sendMessage()
                true
            } else false
        }
    }

    private fun sendMessage() {
        val text = binding.inputEdit.text.toString().trim()
        if (text.isEmpty()) return

        // 显示用户消息
        appendMessage(ChatMessage(role = "user", content = text))
        binding.inputEdit.text.clear()
        binding.sendButton.isEnabled = false

        // 通过 WebSocket 发送 JSON（包含 user_id，服务端据此隔离工作区）
        val payload = """{"user_id":"$userId","message":${com.google.gson.JsonPrimitive(text)}}"""
        wsClient.send(payload)
    }

    private fun appendMessage(raw: String) {
        // 解析服务端推送的 JSON 消息
        try {
            val json = com.google.gson.JsonParser.parseString(raw).asJsonObject
            val type = json.get("type")?.asString ?: return
            val msg = when (type) {
                "progress"    -> ChatMessage("ai-progress", json.get("message")?.asString ?: "")
                "tool_call"   -> ChatMessage("ai-tool", "🔧 ${json.get("tool")?.asString}")
                "tool_result" -> return // 不显示工具结果，减少噪音
                "done"        -> {
                    binding.sendButton.isEnabled = true
                    val content = json.get("message")?.asString ?: ""
                    val apkUrl  = json.get("apk_url")?.asString
                    ChatMessage("ai", content + (apkUrl?.let { "\n\n📦 下载新APK: $it" } ?: ""))
                }
                "error" -> {
                    binding.sendButton.isEnabled = true
                    ChatMessage("error", "❌ ${json.get("message")?.asString}")
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) { }
    }

    private fun appendMessage(msg: ChatMessage) {
        val adapter = binding.chatList.adapter as? ChatAdapter
            ?: ChatAdapter(mutableListOf()).also { binding.chatList.adapter = it }
        adapter.addMessage(msg)
        binding.chatList.scrollToPosition(adapter.itemCount - 1)
    }

    override fun onDestroy() {
        super.onDestroy()
        wsClient.disconnect()
    }
}
