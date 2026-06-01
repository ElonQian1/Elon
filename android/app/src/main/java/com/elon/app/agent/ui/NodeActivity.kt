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
private const val CARD = "#181B20"
private const val TEXT_PRIMARY = "#F2F5FA"
private const val TEXT_SECONDARY = "#A6AFBD"
private const val TEXT_TERTIARY = "#6F7785"
private const val PRIMARY_BG = "#58BE6A"
private const val PRIMARY_TEXT = "#07120A"
private const val ACCENT_BG = "#1A237E"
private const val SECONDARY_BG = "#283140"
private const val SECONDARY_TEXT = "#DDE8FC"
private const val BORDER = "#1E2126"

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
                text = "📦 安装 elon-node-agent"
                textSize = 14f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
            })

            addView(TextView(context).apply {
                text = "在你的 PC 上运行 node-agent，即可将本地 LLM 算力贡献给平台，赚取积分。"
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
                            "curl -o elon-node-agent http://$serverUrl/downloads/elon-node-agent && chmod +x elon-node-agent"
                        )
                    }
                })

                addView(Button(context).apply {
                    text = "🪟 Windows 下载"
                    textSize = 13f
                    setBackgroundColor(Color.parseColor(SECONDARY_BG))
                    setTextColor(Color.parseColor(SECONDARY_TEXT))
                    layoutParams = LinearLayout.LayoutParams(0, 100, 1f)
                    setOnClickListener {
                        copyToClipboard(
                            "Windows 下载地址",
                            "http://$serverUrl/downloads/elon-node-agent.exe"
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
            setBackgroundColor(Color.parseColor(PRIMARY_BG))
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
            NodeInfo(
                agentId = n.optString("agent_id"),
                label = n.optString("label", n.optString("agent_id")),
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
                    text = node.label.ifBlank { node.agentId }
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

            if (node.models.isNotEmpty()) {
                addView(TextView(context).apply {
                    text = "模型: ${node.models.joinToString(", ")}"
                    textSize = 12f
                    setTextColor(Color.parseColor(TEXT_SECONDARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.topMargin = 6 }
                })
            }
        }
    }

    // ── 注册节点弹窗 ──────────────────────────────────────────────────────────

    private fun showRegisterDialog() {
        val editLabel = EditText(this).apply {
            hint = "节点昵称（可选，如：我的游戏 PC）"
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
            setBackgroundColor(Color.parseColor(SECONDARY_BG))
            setPadding(20, 20, 20, 20)
        }

        AlertDialog.Builder(this)
            .setTitle("注册新 PC 节点")
            .setMessage("注册后你将获得 agent_id 和 secret，填入 node-agent 配置即可运行。")
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
            curl -o elon-node-agent $serverBase/downloads/elon-node-agent && chmod +x elon-node-agent
            
            # 启动命令
            NODE_AGENT_ID=${result.agentId} \
            NODE_AGENT_SECRET=${result.agentSecret} \
            NODE_OWNER_USER_ID=${result.ownerUserId} \
            NODE_CLOUD_URL=${result.cloudWsUrl}/agent/ws \
            ./elon-node-agent
        """.trimIndent()

        val winCmd = """
            # PowerShell 启动命令
            ${'$'}env:NODE_AGENT_ID="${result.agentId}"
            ${'$'}env:NODE_AGENT_SECRET="${result.agentSecret}"
            ${'$'}env:NODE_OWNER_USER_ID="${result.ownerUserId}"
            ${'$'}env:NODE_CLOUD_URL="${result.cloudWsUrl}/agent/ws"
            .\elon-node-agent.exe
        """.trimIndent()

        AlertDialog.Builder(this)
            .setTitle("✅ 节点已注册！")
            .setMessage("Agent ID：${result.agentId}\n\n⚠️ Secret 只显示一次，请立即复制保存到 PC：\n\n${result.agentSecret}\n\n点击"复制启动命令"获取完整启动脚本。")
            .setPositiveButton("复制 Linux 命令") { _, _ ->
                copyToClipboard("Linux 启动命令", linuxCmd)
            }
            .setNeutralButton("复制 Windows 命令") { _, _ ->
                copyToClipboard("Windows 启动命令", winCmd)
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
        val label: String,
        val online: Boolean,
        val models: List<String>
    )

    data class RegisterResult(
        val agentId: String,
        val agentSecret: String,
        val cloudWsUrl: String,
        val ownerUserId: String
    )
}
