package com.elon.app

import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class AccountIdentityActivity : AppCompatActivity() {
    private val auth by lazy { GoogleFederatedAuth(this) }
    private lateinit var list: LinearLayout
    private lateinit var bindButton: Button
    private lateinit var status: TextView

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
        bindButton.setOnClickListener { bindGoogle() }
        refresh()
    }

    override fun onSupportNavigateUp(): Boolean {
        finish()
        return true
    }

    private fun refresh() {
        status.text = "读取中…"
        lifecycleScope.launch {
            runCatching { auth.identities() }
                .onSuccess(::render)
                .onFailure { status.text = it.message ?: "读取失败" }
        }
    }

    private fun render(identities: List<LinkedLoginIdentity>) {
        list.removeAllViews()
        identities.forEach { identity -> list.addView(identityRow(identity)) }
        bindButton.visibility = if (identities.any { it.provider == "google" }) View.GONE else View.VISIBLE
        status.text = if (identities.isEmpty()) "尚未绑定第三方登录方式" else "已绑定 ${identities.size} 种身份"
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
