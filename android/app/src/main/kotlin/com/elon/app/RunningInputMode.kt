package com.elon.app

internal enum class RunningInputMode(
    val label: String,
    val activeHint: String
) {
    REMIND_CURRENT("提醒当前", "发送给当前任务"),
    QUEUE_NEXT("排队下一轮", "当前任务结束后发送"),
    FORK("分叉探索", "新会话探索另一种方案")
}

internal const val QUEUED_NEXT_SEND_STATUS = "已排队下一轮"
