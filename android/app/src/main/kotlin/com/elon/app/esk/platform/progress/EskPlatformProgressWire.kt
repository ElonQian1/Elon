package com.elon.app.esk.platform.progress

import android.content.ComponentName
import android.content.Intent
import android.os.Build
import android.os.Bundle
import com.elon.app.esk.platform.sellback.SellbackPage
import com.elon.app.esk.platform.sellback.sellbackAmount
import com.elon.eskcontract.EskPlatformProgressContract

internal fun readEskPlatformProgressRequest(intent: Intent?): Map<String, String>? = runCatching {
    intent ?: return null
    require(intent.action == EskPlatformProgressContract.ACTION && intent.flags == 0)
    require(intent.data == null && intent.type == null && intent.clipData == null && intent.selector == null)
    require(intent.sourceBounds == null && intent.categories.isNullOrEmpty())
    if (Build.VERSION.SDK_INT >= 29) require(intent.identifier == null)
    require(intent.component == ComponentName(ESK_PROGRESS_MAIN_PACKAGE, ESK_PROGRESS_CONSENT_ACTIVITY))
    require(intent.`package` == null || intent.`package` == ESK_PROGRESS_MAIN_PACKAGE)
    val extras = intent.extras ?: return null
    require(extras.keySet() == EskPlatformProgressContract.REQUEST_KEYS)
    val fields = extras.keySet().associateWith { key ->
        @Suppress("DEPRECATION")
        val value = extras.get(key)
        (value as? String)?.takeIf { it.length <= EskPlatformProgressContract.MAX_VALUE_LENGTH }
            ?: error("Expected string")
    }
    fields.takeIf(EskPlatformProgressContract::validRequest)
}.getOrNull()

/** Explicit projection from the strict private page. Never copy its records or policy wholesale. */
internal fun composeEskPlatformProgress(page: SellbackPage, nonce: String, cursor: String, startedAt: Long,
    observedAt: Long, expiresAt: Long): Map<String, String> {
    require(page.requests.size <= EskPlatformProgressContract.MAX_PAGE_COUNT)
    val summary = page.summary
    val fields = linkedMapOf(
        "protocol" to EskPlatformProgressContract.PROTOCOL, "nonce" to nonce, "requested_cursor" to cursor,
        "asset_id" to "esk", "symbol" to "ESK", "decimals" to "6",
        "source" to "platform_recorded", "chain_status" to "not_deployed",
        "simulated" to "false", "funds_moved" to "false",
        "verification_basis" to "authenticated_operator_review", "external_payment_verified" to "false",
        "total" to sellbackAmount(summary.total), "total_base_units" to summary.total.toString(),
        "reserved" to sellbackAmount(summary.reserved), "reserved_base_units" to summary.reserved.toString(),
        "available" to sellbackAmount(summary.available), "available_base_units" to summary.available.toString(),
        "snapshot_digest" to summary.digest, "request_count" to summary.count.toString(),
        "open_count" to summary.openCount.toString(), "range_start" to page.start.toString(),
        "range_end" to page.end.toString(), "page_count" to page.requests.size.toString(),
        "has_more" to (page.nextCursor != null).toString(), "next_cursor" to (page.nextCursor ?: ""),
        "observed_elapsed_ms" to observedAt.toString(), "expires_elapsed_ms" to expiresAt.toString(),
        "service_spending" to "false", "quant_subscription" to "false", "sellback_settlement" to "false",
        "onchain_transfer" to "false", "chain_migration" to "false",
        "submit_request" to "false", "cancel_request" to "false",
    )
    page.requests.forEachIndexed { index, record ->
        fields["request_${index}_id"] = record.id
        fields["request_${index}_amount"] = sellbackAmount(record.amount)
        fields["request_${index}_amount_base_units"] = record.amount.toString()
        fields["request_${index}_status"] = record.status
        fields["request_${index}_created_at"] = record.created
        fields["request_${index}_canceled_at"] = record.canceled ?: ""
    }
    require(EskPlatformProgressContract.validSnapshot(fields, nonce, cursor, startedAt, observedAt))
    return fields.toMap()
}

internal fun eskPlatformProgressResult(fields: Map<String, String>, nonce: String, cursor: String,
    startedAt: Long, now: Long): Intent {
    require(EskPlatformProgressContract.validSnapshot(fields, nonce, cursor, startedAt, now))
    val count = fields.getValue("page_count").toInt()
    val extras = Bundle()
    EskPlatformProgressContract.keysForCount(count).forEach { extras.putString(it, fields.getValue(it)) }
    return Intent().putExtras(extras)
}
