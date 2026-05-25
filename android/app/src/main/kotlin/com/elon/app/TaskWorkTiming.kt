package com.elon.app

internal fun estimateTaskEta(task: RunningTask): String? {
    val step = task.lastStep
    val total = task.lastStepTotal
    if (step <= 0 || step >= total) return null
    val elapsed = System.currentTimeMillis() - task.startedAtMs
    if (elapsed < 5_000L) return null
    val remaining = ((elapsed.toDouble() / step) * (total - step)).toLong()
    return when {
        remaining < 60_000L -> "不到 1 分钟"
        remaining < 120_000L -> "约 1 分钟"
        remaining < 300_000L -> "约 ${remaining / 60_000} 分钟"
        else -> null
    }
}

internal fun elapsedSinceRequestStart(task: RunningTask): Long {
    if (task.startedAtMs <= 0L) return 0L
    return System.currentTimeMillis() - task.startedAtMs
}

internal fun firstChatReplyElapsedMs(task: RunningTask): Long? {
    if (task.startedAtMs <= 0L || task.firstChatReplyAtMs <= 0L) return null
    return task.firstChatReplyAtMs - task.startedAtMs
}
