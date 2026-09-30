package com.elon.app.esk.compute

import com.elon.app.esk.platform.EskPlatformJson
import java.math.BigInteger

/** Same money, freshness and source contract as PC; duplicate JSON keys are rejected. */
internal object EskComputeParser {
    fun parse(bytes: ByteArray, at: Long = System.currentTimeMillis()): EskComputeSnapshot {
        val root = EskPlatformJson.readObject(bytes)
        require(root["schema"] == "yilong.esk.compute_center.v1" && root["asset_source"] == "platform_recorded" &&
            root["chain_status"] == "not_deployed" && root["simulated"] == false && root["funds_moved"] == false)
        require(root["valuation_basis"] == "reference_only" && root["month_basis"] == "UTC_calendar_month")
        val observed = root.number("observed_at_ms")
        val until = root.number("fresh_until_ms")
        require(observed > 0 && observed <= at + 5000 && until > at && until > observed && until - observed <= 60_000)
        val asset = root.obj("asset")
        val total = asset.units("total_base_units")
        val reserved = asset.units("reserved_base_units")
        val remaining = asset.units("remaining_base_units")
        require(total == reserved + remaining)
        require(Regex("^[0-9a-f]{64}$").matches(asset.text("snapshot_digest")))
        val entryCount = asset.units("entry_count").toString()
        val cursor = asset.nullableText("history_next_cursor")
        require(cursor == null || Regex("^ephp1\\.[0-9a-f]{64}\\.eskp_entry_[0-9a-f]{32}$").matches(cursor))
        val quote = parseQuote(root, total, observed)
        val billing = root.obj("billing")
        require(billing["currency"] == "CNY")
        val page = billing.number("page").also { require(it in 1..1000) }.toInt()
        val capabilities = root.obj("capabilities")
        require(capabilities["purchase"] == false && capabilities["esk_service_spending"] == false &&
            root["esk_service_reserved_base_units"] == null && root["esk_service_spent_base_units"] == null)
        val balance = billing.nullableText("balance_fen")?.also { amount(it, 2) }
        val sourceSet = mutableSetOf<String>()
        val usage = root.rows("usage_sources", 6).map { row ->
            val source = row.text("billing_source")
            require(source in sourceLabels && sourceSet.add(source))
            CenterRow(sourceLabels.getValue(source), "${row.units("total_tokens")} Token", listOf(
                "输入" to row.units("input_tokens").toString(), "缓存输入" to row.units("cached_input_tokens").toString(),
                "输出" to row.units("output_tokens").toString(), "调用数量" to row.units("call_count").toString(),
                "计量说明" to if (source == "client_reported") "参考上报，不作为扣费依据" else "收费以实际账单为准",
            ))
        }
        return EskComputeSnapshot(observed, until, total.toString(), reserved.toString(), remaining.toString(),
            entryCount, quote, balance, billing.units("month_cost_fen").toString(),
            asset.rows("entries").map { row -> CenterRow(row.text("created_at"), "+${amount(row.units("amount_base_units").toString())} ESK",
                listOf("登记编号" to row.text("entry_id"), "审核记录" to row.text("allocation_id"), "到账依据" to "管理员审核登记")) },
            usage, billing.rows("bills").map(::bill), billing.rows("holds").map(::hold), page,
            billing.flag("has_more"), cursor != null, billing.flag("holds_has_more"))
    }

    private fun parseQuote(root: Map<String, Any?>, total: BigInteger, at: Long): CenterQuote? {
        if (root["quote"] == null) { require(root["valuation"] == null && root["quote_status"] != "fresh"); return null }
        val quote = root.obj("quote")
        quote.text("quote_id")
        val observed = quote.number("observed_at_ms")
        val until = quote.number("valid_until_ms")
        require(observed > 0 && observed <= at && until > at && until - observed <= 300_000)
        val rate = quote.units("usdt_per_esk_base_units").also { require(it.signum() > 0) }
        val cny = quote.units("cny_per_usdt_base_units").also { require(it.signum() > 0) }
        val valuation = if (root["valuation"] == null) null else root.obj("valuation")
        if (valuation != null) {
            require(root["quote_status"] == "fresh")
            require(valuation.units("usdt_base_units") == (total * rate + BigInteger.valueOf(500_000)) / BigInteger.valueOf(1_000_000))
            require(valuation.units("cny_base_units") == (total * rate * cny + BigInteger.valueOf(500_000_000_000)) / BigInteger.valueOf(1_000_000_000_000))
        }
        return CenterQuote(quote.text("source"), observed, until, valuation?.text("usdt_base_units"), valuation?.text("cny_base_units"))
    }

    private fun bill(row: Map<String, Any?>): CenterRow {
        val version = if (row["price_rule_version"] == null) row.text("price_source") else
            "v${row.number("price_rule_version").also { require(it > 0) }}"
        return CenterRow("${row.nullableText("feature") ?: "AI 服务"} · ${row.nullableText("model") ?: "模型未记录"}",
            "−¥${amount(row.units("cost_fen").toString(), 2)} CNY", listOf(
                "时间" to row.text("created_at"), "输入计费用量" to row.units("input_tokens").toString(),
                "缓存计费用量" to row.units("cached_input_tokens").toString(), "输出计费用量" to row.units("output_tokens").toString(),
                "价格版本" to version, "账单编号" to row.text("id"),
                "任务引用" to (row.nullableText("task_reference") ?: "未记录"),
                "计量回执" to (row.nullableText("token_usage_event_id") ?: "历史记录未关联")))
    }
    private fun hold(row: Map<String, Any?>): CenterRow {
        val label = holdLabels[row.text("status")] ?: error("Invalid hold")
        row.nullableText("expires_at")
        row.text("id")
        return CenterRow("${row.text("feature")} · $label", "¥${amount(row.units("reserved_fen").toString(), 2)} CNY",
            listOf("任务引用" to row.text("task_reference"), "模型" to (row.nullableText("model") ?: "模型未记录")))
    }
    @Suppress("UNCHECKED_CAST") private fun Map<String, Any?>.obj(key: String) = get(key) as? Map<String, Any?> ?: error("Invalid object")
    private fun Map<String, Any?>.text(key: String) = (get(key) as? String ?: error("Invalid text")).also {
        require(it.length <= 256 && it.none(Char::isISOControl)) }
    private fun Map<String, Any?>.nullableText(key: String): String? { require(containsKey(key)); return if (get(key) == null) null else text(key) }
    private fun Map<String, Any?>.units(key: String) = natural(text(key))
    private fun Map<String, Any?>.number(key: String) = ((get(key) as? EskPlatformJson.NumberToken)?.raw ?: error("Invalid number"))
        .also { require(Regex("^(0|[1-9][0-9]*)$").matches(it)) }.toLong()
    private fun Map<String, Any?>.flag(key: String) = get(key) as? Boolean ?: error("Invalid flag")
    @Suppress("UNCHECKED_CAST") private fun Map<String, Any?>.rows(key: String, maximum: Int = 20): List<Map<String, Any?>> =
        (get(key) as? List<*> ?: error("Invalid rows")).also { require(it.size <= maximum) }
            .map { it as? Map<String, Any?> ?: error("Invalid row") }
    private val sourceLabels = mapOf("platform" to "平台 AI", "own_codex" to "本人 AI 账号", "shared_codex" to "共享 AI 账号",
        "user_api_key" to "自带 API Key", "client_reported" to "客户端参考上报", "other" to "其他来源")
    private val holdLabels = mapOf("reserved" to "等待 AI 开始", "dispatch_hold" to "AI 任务执行中", "verification_hold" to "费用核对中")
}
