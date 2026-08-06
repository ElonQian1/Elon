package com.elon.app

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

class LoginActivity : AppCompatActivity() {
    private val serverUrl = "http://43.139.149.158:8080"

    private lateinit var tabLogin: TextView
    private lateinit var tabRegister: TextView
    private lateinit var nicknameRow: LinearLayout
    private lateinit var accountInput: EditText
    private lateinit var nicknameInput: EditText
    private lateinit var passwordInput: EditText
    private lateinit var submitButton: TextView
    private lateinit var skipButton: TextView
    private lateinit var errorText: TextView

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

        tabLogin.setOnClickListener { switchMode(false) }
        tabRegister.setOnClickListener { switchMode(true) }
        submitButton.setOnClickListener { onSubmit() }
        skipButton.setOnClickListener { finishToMain() }

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
        } else {
            tabLogin.setBackgroundColor(Color.parseColor("#20262E"))
            tabLogin.setTextColor(Color.parseColor("#F8F7F4"))
            tabRegister.setBackgroundColor(Color.parseColor("#0E1116"))
            tabRegister.setTextColor(Color.parseColor("#80BEBEBA"))
            nicknameRow.visibility = View.GONE
            submitButton.text = "登录"
        }
        errorText.visibility = View.GONE
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
