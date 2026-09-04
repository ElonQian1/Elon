package com.elon.app.esk.handoff

import android.content.ComponentName
import android.content.Intent
import android.os.Bundle
import android.os.Build
import com.elon.eskcontract.EskSnapshotContract

internal fun readEskSnapshotRequest(intent: Intent?): Map<String, String>? = runCatching {
    intent ?: return null
    require(intent.action == EskSnapshotContract.ACTION && intent.flags == 0)
    require(intent.data == null && intent.type == null && intent.clipData == null && intent.selector == null)
    require(intent.categories.isNullOrEmpty())
    if (Build.VERSION.SDK_INT >= 29) require(intent.identifier == null)
    require(intent.component == ComponentName(ESK_MAIN_PACKAGE, ESK_CONSENT_ACTIVITY))
    require(intent.`package` == null || intent.`package` == ESK_MAIN_PACKAGE)
    val extras = intent.extras ?: return null
    require(extras.keySet() == EskSnapshotContract.REQUEST_KEYS)
    val fields = extras.keySet().associateWith { key ->
        @Suppress("DEPRECATION")
        val value = extras.get(key)
        (value as? String)?.takeIf { it.length <= 128 } ?: error("Expected string")
    }
    fields.takeIf(EskSnapshotContract::validRequest)
}.getOrNull()

internal fun eskSnapshotResult(fields: Map<String, String>, nonce: String, startedAt: Long, now: Long): Intent {
    require(EskSnapshotContract.validSnapshot(fields, nonce, startedAt, now))
    val extras = Bundle()
    EskSnapshotContract.KEYS.forEach { extras.putString(it, fields.getValue(it)) }
    return Intent().putExtras(extras)
}
