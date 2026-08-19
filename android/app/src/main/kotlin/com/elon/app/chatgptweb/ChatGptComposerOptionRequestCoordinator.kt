package com.elon.app.chatgptweb

internal class ChatGptComposerOptionRequestCoordinator(
    private val dismissMenu: (String?) -> Unit,
    private val dispatchRequest: (String, String?) -> Unit,
    private val collectOptions: (String) -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val prepareSection: (String) -> Unit = {},
    private val failSuperseded: (String, String) -> Unit = { _, _ -> },
    private val closeSettleMs: Long = DEFAULT_CLOSE_SETTLE_MS,
    private val menuSettleMs: Long = ChatGptWebInteractionTimings.COMPOSER_MENU_SETTLE_MS,
) {
    private var generation = 0L
    private var queuedRequest: Request? = null
    private var scheduledOpen: Runnable? = null
    private var scheduledCollection: Runnable? = null
    private var activeSection: String? = null

    fun request(section: String, requestId: String? = null): Boolean {
        if (section !in SUPPORTED_SECTIONS) return false
        cancelQueuedRequest()
        cancelScheduledCollection()
        generation += 1
        activeSection = null
        prepareSection(section)
        dismissMenu(null)

        val request = Request(section, requestId)
        queuedRequest = request
        lateinit var open: Runnable
        open = Runnable {
            if (scheduledOpen !== open || queuedRequest !== request) return@Runnable
            dispatchQueuedRequest()
        }
        scheduledOpen = open
        schedule(open, closeSettleMs)
        return true
    }

    fun onMenuDismissed() {
        dispatchQueuedRequest()
    }

    fun scheduleCollection(section: String): Boolean {
        if (section != activeSection) return false
        cancelScheduledCollection()
        val expectedGeneration = generation
        lateinit var collection: Runnable
        collection = Runnable {
            if (
                scheduledCollection !== collection ||
                generation != expectedGeneration ||
                activeSection != section
            ) {
                return@Runnable
            }
            scheduledCollection = null
            collectOptions(section)
        }
        scheduledCollection = collection
        schedule(collection, menuSettleMs)
        return true
    }

    fun complete(section: String) {
        if (activeSection != section) return
        activeSection = null
        cancelScheduledCollection()
    }

    fun dismiss(requestId: String? = null) {
        reset()
        dismissMenu(requestId)
    }

    fun reset() {
        cancelQueuedRequest()
        cancelScheduledCollection()
        generation += 1
        activeSection = null
    }

    fun isActive(): Boolean = queuedRequest != null || activeSection != null

    private fun cancelQueuedRequest() {
        scheduledOpen?.let(cancel)
        scheduledOpen = null
        queuedRequest?.requestId?.let { requestId ->
            failSuperseded(requestId, queuedRequest?.section.orEmpty())
        }
        queuedRequest = null
    }

    private fun dispatchQueuedRequest() {
        val request = queuedRequest ?: return
        scheduledOpen?.let(cancel)
        scheduledOpen = null
        queuedRequest = null
        activeSection = request.section
        dispatchRequest(request.section, request.requestId)
    }

    private fun cancelScheduledCollection() {
        scheduledCollection?.let(cancel)
        scheduledCollection = null
    }

    private data class Request(
        val section: String,
        val requestId: String?,
    )

    private companion object {
        val SUPPORTED_SECTIONS = setOf("model", "tools")
        const val DEFAULT_CLOSE_SETTLE_MS = 300L
    }
}
