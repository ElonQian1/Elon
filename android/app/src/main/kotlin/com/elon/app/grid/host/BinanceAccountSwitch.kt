package com.elon.app.grid.host

/** Only verified identities can complete a switch; the old account cannot silently reconnect. */
internal object BinanceAccountSwitch {
    fun key(account: String, kind: String) = "$account:$kind"
    fun accepts(previous: String, account: String?, kind: String): Boolean =
        account != null && kind in setOf("primary", "sub", "unknown") && account != previous.substringBefore(':')

    fun label(account: String?, kind: String): String {
        if (account == null) return "尚未确认币安账户，请打开官网登录或刷新"
        val type = when (kind) { "primary" -> "主账户"; "sub" -> "子账户"; else -> "账户" }
        return "币安$type · 账户指纹 ${account.take(8)}"
    }
}
