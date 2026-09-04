package com.elon.app.esk.platform

/** Only validated, operator-reviewed platform records; no on-chain or spending claim. */
internal data class EskPlatformAccount(
    val total: String,
    val totalBaseUnits: String,
    val entryCount: String,
    val updatedAt: String?,
    val historyHasMore: Boolean,
    val entries: List<EskPlatformEntry>,
)

internal data class EskPlatformEntry(
    val entryId: String,
    val allocationId: String,
    val amount: String,
    val amountBaseUnits: String,
    val createdAt: String,
)
