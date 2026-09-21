package com.elon.app.sociallinks

/** Page completion and readable content are separate milestones. Uses a caller-supplied clock. */
internal class SocialLinkReaderLoadState {
    var startedAt = 0L; private set
    var generation = 0L; private set
    var finished = false; private set
    var readable = false; private set
    var failed = false; private set
    val waiting get() = !readable && !failed

    fun start(now: Long) {
        generation++; startedAt = now; finished = false; readable = false; failed = false
    }
    fun finish() { finished = true }
    fun visible() { if (!failed) readable = true }
    fun fail() { failed = true }
    fun message(now: Long): String = when {
        failed -> "页面加载失败，请刷新或打开原文。"
        readable -> "内容由原平台提供。"
        now - startedAt >= 10000 -> "加载时间较长，可刷新或打开原文。"
        finished -> "网页正在准备正文…"
        else -> "正在打开…"
    }
}
