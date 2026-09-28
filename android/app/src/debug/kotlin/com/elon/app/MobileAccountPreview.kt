package com.elon.app

import android.content.Context
import android.view.LayoutInflater
import android.view.View
import android.widget.TextView

/** Offline production layouts; no authentication requests or account mutation callbacks. */
internal fun mobileAccountPreview(context: Context, scenario: String): View {
    val layout = if (scenario == "account_security") R.layout.activity_account_identities else R.layout.activity_login
    val root = LayoutInflater.from(context).inflate(layout, null)
    if (scenario == "account_security") {
        root.findViewById<TextView>(R.id.accountCurrentAccount).text = "演示账户 · 离线预览"
        root.findViewById<TextView>(R.id.accountGoogleBindingSummary).text = "Google 尚未绑定 · 示例状态"
        root.findViewById<TextView>(R.id.accountPasswordSummary).text = "已启用密码 · 示例状态"
        root.findViewById<TextView>(R.id.accountIdentityStatus).text = "离线布局预览，不读取或修改真实账号"
        root.findViewById<TextView>(R.id.accountSecurityStatus).text = "恢复码仅显示一次，请妥善保存。"
        renderAccountSessions(context, root.findViewById(R.id.accountSessionList), listOf(
            AccountSecuritySession("preview", "Android 演示设备", true, true, "刚刚", "")
        )) {}
    } else {
        applyLoginMode(root, scenario == "register")
        root.findViewById<View>(R.id.loginTabLogin).setOnClickListener { applyLoginMode(root, false) }
        root.findViewById<View>(R.id.loginTabRegister).setOnClickListener { applyLoginMode(root, true) }
    }
    return root
}
