package com.elon.app

import android.content.Intent
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.inputmethod.EditorInfo
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import java.util.UUID

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var wsClient: ElonWsClient
    private var waitingForReply = false

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
        setSupportActionBar(binding.toolbar)

        // 连接 WebSocket
        wsClient = ElonWsClient(
            serverUrl = "ws://43.139.149.158:8080/ws",
            onMessage = { msg -> runOnUiThread { appendMessage(msg) } },
            onConnected = {
                runOnUiThread {
                    binding.statusText.text = "已连接"
                    if (!waitingForReply) binding.sendButton.isEnabled = true
                }
            },
            onDisconnected = {
                runOnUiThread {
                    binding.statusText.text = "未连接，点击重试"
                    if (waitingForReply) {
                        waitingForReply = false
                        appendMessage(ChatMessage("error", "连接已断开，请重试。"))
                    }
                    binding.sendButton.isEnabled = true
                }
            }
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

        if (!wsClient.isConnected()) {
            appendMessage(ChatMessage("error", "还没有连接到服务器，请点击上方状态栏重试。"))
            binding.statusText.text = "未连接，点击重试"
            wsClient.connect()
            return
        }

        val payload = com.google.gson.JsonObject().apply {
            addProperty("user_id", userId)
            addProperty("message", text)
        }

        // 显示用户消息
        appendMessage(ChatMessage(role = "user", content = text))
        binding.inputEdit.text.clear()
        binding.sendButton.isEnabled = false
        waitingForReply = true

        // 通过 WebSocket 发送 JSON（包含 user_id，服务端据此隔离工作区）
        if (!wsClient.send(payload.toString())) {
            waitingForReply = false
            binding.sendButton.isEnabled = true
            appendMessage(ChatMessage("error", "消息发送失败，请检查网络后重试。"))
        }
    }

    private fun jsonStringOrNull(json: com.google.gson.JsonObject, name: String): String? {
        val element = json.get(name) ?: return null
        if (element.isJsonNull) return null
        return runCatching { element.asString }.getOrNull()
    }

    private fun appendMessage(raw: String) {
        // 解析服务端推送的 JSON 消息
        try {
            val json = com.google.gson.JsonParser.parseString(raw).asJsonObject
            val type = jsonStringOrNull(json, "type") ?: return
            val msg = when (type) {
                "progress"    -> ChatMessage("ai-progress", jsonStringOrNull(json, "message") ?: "")
                "tool_call"   -> return // 内部工具调用不直接展示给用户
                "tool_result" -> return // 不显示工具结果，减少噪音
                "done"        -> {
                    waitingForReply = false
                    binding.sendButton.isEnabled = true
                    val content = jsonStringOrNull(json, "message") ?: ""
                    val apkUrl  = jsonStringOrNull(json, "apk_url")
                    ChatMessage("ai", content + (apkUrl?.let { "\n\n📦 下载新APK: $it" } ?: ""))
                }
                "error" -> {
                    waitingForReply = false
                    binding.sendButton.isEnabled = true
                    ChatMessage("error", "❌ ${jsonStringOrNull(json, "message") ?: "未知错误"}")
                }
                else -> return
            }
            appendMessage(msg)
        } catch (_: Exception) {
            waitingForReply = false
            binding.sendButton.isEnabled = true
            appendMessage(ChatMessage("error", "服务端返回异常，无法解析。"))
        }
    }

    private fun appendMessage(msg: ChatMessage) {
        val adapter = binding.chatList.adapter as? ChatAdapter
            ?: ChatAdapter(mutableListOf()).also { binding.chatList.adapter = it }
        adapter.addMessage(msg)
        binding.chatList.scrollToPosition(adapter.itemCount - 1)
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        menuInflater.inflate(R.menu.menu_main, menu)
        return true
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == R.id.action_settings) {
            startActivity(Intent(this, SettingsActivity::class.java))
            return true
        }
        return super.onOptionsItemSelected(item)
    }

    override fun onDestroy() {
        super.onDestroy()
        wsClient.disconnect()
    }
}
