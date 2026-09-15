package com.elon.app.chatgptweb

internal enum class ChatGptWebImagePreviewState {
    IDLE,
    PREPARING,
    FAILED,
}

internal class ChatGptWebImageAssetCoordinator(
    private val store: ChatGptWebImageAssetStore,
    private val request: (String) -> Boolean,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val dispatch: (() -> Unit) -> Unit,
    private val onChanged: () -> Unit,
) {
    private val queued = linkedSetOf<String>()
    private val attempts = mutableMapOf<String, Int>()
    private val failed = linkedSetOf<String>()
    private var activeHandle: String? = null
    private var timeout: Runnable? = null
    private var generation = 0L

    fun observe(snapshot: ChatGptWebSnapshot) {
        snapshot.messages.asSequence()
            .flatMap { message -> message.parts.asSequence() }
            .filter { part -> part.type == "image" }
            .mapNotNull { part -> part.metadata?.assetHandle }
            .filter { handle -> handle != activeHandle &&
                store.resolvePath(handle) == null && handle !in failed }
            .forEach(queued::add)
        pump()
    }

    fun accept(asset: ChatGptWebImageAsset) {
        if (asset.handle != activeHandle) return
        if (!asset.ready) {
            onAttemptFailed(asset.handle)
            return
        }
        failed.remove(asset.handle)
        queued.remove(asset.handle)
        val owner = generation
        store.save(asset) { saved ->
            dispatch {
                if (owner != generation || asset.handle != activeHandle) return@dispatch
                clearActiveTimeout()
                activeHandle = null
                if (!saved) failed += asset.handle
                onChanged()
                pump()
            }
        }
    }

    fun retry(handle: String) {
        if (!ChatGptWebImageAssetProtocol.validHandle(handle)) return
        if (handle == activeHandle) return
        failed.remove(handle)
        attempts.remove(handle)
        if (store.resolvePath(handle) == null) queued += handle
        pump()
    }

    fun retryMissing(snapshot: ChatGptWebSnapshot?) {
        failed.clear()
        attempts.clear()
        snapshot?.let(::observe)
    }

    fun resolvePath(handle: String): String? = store.resolvePath(handle)

    fun state(handle: String): ChatGptWebImagePreviewState = when {
        store.resolvePath(handle) != null -> ChatGptWebImagePreviewState.IDLE
        handle in failed -> ChatGptWebImagePreviewState.FAILED
        activeHandle == handle || handle in queued -> ChatGptWebImagePreviewState.PREPARING
        else -> ChatGptWebImagePreviewState.IDLE
    }

    fun state(): ChatGptWebImagePreviewState = when {
        activeHandle != null || queued.isNotEmpty() -> ChatGptWebImagePreviewState.PREPARING
        failed.isNotEmpty() -> ChatGptWebImagePreviewState.FAILED
        else -> ChatGptWebImagePreviewState.IDLE
    }

    fun reset() {
        generation++
        clearActiveTimeout()
        queued.clear()
        attempts.clear()
        failed.clear()
        activeHandle = null
    }

    private fun pump() {
        if (activeHandle != null) return
        while (queued.isNotEmpty()) {
            val handle = queued.first().also(queued::remove)
            if (handle in failed || store.resolvePath(handle) != null) continue
            if (!request(handle)) {
                failed += handle
                onChanged()
                continue
            }
            activeHandle = handle
            val task = Runnable {
                if (activeHandle == handle) onAttemptFailed(handle)
            }
            timeout = task
            schedule(task, REQUEST_TIMEOUT_MS)
            onChanged()
            return
        }
    }

    private fun onAttemptFailed(handle: String) {
        if (activeHandle == handle) {
            clearActiveTimeout()
            activeHandle = null
        }
        val nextAttempt = (attempts[handle] ?: 0) + 1
        attempts[handle] = nextAttempt
        if (nextAttempt < MAX_ATTEMPTS) {
            queued += handle
        } else {
            failed += handle
        }
        onChanged()
        pump()
    }

    private fun clearActiveTimeout() {
        timeout?.let(cancel)
        timeout = null
    }

    private companion object {
        const val MAX_ATTEMPTS = 2
        const val REQUEST_TIMEOUT_MS = 12_000L
    }
}
