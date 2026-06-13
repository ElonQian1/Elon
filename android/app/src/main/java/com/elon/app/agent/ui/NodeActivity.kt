// ui/NodeActivity.kt
// module: ui | layer: presentation | role: 节点管理界面
// summary: PC 节点注册、状态查看、积分余额、下载 node-agent 二进制

package com.elon.app.agent.ui

import android.app.Activity
import android.app.AlertDialog
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.widget.*
import com.elon.app.agent.infrastructure.auth.AuthService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

private const val BG = "#101010"
private const val CARD = "#222222"
private const val TEXT_PRIMARY = "#D6D6D6"
private const val TEXT_SECONDARY = "#A8A8A8"
private const val TEXT_TERTIARY = "#777777"
private const val PRIMARY_BG = "#58BE6A"
private const val ACTION_BG = "#C8C8C8"
private const val PRIMARY_TEXT = "#101010"
private const val ACCENT_BG = "#2A2A2A"
private const val SECONDARY_BG = "#2A2A2A"
private const val SECONDARY_TEXT = "#D6D6D6"
private const val BORDER = "#2E2E2E"

/**
 * 节点管理界面：
 * - 查看/注册 PC 节点，获取 agent_id + secret
 * - 查看节点积分余额和收益流水
 * - 下载 elon-node-agent 二进制（一键复制启动命令）
 */
class NodeActivity : Activity() {

    private lateinit var authService: AuthService
    private val mainHandler = Handler(Looper.getMainLooper())
    private val scope = CoroutineScope(Dispatchers.Main)

    private lateinit var balanceView: TextView
    private lateinit var nodeListView: LinearLayout
    private lateinit var progressBar: ProgressBar
    private lateinit var tvError: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authService = AuthService(this)
        setContentView(createLayout())
        loadData()
    }

    override fun onResume() {
        super.onResume()
        loadData()
    }

    // ── 布局 ──────────────────────────────────────────────────────────────────

    private fun createLayout(): View {
        return ScrollView(this).apply {
            setBackgroundColor(Color.parseColor(BG))
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                setBackgroundColor(Color.parseColor(BG))
                setPadding(0, 0, 0, 80)

                addView(createHeader())
                addView(createBalanceCard())
                addView(createDownloadCard())
                addView(createNodesSection())
                addView(createRegisterButton())

                progressBar = ProgressBar(context).apply {
                    visibility = View.GONE
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.gravity = Gravity.CENTER_HORIZONTAL; it.topMargin = 24 }
                }
                addView(progressBar)

                tvError = TextView(context).apply {
                    visibility = View.GONE
                    setTextColor(Color.parseColor("#D97A7A"))
                    textSize = 14f
                    gravity = Gravity.CENTER
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.topMargin = 12; it.marginStart = 24; it.marginEnd = 24 }
                }
                addView(tvError)
            })
        }
    }

    private fun createHeader(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor(CARD))
            gravity = Gravity.CENTER_VERTICAL
            setPadding(24, 48, 24, 24)

            addView(ImageButton(context).apply {
                setImageResource(android.R.drawable.ic_menu_revert)
                setBackgroundColor(Color.TRANSPARENT)
                setColorFilter(Color.parseColor(TEXT_SECONDARY))
                layoutParams = LinearLayout.LayoutParams(80, 80)
                setOnClickListener { finish() }
            })

            addView(TextView(context).apply {
                text = "🖥️ 我的节点"
                textSize = 20f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.marginStart = 16 }
            })
        }
    }

    private fun createBalanceCard(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(24, 24, 24, 24)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = 12; it.marginStart = 16; it.marginEnd = 16 }

            addView(TextView(context).apply {
                text = "💰 节点积分收益"
                textSize = 14f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
            })

            balanceView = TextView(context).apply {
                text = "加载中..."
                textSize = 28f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(PRIMARY_BG))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 8 }
            }
            addView(balanceView)

            addView(TextView(context).apply {
                text = "贡献算力后，每 1k tokens 可获得平台积分奖励"
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_TERTIARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 6 }
            })
        }
    }

    private fun createDownloadCard(): LinearLayout {
        val serverUrl = authService.getServerUrl().replace("http://", "").replace("https://", "")
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(24, 24, 24, 24)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = 12; it.marginStart = 16; it.marginEnd = 16 }

            addView(TextView(context).apply {
                text = "📦 安装一龙 PC 节点"
                textSize = 14f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
            })

            addView(TextView(context).apply {
                text = "Windows 下载客户端包后双击安装，登录一次即可自动注册为 PC 节点并贡献算力。"
                textSize = 13f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 8 }
            })

            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 16 }

                addView(Button(context).apply {
                    text = "🐧 Linux 下载"
                    textSize = 13f
                    setBackgroundColor(Color.parseColor(SECONDARY_BG))
                    setTextColor(Color.parseColor(SECONDARY_TEXT))
                    layoutParams = LinearLayout.LayoutParams(0, 100, 1f).also { it.marginEnd = 8 }
                    setOnClickListener {
                        copyToClipboard(
                            "Linux 下载命令",
                            "curl -o elon-node-agent http://$serverUrl/api/node-agent/download/linux && chmod +x elon-node-agent"
                        )
                    }
                })

                addView(Button(context).apply {
                    text = "🪟 Windows 客户端包"
                    textSize = 13f
                    setBackgroundColor(Color.parseColor(SECONDARY_BG))
                    setTextColor(Color.parseColor(SECONDARY_TEXT))
                    layoutParams = LinearLayout.LayoutParams(0, 100, 1f)
                    setOnClickListener {
                        copyToClipboard(
                            "Windows 客户端包下载地址",
                            "http://$serverUrl/api/node-agent/download/windows-client"
                        )
                    }
                })
            })
        }
    }

    private fun createNodesSection(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = 20; it.marginStart = 16; it.marginEnd = 16 }

            addView(TextView(context).apply {
                text = "我的节点"
                textSize = 16f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
                setPadding(0, 0, 0, 12)
            })

            nodeListView = LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
            }
            addView(nodeListView)
        }
    }

    private fun createRegisterButton(): Button {
        return Button(this).apply {
            text = "＋ 注册新节点"
            textSize = 16f
            setTypeface(null, Typeface.BOLD)
            setBackgroundColor(Color.parseColor(ACTION_BG))
            setTextColor(Color.parseColor(PRIMARY_TEXT))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 130
            ).also { it.topMargin = 24; it.marginStart = 16; it.marginEnd = 16 }
            setOnClickListener { showRegisterDialog() }
        }
    }

    // ── 数据加载 ───────────────────────────────────────────────────────────────

    private fun loadData() {
        if (!authService.isLoggedIn()) {
            showError("请先登录")
            return
        }
        scope.launch {
            setLoading(true)
            try {
                val balance = fetchBalance()
                val nodes = fetchMyNodes()
                withContext(Dispatchers.Main) {
                    balanceView.text = "%.4f 积分".format(balance)
                    renderNodes(nodes)
                }
            } catch (e: Exception) {
                showError("加载失败: ${e.message}")
            } finally {
                setLoading(false)
            }
        }
    }

    private suspend fun fetchBalance(): Double = withContext(Dispatchers.IO) {
        val url = URL("${authService.getServerUrl()}/api/me/node-balance")
        val conn = (url.openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            connectTimeout = 10000
            readTimeout = 10000
        }
        val resp = conn.inputStream.bufferedReader().readText()
        JSONObject(resp).optDouble("balance", 0.0)
    }

    private suspend fun fetchMyNodes(): List<NodeInfo> = withContext(Dispatchers.IO) {
        val url = URL("${authService.getServerUrl()}/api/me/nodes")
        val conn = (url.openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            connectTimeout = 10000
            readTimeout = 10000
        }
        val resp = conn.inputStream.bufferedReader().readText()
        val json = JSONObject(resp)
        val arr = json.optJSONArray("nodes") ?: return@withContext emptyList()
        (0 until arr.length()).map { i ->
            val n = arr.getJSONObject(i)
            val agentId = n.optString("agent_id", n.optString("node_id"))
            val nodeId = n.optString("node_id", agentId)
            val shortId = n.optString("short_id").ifBlank { formatNodeId(nodeId) }
            val deviceName = n.optString("device_name").trim()
            val displayName = n.optString("display_name")
                .ifBlank { n.optString("label") }
                .ifBlank { deviceName }
                .ifBlank { shortId }
            NodeInfo(
                agentId = agentId,
                displayName = displayName,
                shortId = shortId,
                deviceName = deviceName,
                online = n.optBoolean("online", false),
                models = run {
                    val ms = n.optJSONArray("models")
                    if (ms == null) emptyList()
                    else (0 until ms.length()).map { j -> ms.getJSONObject(j).optString("model_id") }
                }
            )
        }
    }

    private fun renderNodes(nodes: List<NodeInfo>) {
        nodeListView.removeAllViews()
        if (nodes.isEmpty()) {
            nodeListView.addView(TextView(this).apply {
                text = "暂无节点。点击下方按钮注册第一台 PC 节点 →"
                setTextColor(Color.parseColor(TEXT_TERTIARY))
                textSize = 13f
                setPadding(0, 8, 0, 8)
            })
            return
        }
        nodes.forEach { node ->
            nodeListView.addView(createNodeCard(node))
        }
    }

    private fun createNodeCard(node: NodeInfo): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(20, 16, 20, 16)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.bottomMargin = 10 }

            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(context).apply {
                    text = if (node.online) "🟢" else "⚫"
                    textSize = 16f
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.marginEnd = 10 }
                })

                addView(TextView(context).apply {
                    text = node.displayName.ifBlank { node.shortId.ifBlank { node.agentId } }
                    textSize = 15f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })

                addView(TextView(context).apply {
                    text = if (node.online) "在线" else "离线"
                    textSize = 12f
                    setTextColor(if (node.online) Color.parseColor(PRIMARY_BG) else Color.parseColor(TEXT_TERTIARY))
                })
            })

            addView(TextView(context).apply {
                text = node.subtitle()
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_TERTIARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 6 }
            })

            addView(TextView(context).apply {
                text = if (node.models.isNotEmpty()) {
                    "模型: ${node.models.joinToString(", ")}"
                } else {
                    "暂无可用模型，确认 PC 端 Ollama / LM Studio 已启动"
                }
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_SECONDARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = 6 }
            })
        }
    }

    // ── 注册节点弹窗 ──────────────────────────────────────────────────────────

    private fun showRegisterDialog() {
        val editLabel = EditText(this).apply {
            hint = "节点昵称（可选，如：工作台 / 游戏主机）"
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
            setBackgroundColor(Color.parseColor(SECONDARY_BG))
            setPadding(20, 20, 20, 20)
        }

        AlertDialog.Builder(this)
            .setTitle("注册新 PC 节点")
            .setMessage("普通用户建议直接在 PC 端登录管理页，系统会自动读取电脑名作为设备名称。这里填写的是额外昵称，不填也能通过电脑名区分。")
            .setView(editLabel)
            .setPositiveButton("注册") { _, _ ->
                val label = editLabel.text.toString().trim()
                doRegisterNode(label.ifBlank { null })
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doRegisterNode(label: String?) {
        scope.launch {
            setLoading(true)
            try {
                val result = registerNode(label)
                withContext(Dispatchers.Main) {
                    showNodeCredentials(result)
                    loadData()
                }
            } catch (e: Exception) {
                showError("注册失败: ${e.message}")
            } finally {
                setLoading(false)
            }
        }
    }

    private suspend fun registerNode(label: String?): RegisterResult = withContext(Dispatchers.IO) {
        val url = URL("${authService.getServerUrl()}/api/me/nodes/register")
        val body = JSONObject().apply {
            if (label != null) put("label", label)
        }.toString()
        val conn = (url.openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            setRequestProperty("Content-Type", "application/json")
            setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            connectTimeout = 10000
            readTimeout = 10000
            doOutput = true
        }
        OutputStreamWriter(conn.outputStream).use { it.write(body) }
        val code = conn.responseCode
        if (code != 200) {
            val err = conn.errorStream?.bufferedReader()?.readText() ?: "HTTP $code"
            throw RuntimeException("服务器返回 $code: $err")
        }
        val resp = JSONObject(conn.inputStream.bufferedReader().readText())
        RegisterResult(
            agentId = resp.getString("agent_id"),
            agentSecret = resp.getString("agent_secret"),
            cloudWsUrl = resp.getString("cloud_ws_url"),
            ownerUserId = resp.getString("owner_user_id")
        )
    }

    private fun showNodeCredentials(result: RegisterResult) {
        val serverBase = authService.getServerUrl()
        val linuxCmd = """
            # 下载 node-agent
            curl -o elon-node-agent $serverBase/api/node-agent/download/linux && chmod +x elon-node-agent
            
            # 启动命令
            NODE_AGENT_ID=${result.agentId} \
            NODE_AGENT_SECRET=${result.agentSecret} \
            NODE_OWNER_USER_ID=${result.ownerUserId} \
            NODE_CLOUD_URL=${result.cloudWsUrl}/agent/ws \
            ./elon-node-agent
        """.trimIndent()

        val winCmd = """
            # Windows 推荐流程：
            # 1. 下载客户端包：$serverBase/api/node-agent/download/windows-client
            # 2. 解压后双击「安装一龙PC节点.cmd」
            # 3. 本地管理页打开后，用一龙账号登录一次即可自动注册节点
            #
            # 如需手动免网页登录，也可以在 node-agent.env 中配置：
            NODE_AGENT_ID=${result.agentId}
            NODE_AGENT_SECRET=${result.agentSecret}
            NODE_OWNER_USER_ID=${result.ownerUserId}
            NODE_CLOUD_URL=${result.cloudWsUrl}/agent/ws
        """.trimIndent()

        AlertDialog.Builder(this)
            .setTitle("✅ 节点已注册！")
            .setMessage("Agent ID：${result.agentId}\n\nWindows 用户推荐直接下载客户端包并在本地管理页登录，通常不需要手动保存 Secret。\n\n高级手动配置场景下，Secret 只显示一次：\n\n${result.agentSecret}")
            .setPositiveButton("复制 Linux 命令") { _, _ ->
                copyToClipboard("Linux 启动命令", linuxCmd)
            }
            .setNeutralButton("复制 Windows 安装说明") { _, _ ->
                copyToClipboard("Windows 安装说明", winCmd)
            }
            .setNegativeButton("关闭", null)
            .show()
    }

    // ── 工具方法 ───────────────────────────────────────────────────────────────

    private fun copyToClipboard(label: String, text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
        Toast.makeText(this, "已复制到剪贴板", Toast.LENGTH_SHORT).show()
    }

    private fun formatNodeId(id: String): String {
        return if (id.length > 16) "...${id.takeLast(14)}" else id
    }

    private fun setLoading(loading: Boolean) {
        mainHandler.post {
            progressBar.visibility = if (loading) View.VISIBLE else View.GONE
        }
    }

    private fun showError(msg: String) {
        mainHandler.post {
            tvError.text = msg
            tvError.visibility = View.VISIBLE
        }
    }

    // ── 数据类 ────────────────────────────────────────────────────────────────

    data class NodeInfo(
        val agentId: String,
        val displayName: String,
        val shortId: String,
        val deviceName: String,
        val online: Boolean,
        val models: List<String>
    ) {
        fun subtitle(): String {
            val idText = shortId.ifBlank {
                if (agentId.length > 16) "...${agentId.takeLast(14)}" else agentId
            }
            return if (deviceName.isNotBlank() && !deviceName.equals(displayName, ignoreCase = true)) {
                "设备: $deviceName · ID: $idText"
            } else {
                "ID: $idText"
            }
        }
    }

    data class RegisterResult(
        val agentId: String,
        val agentSecret: String,
        val cloudWsUrl: String,
        val ownerUserId: String
    )
}
