package com.elon.app

import android.content.SharedPreferences
import android.os.Handler
import android.os.Looper
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import okhttp3.OkHttpClient
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.security.MessageDigest

internal class AiConversationShareReader(
    private val activity: AppCompatActivity,
    http: OkHttpClient,
    private val serverUrl: String,
) : DefaultLifecycleObserver {
    private val api = AiConversationShareApi(activity.applicationContext, http, serverUrl)
    private val main = Handler(Looper.getMainLooper())
    private val worker = Executors.newFixedThreadPool(3)
    private val media = AiConversationShareReaderMedia(activity)
    private val positions = linkedMapOf<String, AiConversationShareReaderPosition>()
    private var view: AiConversationShareReaderView? = null
    private var card: AiConversationShareCard? = null
    private var continueAction: ((AiConversationShareSnapshot) -> Unit)? = null
    private var job: Future<*>? = null
    private val imageJobs = mutableListOf<Future<*>>()
    @Volatile private var generation = 0L
    @Volatile private var disposed = false
    @Volatile private var scope = accountScope()
    private val authChanges = SharedPreferences.OnSharedPreferenceChangeListener { _, _ ->
        main.post {
            if (scope != accountScope()) {
                close()
                positions.clear()
                scope = accountScope()
            }
        }
    }

    init {
        AuthManager.prefs(activity).registerOnSharedPreferenceChangeListener(authChanges)
        activity.lifecycle.addObserver(this)
    }

    /** Always authorize a new open. No cached transcript is rendered before API.read succeeds. */
    fun show(card: AiConversationShareCard, onDiscuss: () -> Unit = {}, onContinue: ((AiConversationShareSnapshot) -> Unit)? = null) {
        if (!canShow()) return
        close()
        scope = accountScope()
        this.card = card
        continueAction = onContinue
        val key = key(card)
        val screen = AiConversationShareReaderView(activity, card, onDiscuss, { position ->
            if (scope == accountScope() && position != null) remember(key, position)
            closeSession()
        }, ::refresh)
        view = screen
        screen.show()
        refresh()
    }

    /** Local preview only; its detached rows cannot write back to a live conversation. */
    fun showSnapshot(snapshot: AiConversationShareSnapshot, onDismiss: () -> Unit = {}) {
        if (!canShow()) return
        close()
        scope = accountScope()
        val frozen = snapshot.copy(messages = snapshot.messages.map { it.copyForSharing() }, gaps = snapshot.gaps.toSet())
        val screen = AiConversationShareReaderView(activity, frozen.card, null, {
            closeSession()
            onDismiss()
        }, null)
        view = screen
        screen.show()
        display(screen, frozen, null)
    }

    fun refresh() {
        val requested = card ?: return
        val screen = view ?: return
        screen.setContinueAction(null)
        val expectedScope = scope
        if (!validScope(expectedScope)) { close(); return }
        val position = screen.position() ?: positions[key(requested)]
        job?.cancel(true)
        imageJobs.forEach { it.cancel(true) }
        imageJobs.clear()
        val ticket = ++generation
        job = worker.submit {
            if (!validScope(expectedScope)) return@submit
            val result = runCatching { api.read(requested) }
            if (ticket == generation && validScope(expectedScope) && (result.isFailure || result.getOrNull()?.revoked == true)) {
                runCatching { api.invalidate(requested) }
            }
            main.post {
                if (ticket != generation || view !== screen) return@post
                if (!validScope(expectedScope)) { close(); return@post }
                result.fold(onSuccess = { snapshot ->
                    if (snapshot.card.id != requested.id || snapshot.card.groupId != requested.groupId) {
                        deny(screen, requested, false)
                    } else if (snapshot.revoked) deny(screen, requested, true)
                    else display(screen, snapshot, position)
                }, onFailure = {
                    // Fail closed for every read error, including 401/403/404/410.
                    deny(screen, requested, false)
                })
            }
        }
    }

    /** Parent calls this after successful revoke/recall or an access-denied group update. */
    fun invalidate(card: AiConversationShareCard) {
        val expected = scope
        if (!disposed) worker.submit { if (validScope(expected)) runCatching { api.invalidate(card) } }
        positions.remove(key(card))
        if (this.card?.id == card.id && this.card?.groupId == card.groupId) {
            generation++
            job?.cancel(true)
            view?.let { deny(it, card, true) }
        }
    }

    fun loadCover(card: AiConversationShareCard, onLoaded: (String?) -> Unit) {
        val assetId = card.coverAssetId ?: return onLoaded(null)
        if (!canShow()) return onLoaded(null)
        val expected = accountScope()
        worker.submit {
            if (!validScope(expected)) return@submit
            val result = runCatching { api.loadImage(card, assetId) }
            val denied = (result.exceptionOrNull() as? AiConversationShareApiException)?.accessDenied == true
            if (denied && validScope(expected)) runCatching { api.invalidate(card) }
            main.post {
                if (validScope(expected)) {
                    if (denied && this.card?.id == card.id && this.card?.groupId == card.groupId) {
                        view?.let { deny(it, card, false) }
                    }
                    onLoaded(result.getOrNull())
                }
            }
        }
    }

    fun close() {
        val current = view
        current?.dialog?.dismiss()
        closeSession()
    }

    fun dismiss() = close()
    fun destroy() = dispose()

    override fun onDestroy(owner: LifecycleOwner) = dispose()
    override fun onResume(owner: LifecycleOwner) {
        if (!validScope(scope)) close() else if (view != null && card != null) refresh()
    }

    fun dispose() {
        if (disposed) return
        close()
        disposed = true
        positions.clear()
        worker.shutdownNow()
        main.removeCallbacksAndMessages(null)
        AuthManager.prefs(activity).unregisterOnSharedPreferenceChangeListener(authChanges)
        activity.lifecycle.removeObserver(this)
    }

    private fun display(screen: AiConversationShareReaderView, snapshot: AiConversationShareSnapshot,
                        position: AiConversationShareReaderPosition?) {
        if (snapshot.revoked) { deny(screen, snapshot.card, true); return }
        val rows = AiConversationShareReaderPresentation.messages(snapshot, pendingImages = card != null)
        val adapter = ChatAdapter(rows.toMutableList(), readOnly = true).apply {
            onWebChatContentOpen = { _, part ->
                if (validScope(scope) && view === screen) {
                    if (part.type == "image" && part.imageSource == null && card != null) {
                        if (!part.previewPending) refresh()
                    } else media.open(part)
                }
            }
        }
        screen.render(snapshot, rows, adapter, position)
        val action = continueAction
        if (action != null) screen.setContinueAction {
            if (view === screen && validScope(scope)) action(snapshot)
        }
        if (card != null) loadImages(screen, snapshot.card, rows)
    }

    private fun loadImages(screen: AiConversationShareReaderView, card: AiConversationShareCard,
                           rows: List<ChatMessage>) {
        val ticket = generation
        val expected = scope
        val assets = rows.flatMap { row -> row.webChatMessage?.contentParts.orEmpty() }
            .filter { it.type == "image" && it.previewPending }
            .mapNotNull { it.assetHandle }.distinct().take(12)
        assets.forEach { assetId ->
            imageJobs += worker.submit {
                if (!validScope(expected) || ticket != generation) return@submit
                val result = runCatching { api.loadImage(card, assetId) }
                val denied = (result.exceptionOrNull() as? AiConversationShareApiException)?.accessDenied == true
                if (denied && ticket == generation && validScope(expected)) runCatching { api.invalidate(card) }
                main.post {
                    if (view !== screen || ticket != generation) return@post
                    if (!validScope(expected)) { close(); return@post }
                    if (denied) { deny(screen, card, false); return@post }
                    val path = result.getOrNull()
                    rows.forEachIndexed { index, message ->
                        val metadata = message.webChatMessage ?: return@forEachIndexed
                        if (metadata.contentParts.none { it.assetHandle == assetId }) return@forEachIndexed
                        message.webChatMessage = metadata.copy(contentParts = metadata.contentParts.map { part ->
                            if (part.assetHandle != assetId) part else part.copy(
                                imageSource = AiConversationShareReaderPresentation.localImageSource(path),
                                previewPending = false,
                                label = if (path == null) activity.getString(R.string.ai_conversation_share_image_unavailable) else part.label,
                            )
                        })
                        screen.messageChanged(index)
                    }
                }
            }
        }
    }

    private fun deny(screen: AiConversationShareReaderView, card: AiConversationShareCard, revoked: Boolean) {
        media.close()
        generation++
        imageJobs.forEach { it.cancel(true) }
        imageJobs.clear()
        positions.remove(key(card))
        screen.clear()
        screen.showNotice(if (revoked) R.string.ai_conversation_share_revoked
            else R.string.ai_conversation_share_unavailable, canRetry = !revoked)
    }

    private fun closeSession() {
        generation++
        job?.cancel(true)
        job = null
        imageJobs.forEach { it.cancel(true) }
        imageJobs.clear()
        media.close()
        view = null
        card = null
        continueAction = null
    }

    private fun remember(key: String, position: AiConversationShareReaderPosition) {
        positions.remove(key)
        positions[key] = position
        while (positions.size > 32) positions.remove(positions.keys.first())
    }

    private fun key(card: AiConversationShareCard) = "$serverUrl\n$scope\n${card.groupId}\n${card.id}"
    private fun accountScope(): String = MessageDigest.getInstance("SHA-256")
        .digest(socialSession(activity).toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }
    private fun validScope(expected: String) = !disposed && expected == accountScope() && canShow() &&
        !AuthManager.isSessionExpired(activity)
    private fun canShow() = !disposed && !activity.isFinishing && !activity.isDestroyed
}
