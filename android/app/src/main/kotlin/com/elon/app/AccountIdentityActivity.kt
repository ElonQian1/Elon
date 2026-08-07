package com.elon.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class AccountIdentityActivity : AppCompatActivity() {
    private val auth by lazy { GoogleFederatedAuth(this) }
    private val securityApi by lazy { AccountSecurityApi(this) }
    private lateinit var list: LinearLayout
    private lateinit var bindButton: Button
    private lateinit var status: TextView
    private lateinit var currentAccount: TextView
    private lateinit var googleBindingState: TextView
    private lateinit var passwordSummary: TextView
    private lateinit var currentPassword: EditText
    private lateinit var newPassword: EditText
    private lateinit var confirmPassword: EditText
    private lateinit var changePasswordButton: Button
    private lateinit var rotateRecoveryCodesButton: Button
    private lateinit var securityStatus: TextView
    private lateinit var sessionList: LinearLayout
    private lateinit var revokeOtherSessionsButton: Button
    private var securitySnapshot: AccountSecuritySnapshot? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_account_identities)
        supportActionBar?.apply {
            title = "账号与安全"
            setDisplayHomeAsUpEnabled(true)
        }
        list = findViewById(R.id.accountIdentityList)
        bindButton = findViewById(R.id.accountBindGoogleButton)
        status = findViewById(R.id.accountIdentityStatus)
        currentAccount = findViewById(R.id.accountCurrentAccount)
        googleBindingState = findViewById(R.id.accountGoogleBindingSummary)
        passwordSummary = findViewById(R.id.accountPasswordSummary)
        currentPassword = findViewById(R.id.accountCurrentPassword)
        newPassword = findViewById(R.id.accountNewPassword)
        confirmPassword = findViewById(R.id.accountConfirmPassword)
        changePasswordButton = findViewById(R.id.accountChangePasswordButton)
        rotateRecoveryCodesButton = findViewById(R.id.accountRotateRecoveryCodesButton)
        securityStatus = findViewById(R.id.accountSecurityStatus)
        sessionList = findViewById(R.id.accountSessionList)
        revokeOtherSessionsButton = findViewById(R.id.accountRevokeOtherSessionsButton)
        currentAccount.text = maskedYilongAccount(AuthManager.account(this))
        bindButton.text = "绑定 Google 到 ${maskedYilongAccount(AuthManager.account(this))}"
        bindButton.setOnClickListener { bindGoogle() }
        changePasswordButton.setOnClickListener { changePassword() }
        rotateRecoveryCodesButton.setOnClickListener { rotateRecoveryCodes() }
        revokeOtherSessionsButton.setOnClickListener { confirmRevokeOtherSessions() }
        refresh()
        refreshSecurity()
    }

    override fun onSupportNavigateUp(): Boolean {
        finish()
        return true
    }

    private fun refresh() {
        status.text = "读取中…"
        googleBindingState.text = "正在读取绑定状态…"
        lifecycleScope.launch {
            runCatching { auth.identities() to auth.isGoogleConfigured() }
                .onSuccess { (identities, configured) -> render(identities, configured) }
                .onFailure {
                    val message = it.message ?: "读取失败"
                    status.text = message
                    googleBindingState.text = "Google 绑定状态暂不可用"
                }
        }
    }

    private fun render(identities: List<LinkedLoginIdentity>, googleConfigured: Boolean) {
        list.removeAllViews()
        identities.forEach { identity -> list.addView(identityRow(identity)) }
        val hasGoogle = identities.any { it.provider == "google" }
        bindButton.visibility = if (hasGoogle || !googleConfigured) View.GONE else View.VISIBLE
        googleBindingState.text = googleBindingSummary(identities, googleConfigured)
        status.text = when {
            hasGoogle -> "Google 已绑定到当前一龙账号"
            !googleConfigured -> "Google 登录尚未配置，暂时无法绑定"
            identities.isEmpty() -> "尚未绑定第三方登录方式"
            else -> "已绑定 ${identities.size} 种身份"
        }
    }

    private fun identityRow(identity: LinkedLoginIdentity): View {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = android.view.Gravity.CENTER_VERTICAL
            setPadding(16, 12, 8, 12)
            setBackgroundResource(R.drawable.bg_orbital_panel)
        }
        val label = TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            text = "${identity.displayName ?: "Google"}\n${identity.email ?: "已绑定 Google 身份"}"
            setTextColor(Color.parseColor("#F8F7F4"))
            textSize = 14f
        }
        val unlink = Button(this).apply {
            text = "解绑"
            minHeight = 48
            setOnClickListener { confirmUnlink(identity) }
        }
        row.addView(label)
        row.addView(unlink)
        return row
    }

    private fun bindGoogle() {
        val account = maskedYilongAccount(AuthManager.account(this))
        AlertDialog.Builder(this)
            .setTitle("绑定 Google 账号")
            .setMessage("接下来选择的 Google 账号将绑定到当前一龙账号 $account。不会按相同邮箱自动合并，也不会保存 Google 密码。")
            .setNegativeButton("取消", null)
            .setPositiveButton("继续使用 Google") { _, _ -> performGoogleBind() }
            .show()
    }

    private fun performGoogleBind() {
        bindButton.isEnabled = false
        status.text = "正在打开 Google…"
        lifecycleScope.launch {
            runCatching { auth.authenticate("bind") }
                .onSuccess {
                    Toast.makeText(this@AccountIdentityActivity, "Google 账号已绑定", Toast.LENGTH_SHORT).show()
                    refresh()
                }
                .onFailure { status.text = it.message ?: "绑定失败" }
            bindButton.isEnabled = true
        }
    }

    private fun refreshSecurity() {
        securityStatus.text = "读取账号安全状态…"
        lifecycleScope.launch {
            runCatching { securityApi.status() }
                .onSuccess { snapshot ->
                    securitySnapshot = snapshot
                    val passwordState = if (snapshot.passwordEnabled) "已启用密码" else "尚未设置密码"
                    val changed = snapshot.passwordChangedAt?.let { " · 修改于 $it" }.orEmpty()
                    passwordSummary.text = "$passwordState$changed\n可用离线恢复码：${snapshot.recoveryCodeCount} 个"
                    changePasswordButton.text = if (snapshot.passwordEnabled) "修改密码" else "设置密码"
                    currentPassword.visibility = if (snapshot.passwordEnabled) View.VISIBLE else View.GONE
                    renderAccountSessions(this@AccountIdentityActivity, sessionList, snapshot.sessions) {
                        confirmRevokeSession(it)
                    }
                    revokeOtherSessionsButton.isEnabled = snapshot.sessions.any { !it.current }
                    securityStatus.text = "恢复码仅显示一次；邮件与短信恢复尚未配置"
                }
                .onFailure { securityStatus.text = it.message ?: "读取账号安全状态失败" }
        }
    }

    private fun changePassword() {
        val next = newPassword.text.toString()
        if (next.length < 8) return showSecurityError("新密码至少 8 位")
        if (next != confirmPassword.text.toString()) return showSecurityError("两次输入的新密码不一致")
        if (securitySnapshot?.passwordEnabled == true && currentPassword.text.isBlank()) {
            return showSecurityError("请输入当前密码")
        }
        setSecurityBusy(true, "正在更新密码…")
        lifecycleScope.launch {
            runCatching {
                securityApi.changePassword(currentPassword.text.toString(), next)
            }.onSuccess {
                currentPassword.text.clear()
                newPassword.text.clear()
                confirmPassword.text.clear()
                Toast.makeText(this@AccountIdentityActivity, "密码已更新，其他设备会话已撤销", Toast.LENGTH_LONG).show()
                refreshSecurity()
            }.onFailure { showSecurityError(it.message ?: "密码更新失败") }
            setSecurityBusy(false)
        }
    }

    private fun rotateRecoveryCodes() {
        if (securitySnapshot?.passwordEnabled == true && currentPassword.text.isBlank()) {
            return showSecurityError("生成恢复码前请输入当前密码")
        }
        AlertDialog.Builder(this)
            .setTitle("生成新的恢复码")
            .setMessage("旧恢复码会立即失效，新恢复码只显示一次。确定继续？")
            .setNegativeButton("取消", null)
            .setPositiveButton("生成") { _, _ ->
                setSecurityBusy(true, "正在生成恢复码…")
                lifecycleScope.launch {
                    runCatching { securityApi.rotateRecoveryCodes(currentPassword.text.toString()) }
                        .onSuccess {
                            showOneTimeRecoveryCodes(this@AccountIdentityActivity, it)
                            refreshSecurity()
                        }
                        .onFailure { showSecurityError(it.message ?: "恢复码生成失败") }
                    setSecurityBusy(false)
                }
            }
            .show()
    }

    private fun confirmRevokeSession(session: AccountSecuritySession) {
        AlertDialog.Builder(this)
            .setTitle(if (session.current) "退出当前设备" else "撤销设备会话")
            .setMessage("确定撤销 ${session.deviceName} 的登录会话？")
            .setNegativeButton("取消", null)
            .setPositiveButton("撤销") { _, _ ->
                lifecycleScope.launch {
                    runCatching { securityApi.revokeSession(session.id) }
                        .onSuccess {
                            if (session.current) finishAfterLogout() else refreshSecurity()
                        }
                        .onFailure { showSecurityError(it.message ?: "撤销失败") }
                }
            }
            .show()
    }

    private fun confirmRevokeOtherSessions() {
        AlertDialog.Builder(this)
            .setTitle("退出其他设备")
            .setMessage("保留当前设备，撤销其他全部登录会话？")
            .setNegativeButton("取消", null)
            .setPositiveButton("退出其他设备") { _, _ ->
                lifecycleScope.launch {
                    runCatching { securityApi.revokeOtherSessions() }
                        .onSuccess {
                            Toast.makeText(this@AccountIdentityActivity, "已撤销 $it 个会话", Toast.LENGTH_SHORT).show()
                            refreshSecurity()
                        }
                        .onFailure { showSecurityError(it.message ?: "操作失败") }
                }
            }
            .show()
    }

    private fun setSecurityBusy(busy: Boolean, message: String? = null) {
        changePasswordButton.isEnabled = !busy
        rotateRecoveryCodesButton.isEnabled = !busy
        if (message != null) securityStatus.text = message
    }

    private fun showSecurityError(message: String) {
        securityStatus.text = message
    }

    private fun finishAfterLogout() {
        AuthManager.clear(this)
        startActivity(Intent(this, LoginActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        })
        finish()
    }

    private fun confirmUnlink(identity: LinkedLoginIdentity) {
        AlertDialog.Builder(this)
            .setTitle("解绑 Google 账号")
            .setMessage("确定解绑 ${identity.email ?: "这个身份"}？")
            .setNegativeButton("取消", null)
            .setPositiveButton("解绑") { _, _ ->
                lifecycleScope.launch {
                    runCatching { auth.unlink(identity.id) }
                        .onSuccess { refresh() }
                        .onFailure { status.text = it.message ?: "解绑失败" }
                }
            }
            .show()
    }
}
