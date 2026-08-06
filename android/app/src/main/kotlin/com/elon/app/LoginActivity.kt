package com.elon.app

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

class LoginActivity : AppCompatActivity() {
    private val serverUrl get() = ElonApplication.activeServerUrl(this).trimEnd('/')

    private lateinit var tabLogin: TextView
    private lateinit var tabRegister: TextView
    private lateinit var nicknameRow: LinearLayout
    private lateinit var accountInput: EditText
    private lateinit var nicknameInput: EditText
    private lateinit var passwordInput: EditText
    private lateinit var submitButton: TextView
    private lateinit var skipButton: TextView
    private lateinit var errorText: TextView
    private lateinit var googleButton: TextView
    private lateinit var recoveryButton: TextView

    private var isRegisterMode = false
    private var submitting = false
    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_login)

        tabLogin = findViewById(R.id.loginTabLogin)
        tabRegister = findViewById(R.id.loginTabRegister)
        nicknameRow = findViewById(R.id.loginNicknameRow)
        accountInput = findViewById(R.id.loginAccountInput)
        nicknameInput = findViewById(R.id.loginNicknameInput)
        passwordInput = findViewById(R.id.loginPasswordInput)
        submitButton = findViewById(R.id.loginSubmitButton)
        skipButton = findViewById(R.id.loginSkipButton)
        errorText = findViewById(R.id.loginErrorText)
        googleButton = findViewById(R.id.loginGoogleButton)
        recoveryButton = findViewById(R.id.loginRecoveryButton)

        tabLogin.setOnClickListener { switchMode(false) }
        tabRegister.setOnClickListener { switchMode(true) }
        submitButton.setOnClickListener { onSubmit() }
        skipButton.setOnClickListener { finishToMain() }
        googleButton.setOnClickListener { signInWithGoogle() }
        recoveryButton.setOnClickListener { showRecoveryDialog() }

        switchMode(false)
    }

    private fun switchMode(register: Boolean) {
        isRegisterMode = register
        if (register) {
            tabRegister.setBackgroundColor(Color.parseColor("#20262E"))
            tabRegister.setTextColor(Color.parseColor("#F8F7F4"))
            tabLogin.setBackgroundColor(Color.parseColor("#0E1116"))
            tabLogin.setTextColor(Color.parseColor("#80BEBEBA"))
            nicknameRow.visibility = View.VISIBLE
            submitButton.text = "注册并登录"
            googleButton.visibility = View.GONE
            recoveryButton.visibility = View.GONE
        } else {
            tabLogin.setBackgroundColor(Color.parseColor("#20262E"))
            tabLogin.setTextColor(Color.parseColor("#F8F7F4"))
            tabRegister.setBackgroundColor(Color.parseColor("#0E1116"))
            tabRegister.setTextColor(Color.parseColor("#80BEBEBA"))
            nicknameRow.visibility = View.GONE
            submitButton.text = "登录"
            googleButton.visibility = View.VISIBLE
            recoveryButton.visibility = View.VISIBLE
        }
        errorText.visibility = View.GONE
    }

    private fun signInWithGoogle() {
        if (submitting) return
        submitting = true
        googleButton.isEnabled = false
        googleButton.text = "正在打开 Google…"
        errorText.visibility = View.GONE
        lifecycleScope.launch {
            try {
                val result = GoogleFederatedAuth(this@LoginActivity).authenticate("login")
                AuthManager.handleFederatedAuthResponse(this@LoginActivity, result)
                finishToMain()
            } catch (error: Throwable) {
                showError(error.message ?: "Google 登录失败")
            } finally {
                submitting = false
                googleButton.isEnabled = true
                googleButton.text = "使用 Google 账号登录"
            }
        }
    }

    private fun showRecoveryDialog() {
        val container = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(40, 4, 40, 0)
        }
        val account = recoveryInput("账号", "username", accountInput.text.toString().trim())
        val code = recoveryInput("离线恢复码", null, "")
        val password = recoveryInput("新密码（至少 8 位）", "newPassword", "", true)
        val confirm = recoveryInput("再次输入新密码", "newPassword", "", true)
        val note = TextView(this).apply {
            text = "恢复成功后所有现有会话都会撤销。邮件/短信找回接口已保留，但尚未配置。"
            setTextColor(Color.parseColor("#80BEBEBA"))
            textSize = 12f
            setPadding(0, 16, 0, 0)
        }
        container.addView(account)
        container.addView(code)
        container.addView(password)
        container.addView(confirm)
        container.addView(note)
        val dialog = AlertDialog.Builder(this)
            .setTitle("使用恢复码重置密码")
            .setView(container)
            .setNeutralButton("检查邮件/短信恢复", null)
            .setNegativeButton("取消", null)
            .setPositiveButton("重置密码", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                lifecycleScope.launch {
                    runCatching { AccountSecurityApi(this@LoginActivity).startExternalRecovery(account.text.toString()) }
                        .onSuccess { note.text = it }
                        .onFailure { note.text = it.message ?: "恢复服务不可用" }
                }
            }
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val next = password.text.toString()
                when {
                    account.text.isBlank() -> note.text = "请输入账号"
                    code.text.isBlank() -> note.text = "请输入离线恢复码"
                    next.length < 8 -> note.text = "新密码至少 8 位"
                    next != confirm.text.toString() -> note.text = "两次输入的新密码不一致"
                    else -> lifecycleScope.launch {
                        dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = false
                        runCatching {
                            AccountSecurityApi(this@LoginActivity).recoverPassword(
                                account.text.toString().trim(),
                                code.text.toString().trim(),
                                next,
                            )
                        }.onSuccess {
                            accountInput.setText(account.text)
                            passwordInput.text.clear()
                            showError("密码已重置，请使用新密码登录")
                            dialog.dismiss()
                        }.onFailure { note.text = it.message ?: "密码重置失败" }
                        dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = true
                    }
                }
            }
        }
        dialog.show()
    }

    private fun recoveryInput(
        hint: String,
        autofillHint: String?,
        value: String,
        password: Boolean = false,
    ) = EditText(this).apply {
        this.hint = hint
        setText(value)
        if (autofillHint != null) setAutofillHints(autofillHint)
        inputType = if (password) android.text.InputType.TYPE_CLASS_TEXT or
            android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD else android.text.InputType.TYPE_CLASS_TEXT
        setTextColor(Color.parseColor("#F8F7F4"))
        setHintTextColor(Color.parseColor("#80BEBEBA"))
        setSingleLine(true)
    }

    private fun onSubmit() {
        if (submitting) return
        val account = accountInput.text.toString().trim()
        val password = passwordInput.text.toString()
        val nickname = nicknameInput.text.toString().trim()
        if (account.isEmpty()) return showError("请输入账号")
        if (password.length < 6) return showError("密码至少 6 位")
        if (isRegisterMode && nickname.isEmpty()) return showError("请输入昵称")

        submitting = true
        submitButton.text = if (isRegisterMode) "注册中…" else "登录中…"
        errorText.visibility = View.GONE

        val path = if (isRegisterMode) "/api/auth/register" else "/api/auth/login"
        val payload = JSONObject().apply {
            put("account", account)
            put("password", password)
            if (isRegisterMode) put("nickname", nickname)
        }
        val req = Request.Builder()
            .url("$serverUrl$path")
            .post(payload.toString().toRequestBody("application/json".toMediaTypeOrNull()))
            .build()

        thread(name = "login-submit") {
            try {
                http.newCall(req).execute().use { resp ->
                    val body = resp.body?.string().orEmpty()
                    if (!resp.isSuccessful) {
                        val msg = parseErrorMessage(body) ?: "请求失败 (${resp.code})"
                        runOnUiThread { onResult(false, msg) }
                        return@thread
                    }
                    AuthManager.handleAuthResponse(this, body)
                    runOnUiThread { onResult(true, null) }
                }
            } catch (t: Throwable) {
                runOnUiThread { onResult(false, t.message ?: "网络错误") }
            }
        }
    }

    private fun parseErrorMessage(body: String): String? = try {
        val j = JSONObject(body)
        j.optString("message").takeIf { it.isNotBlank() }
            ?: j.optString("error").takeIf { it.isNotBlank() }
    } catch (_: Throwable) {
        null
    }

    private fun onResult(success: Boolean, error: String?) {
        submitting = false
        submitButton.text = if (isRegisterMode) "注册并登录" else "登录"
        if (success) {
            finishToMain()
        } else {
            showError(error ?: "未知错误")
        }
    }

    private fun showError(msg: String) {
        errorText.text = msg
        errorText.visibility = View.VISIBLE
    }

    private fun finishToMain() {
        val intent = Intent(this, MainActivity::class.java)
        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        startActivity(intent)
        finish()
    }
}
