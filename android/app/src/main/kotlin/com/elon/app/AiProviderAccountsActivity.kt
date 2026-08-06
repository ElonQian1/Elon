package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.MenuItem
import android.view.View
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import java.util.concurrent.TimeUnit

class AiProviderAccountsActivity : AppCompatActivity() {
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()
    private val api by lazy { AiProviderAccountsApi(this, http) }

    private lateinit var nodeSpinner: Spinner
    private lateinit var progress: ProgressBar
    private lateinit var status: TextView
    private lateinit var providerContainer: LinearLayout
    private var nodes: List<AiProviderNode> = emptyList()
    private var providers: List<AiProviderAccount> = emptyList()
    private var pollJob: Job? = null
    private var spinnerReady = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_ai_provider_accounts)
        supportActionBar?.apply {
            title = "AI 厂商账号"
            setDisplayHomeAsUpEnabled(true)
        }
        if (!AuthManager.isLoggedIn(this)) {
            Toast.makeText(this, "请先登录一龙账号", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        nodeSpinner = findViewById(R.id.aiProviderNodeSpinner)
        progress = findViewById(R.id.aiProviderProgress)
        status = findViewById(R.id.aiProviderStatus)
        providerContainer = findViewById(R.id.aiProviderContainer)
        findViewById<Button>(R.id.aiProviderRefresh).setOnClickListener { loadNodes() }
        nodeSpinner.onItemSelectedListener = SimpleItemSelectedListener { position ->
            if (spinnerReady) nodes.getOrNull(position)?.let(::loadAccounts)
        }
        loadNodes()
    }

    private fun loadNodes() = launchRequest("正在读取我的 Win 节点…") {
        val fetched = withContext(Dispatchers.IO) { api.fetchNodes() }
        nodes = fetched.sortedWith(compareByDescending<AiProviderNode> { it.online }.thenBy { it.label })
        spinnerReady = false
        nodeSpinner.adapter = ArrayAdapter(
            this,
            android.R.layout.simple_spinner_item,
            nodes.map { "${it.label} · ${if (it.online) "在线" else "离线"}" },
        ).also { it.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item) }
        spinnerReady = true
        val onlineIndex = nodes.indexOfFirst(AiProviderNode::online)
        if (onlineIndex < 0) {
            providers = emptyList()
            renderProviders()
            status.text = if (nodes.isEmpty()) "尚未绑定 Win 节点。" else "没有在线 Win 节点。"
        } else {
            nodeSpinner.setSelection(onlineIndex)
            loadAccounts(nodes[onlineIndex])
        }
    }

    private fun loadAccounts(node: AiProviderNode) {
        if (!node.online) {
            providers = emptyList()
            renderProviders()
            status.text = "该 Win 节点离线，无法管理官方 CLI 账号。"
            return
        }
        launchRequest("正在读取 ${node.label} 的账号状态…") {
            providers = withContext(Dispatchers.IO) { api.fetchAccounts(node.id) }
            status.text = "账号凭据保留在 ${node.label}，不会上传到一龙云端。"
            renderProviders()
            schedulePollingIfNeeded(node)
        }
    }

    private fun renderProviders() {
        renderAiProviderRows(
            layoutInflater,
            providerContainer,
            providers,
            AiProviderRowActions(
                onPrimary = ::handlePrimaryAction,
                onOfficialLogin = ::openOfficialLogin,
            ),
        )
    }

    private fun handlePrimaryAction(provider: AiProviderAccount) {
        val node = selectedOnlineNode() ?: return
        val attempt = provider.activeLogin
        when {
            attempt?.active == true -> launchRequest("正在取消登录…") {
                withContext(Dispatchers.IO) {
                    api.cancelLogin(node.id, provider.id, attempt.loginId)
                }
                loadAccounts(node)
            }
            provider.cliLoggedIn == true -> launchRequest("正在退出 ${provider.label}…") {
                withContext(Dispatchers.IO) { api.logout(node.id, provider.id) }
                loadAccounts(node)
            }
            else -> launchRequest("正在启动 ${provider.label} 官方登录…") {
                val started = withContext(Dispatchers.IO) { api.startLogin(node.id, provider.id) }
                providers = providers.map {
                    if (it.id == provider.id) it.copy(activeLogin = started) else it
                }
                renderProviders()
                if (provider.id == "codex_cli") openOfficialLogin(started)
                else Toast.makeText(
                    this,
                    "请回到 ${node.label} 完成 ${provider.label} 官方登录",
                    Toast.LENGTH_LONG,
                ).show()
                schedulePollingIfNeeded(node)
            }
        }
    }

    private fun openOfficialLogin(attempt: AiProviderLoginAttempt) {
        attempt.userCode?.let { code ->
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("Codex 设备码", code))
            Toast.makeText(this, "设备码已复制", Toast.LENGTH_SHORT).show()
        }
        val url = attempt.verificationUrl ?: attempt.authUrl ?: return
        if (!url.startsWith("https://")) {
            Toast.makeText(this, "已拦截非 HTTPS 登录地址", Toast.LENGTH_SHORT).show()
            return
        }
        runCatching { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
            .onFailure { Toast.makeText(this, "无法打开官方登录页", Toast.LENGTH_SHORT).show() }
    }

    private fun schedulePollingIfNeeded(node: AiProviderNode) {
        pollJob?.cancel()
        val active = providers.firstOrNull { it.activeLogin?.active == true } ?: return
        val attempt = active.activeLogin ?: return
        pollJob = scope.launch {
            while (true) {
                delay(2_000)
                val latest = runCatching {
                    withContext(Dispatchers.IO) {
                        api.loginStatus(node.id, active.id, attempt.loginId)
                    }
                }.getOrElse {
                    status.text = "登录状态刷新失败：${it.message.orEmpty().take(160)}"
                    return@launch
                }
                providers = providers.map {
                    if (it.id == active.id) it.copy(activeLogin = latest) else it
                }
                renderProviders()
                if (!latest.active) {
                    if (latest.state == "completed") {
                        Toast.makeText(this@AiProviderAccountsActivity, "${active.label} 登录成功", Toast.LENGTH_SHORT).show()
                    }
                    loadAccounts(node)
                    return@launch
                }
            }
        }
    }

    private fun selectedOnlineNode(): AiProviderNode? {
        val node = nodes.getOrNull(nodeSpinner.selectedItemPosition)
        if (node?.online == true) return node
        Toast.makeText(this, "请选择在线 Win 节点", Toast.LENGTH_SHORT).show()
        return null
    }

    private fun launchRequest(message: String, block: suspend () -> Unit) {
        progress.visibility = View.VISIBLE
        status.text = message
        scope.launch {
            runCatching { block() }
                .onFailure { status.text = "操作失败：${it.message.orEmpty().take(300)}" }
            progress.visibility = View.GONE
        }
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == android.R.id.home) {
            finish()
            return true
        }
        return super.onOptionsItemSelected(item)
    }

    override fun onDestroy() {
        pollJob?.cancel()
        scope.cancel()
        http.dispatcher.executorService.shutdown()
        super.onDestroy()
    }
}
