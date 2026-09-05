package com.elon.app.chatgptweb

internal class ChatGptComposerRefreshInterlock(
    private val suspendRefresh: () -> Unit,
    private val resumeRefresh: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val quietDelayMs: Long = DEFAULT_QUIET_DELAY_MS,
) {
    private var held = false
    private var scheduledRelease: Runnable? = null

    fun acquire() {
        cancelScheduledRelease()
        if (held) return
        held = true
        suspendRefresh()
    }

    fun releaseAfterQuietPeriod() {
        if (!held) return
        cancelScheduledRelease()
        lateinit var release: Runnable
        release = Runnable {
            if (scheduledRelease !== release) return@Runnable
            scheduledRelease = null
            releaseNow()
        }
        scheduledRelease = release
        schedule(release, quietDelayMs)
    }

    fun releaseNow() {
        cancelScheduledRelease()
        if (!held) return
        held = false
        resumeRefresh()
    }

    fun abandon() {
        cancelScheduledRelease()
        held = false
    }

    fun isHeld(): Boolean = held

    private fun cancelScheduledRelease() {
        scheduledRelease?.let(cancel)
        scheduledRelease = null
    }

    private companion object {
        const val DEFAULT_QUIET_DELAY_MS = 2_000L
    }
}
