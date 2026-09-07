package com.elon.app

internal data class WebChatFileDownloadState(
    val requestId: String,
    val stage: Stage,
    val receivedBytes: Long = 0,
    val totalBytes: Long = -1,
) {
    enum class Stage(val wireName: String) {
        PREPARING("preparing"), TRANSFERRING("transferring"), SAVING("saving"),
        CANCELLING("cancelling"), CANCELLED("cancelled"), SAVED("saved"),
        QUEUED("queued"), FAILED("failed"), UNCONFIRMED("unconfirmed"),
    }

    val active: Boolean get() = stage in setOf(Stage.PREPARING, Stage.TRANSFERRING, Stage.SAVING, Stage.CANCELLING)
    val canCancel: Boolean get() = stage == Stage.PREPARING || stage == Stage.TRANSFERRING
    val progressPercent: Int? get() = if (totalBytes > 0) {
        ((receivedBytes.coerceAtMost(totalBytes) * 100) / totalBytes).coerceIn(0, 100).toInt()
    } else null
}
