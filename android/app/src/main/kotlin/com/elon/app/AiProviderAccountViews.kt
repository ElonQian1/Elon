package com.elon.app

import android.view.LayoutInflater
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

internal data class AiProviderRowActions(
    val onPrimary: (AiProviderAccount) -> Unit,
    val onOfficialLogin: (AiProviderLoginAttempt) -> Unit,
)

internal fun renderAiProviderRows(
    inflater: LayoutInflater,
    container: LinearLayout,
    providers: List<AiProviderAccount>,
    actions: AiProviderRowActions,
) {
    container.removeAllViews()
    providers.forEach { provider ->
        val row = inflater.inflate(R.layout.item_ai_provider_account, container, false)
        row.findViewById<TextView>(R.id.providerName).text = provider.label
        row.findViewById<TextView>(R.id.providerStatus).text = provider.statusText()
        row.findViewById<TextView>(R.id.providerDetail).text = provider.detailText()

        val primary = row.findViewById<Button>(R.id.providerPrimaryAction)
        val attempt = provider.activeLogin
        primary.text = provider.primaryActionLabel()
        primary.isEnabled = provider.implementationState == "available" &&
            (provider.cliRunnable || attempt?.active == true) &&
            !(provider.cliLoggedIn == true && !provider.logoutSupported)
        primary.setOnClickListener { actions.onPrimary(provider) }

        val official = row.findViewById<Button>(R.id.providerOfficialLoginAction)
        val officialUrl = attempt?.let { it.verificationUrl ?: it.authUrl }
        val canOpen = attempt?.active == true &&
            (officialUrl?.startsWith("https://") == true || !attempt.userCode.isNullOrBlank())
        official.visibility = if (canOpen) View.VISIBLE else View.GONE
        official.text = if (attempt?.userCode.isNullOrBlank()) "打开官方登录" else "复制验证码并打开"
        official.setOnClickListener { attempt?.let(actions.onOfficialLogin) }

        container.addView(row)
    }
}

private fun AiProviderAccount.statusText(): String = when {
    implementationState == "reserved" -> "接口已保留"
    activeLogin?.state == "starting" -> "正在启动官方登录"
    activeLogin?.state == "waiting_for_user" -> "等待用户完成登录"
    activeLogin?.state == "failed" -> "上次登录失败"
    activeLogin?.state == "canceled" -> "登录已取消"
    activeLogin?.state == "expired" -> "登录已过期"
    cliLoggedIn == true -> "已登录"
    !cliRunnable -> "CLI 未安装或不可运行"
    else -> "未登录"
}

private fun AiProviderAccount.detailText(): String = when {
    implementationState == "reserved" -> reason.orEmpty()
    activeLogin?.active == true && id == "codex_cli" -> {
        val code = activeLogin.userCode?.let { "\n设备码：$it" }.orEmpty()
        "请在官方 OpenAI 页面完成验证。$code"
    }
    activeLogin?.active == true && id == "gemini_cli" ->
        "Google 官方登录已在所选 Win 节点启动，请回到该电脑完成浏览器授权。"
    activeLogin?.active == true && id == "claude_cli" ->
        "Anthropic 官方登录已在所选 Win 节点启动，请回到该电脑完成浏览器授权。"
    activeLogin?.active == true && id == "copilot_cli" ->
        "GitHub 官方登录已在所选 Win 节点启动，请回到该电脑完成浏览器授权。"
    activeLogin?.error != null -> activeLogin.error + if (activeLogin.retryable) "\n可以安全地重新发起登录。" else ""
    cliLoggedIn == true && vaultBackupSupported ->
        "凭据由官方 CLI 保存；只有用户明确同意时才会进入现有 Codex 加密保险箱。"
    cliLoggedIn == true -> "凭据仅由官方 CLI 保存在 Win 节点；APK 与云端不保存凭据。"
    !cliRunnable -> cliDetail ?: "请先在所选 Win 节点安装官方 CLI。"
    id == "gemini_cli" && !remoteLoginSupported ->
        "APK 可以发起流程，但 Google 浏览器授权必须在所选 Win 节点完成。"
    else -> "通过官方登录协议绑定当前 Win 节点。"
}

private fun AiProviderAccount.primaryActionLabel(): String = when {
    implementationState == "reserved" -> "等待官方接口"
    activeLogin?.active == true -> "取消登录"
    cliLoggedIn == true && !logoutSupported -> "请在 CLI 退出"
    cliLoggedIn == true -> "退出登录"
    !cliRunnable -> "CLI 不可用"
    else -> "登录"
}
