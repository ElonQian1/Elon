package com.elon.app.esk.platform.handoff

import android.content.ComponentName
import android.content.Intent
import android.os.Build
import android.os.Bundle
import com.elon.app.esk.platform.EskPlatformAccount
import com.elon.eskcontract.EskPlatformSnapshotContract

internal fun readEskPlatformSnapshotRequest(intent: Intent?): Map<String, String>? = runCatching {
    intent ?: return null
    require(intent.action == EskPlatformSnapshotContract.ACTION && intent.flags == 0)
    require(intent.data == null && intent.type == null && intent.clipData == null && intent.selector == null)
    require(intent.sourceBounds == null)
    require(intent.categories.isNullOrEmpty())
    if (Build.VERSION.SDK_INT >= 29) require(intent.identifier == null)
    require(intent.component == ComponentName(ESK_PLATFORM_MAIN_PACKAGE, ESK_PLATFORM_CONSENT_ACTIVITY))
    require(intent.`package` == null || intent.`package` == ESK_PLATFORM_MAIN_PACKAGE)
    val extras = intent.extras ?: return null
    require(extras.keySet() == EskPlatformSnapshotContract.REQUEST_KEYS)
    val fields = extras.keySet().associateWith { key ->
        @Suppress("DEPRECATION")
        val value = extras.get(key)
        (value as? String)?.takeIf { it.length <= 128 } ?: error("Expected string")
    }
    fields.takeIf(EskPlatformSnapshotContract::validRequest)
}.getOrNull()

/** Project an already validated formal account into the independent, identity-free IPC summary. */
internal fun composeEskPlatformSnapshot(account: EskPlatformAccount, nonce: String, startedAt: Long,
    observedAt: Long, expiresAt: Long): Map<String, String> {
    val fields = mapOf(
        "protocol" to EskPlatformSnapshotContract.PROTOCOL,
        "nonce" to nonce,
        "asset_id" to "esk",
        "symbol" to "ESK",
        "decimals" to "6",
        "source" to "platform_recorded",
        "chain_status" to "not_deployed",
        "simulated" to "false",
        "funds_moved" to "false",
        "verification_basis" to "authenticated_operator_review",
        "external_payment_verified" to "false",
        "total" to account.total,
        "total_base_units" to account.totalBaseUnits,
        "entry_count" to account.entryCount,
        "observed_elapsed_ms" to observedAt.toString(),
        "expires_elapsed_ms" to expiresAt.toString(),
        "service_spending" to "false",
        "quant_subscription" to "false",
        "sellback_settlement" to "false",
        "onchain_transfer" to "false",
        "chain_migration" to "false",
    )
    require(EskPlatformSnapshotContract.validSnapshot(fields, nonce, startedAt, observedAt))
    return fields
}

internal fun eskPlatformSnapshotResult(fields: Map<String, String>, nonce: String, startedAt: Long, now: Long): Intent {
    require(EskPlatformSnapshotContract.validSnapshot(fields, nonce, startedAt, now))
    val extras = Bundle()
    EskPlatformSnapshotContract.KEYS.forEach { extras.putString(it, fields.getValue(it)) }
    return Intent().putExtras(extras)
}
