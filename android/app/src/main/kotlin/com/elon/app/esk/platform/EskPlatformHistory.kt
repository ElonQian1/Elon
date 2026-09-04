package com.elon.app.esk.platform

/** Whole-account summary plus one bounded page; not a second balance or chain proof. */
internal data class EskPlatformHistoryPage(
    val snapshotDigest: String,
    val total: String,
    val totalBaseUnits: String,
    val entryCount: String,
    val updatedAt: String?,
    val rangeStart: String,
    val rangeEnd: String,
    val entries: List<EskPlatformEntry>,
    val hasMore: Boolean,
    val nextCursor: String?,
)
