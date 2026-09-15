package com.elon.app.articles.square

import org.json.JSONObject

internal class SquareAccount(private val a: SquareActivity) {
    fun render() {
        val account = a.account!!; val bound = account.optBoolean("bound")
        a.content.addView(a.ui.text(if (bound) "${account.optString("label")} · ${account.optString("masked_key")}" else "绑定自己的币安广场账号", 20f))
        a.content.addView(a.ui.text(if (bound) if (account.isNull("verified_at")) "凭证已保存，尚未通过实际发帖验证" else "已通过实际发帖验证" else "使用Square发帖凭证，无需交易密钥。", 14f, true))
        a.content.addView(a.button("在币安创作者中心获取凭证 ↗") { a.browse(SquareApi.CREATOR) })
        val label = a.field("账号备注", account.optString("label")); label.filters = arrayOf(android.text.InputFilter.LengthFilter(60))
        val key = a.field(if (bound) "更换发帖凭证" else "发帖凭证", secret = true); key.filters = arrayOf(android.text.InputFilter.LengthFilter(512))
        a.content.addView(a.ui.text("通过HTTPS传输并加密保存。更换或解绑会取消尚未提交的任务。", 14f, true))
        a.content.addView(a.button("保存绑定") {
            val body = JSONObject().put("label", label.text.toString()).put("api_key", key.text.toString())
            a.work({ a.api.request("/account", "PUT", body) }) { key.text.clear(); a.account = it; a.show("account"); a.notice("已保存，首次发帖成功后更新验证状态") }
        })
        if (bound) a.content.addView(a.button("解绑账号") { a.confirm("解绑会取消尚未提交任务并删除凭证，币安已有帖子不受影响。") { a.work({ a.api.request("/account", "DELETE") }) { key.text.clear(); a.account = it; a.show("account") } } })
    }
}
