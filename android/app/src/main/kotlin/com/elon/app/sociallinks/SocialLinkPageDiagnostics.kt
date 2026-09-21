package com.elon.app.sociallinks

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.webkit.ConsoleMessage
import android.webkit.WebView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import com.elon.app.DebugTraceStore
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicLong

/** Per-WebView bounded observation. No JavaScript bridge, page content, URLs or console text. */
internal class SocialLinkPageDiagnostics(
    private val web: WebView,
    private val creationMs: Long,
    private val onUi: (Boolean, String) -> Unit,
    private val onReadable: () -> Unit,
) {
    val state = SocialLinkReaderLoadState()
    private val handler = Handler(Looper.getMainLooper())
    private val id = nextId.incrementAndGet()
    private val script = script(web)
    private val providerVersion = runCatching { WebViewCompat.getCurrentWebViewPackage(web.context)?.versionName.orEmpty().take(40) }.getOrDefault("")
    private var native = JSONObject()
    private var page = JSONObject()
    private var errors = JSONArray()
    private var pending: Runnable? = null
    private var deadline: Runnable? = null
    private var observationDeadline: Runnable? = null
    private var observing = false
    private var inFlight = false
    private var destroyed = false
    private var finishedAt = 0L
    private var lastReadyGeneration = -1L
    private var observedGeneration = -1L
    private var timeoutLogged = false
    private var documentStart = false
    private var sampleUntil = 0L

    init {
        if (runCatching { WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT) }.getOrDefault(false)) {
            documentStart = runCatching { WebViewCompat.addDocumentStartJavaScript(web, script, setOf("*")); true }.getOrDefault(false)
        }
    }

    fun started(url: String?) {
        cancelTimers(); state.start(now()); finishedAt = 0; inFlight = false; timeoutLogged = false
        native = JSONObject().put("session_id", id).put("navigation_id", state.generation)
            .put("host", SocialLinkPerformanceData.host(url)).put("webview_create_ms", creationMs)
            .put("webview_version", providerVersion)
            .put("document_start_observer", documentStart).put("started_at_ms", System.currentTimeMillis())
        page = JSONObject(); errors = JSONArray(); observeUntil(now() + 30000)
        updateUi(); publish("started", true)
        val generation = state.generation
        deadline = Runnable {
            if (valid(generation) && state.waiting) {
                timeoutLogged = true; updateUi(); publish("slow", true)
            }
        }.also { handler.postDelayed(it, 10000) }
    }

    fun committed() { if (!destroyed) { native.put("commit_visible_ms", elapsed()); publish("committed"); schedule(0) } }
    fun finished() {
        if (destroyed || state.failed) return
        state.finish(); finishedAt = now(); native.put("page_finished_ms", elapsed())
        // A late load completion gets a short final observation window after the initial deadline.
        observeUntil(maxOf(sampleUntil, now() + 2500))
        updateUi(); publish("load_finished"); schedule(0)
    }
    fun error(kind: String, url: String?, code: Int, mainFrame: Boolean) {
        if (destroyed) return
        if (errors.length() < 12) errors.put(JSONObject().put("kind", kind).put("host", SocialLinkPerformanceData.host(url))
            .put("code", code).put("main_frame", mainFrame))
        if (mainFrame) { state.fail(); cancelTimers(); updateUi(); publish("failed", true) }
        else publish("resource_error")
    }
    fun console(message: ConsoleMessage) {
        if (destroyed || message.messageLevel() !in setOf(ConsoleMessage.MessageLevel.ERROR, ConsoleMessage.MessageLevel.WARNING)) return
        val key = if (message.messageLevel() == ConsoleMessage.MessageLevel.ERROR) "console_errors" else "console_warnings"
        native.put(key, (native.optInt(key) + 1).coerceAtMost(1000))
        // Console message/source can contain credentials or page text; never store either.
    }
    fun resume() {
        if (destroyed || state.generation == 0L) return
        updateUi()
        if (state.waiting && !state.failed) { observeUntil(maxOf(sampleUntil, now() + 2500)); schedule(0) }
    }
    fun destroy() {
        if (destroyed) return
        publish("closed", true); destroyed = true; cancelTimers(); synchronized(recentLock) { recent.remove(id) }
    }
    private fun schedule(delay: Long) {
        pending?.let(handler::removeCallbacks)
        if (destroyed || state.failed || !observing) return
        pending = Runnable { poll() }.also { handler.postDelayed(it, delay) }
    }
    private fun poll() {
        if (destroyed || state.failed || !observing) return
        if (inFlight) { schedule(500); return }
        val generation = state.generation
        inFlight = true
        val expression = "(function(){if(!window.__elonLinkPerformance){$script}return window.__elonLinkPerformance?window.__elonLinkPerformance.snapshot():null})()"
        runCatching {
            web.evaluateJavascript(expression) { encoded ->
                if (!valid(generation) || !observing) return@evaluateJavascript
                inFlight = false
                if (encoded.length <= 16384) runCatching { JSONObject(encoded) }.getOrNull()?.let {
                    page = SocialLinkPerformanceData.sanitize(it)
                    if (page.optBoolean("content_visible") && !state.readable) confirmVisible(generation)
                }
                updateUi(); publish("sample")
                val settled = state.readable && state.finished && now() - finishedAt >= 1500
                if (settled || now() >= sampleUntil) {
                    stopObserving()
                    publish(if (settled) "settled" else "observation_timeout", true)
                } else schedule(if (elapsed() < 5000) 400 else 1000)
            }
        }.onFailure { inFlight = false; if (now() < sampleUntil) schedule(1000) else publish("observer_unavailable", true) }
    }
    private fun observeUntil(until: Long) {
        sampleUntil = until; observing = true
        observationDeadline?.let(handler::removeCallbacks)
        val generation = state.generation
        observationDeadline = Runnable {
            if (valid(generation) && observing) { stopObserving(); publish("observation_timeout", true) }
        }.also { handler.postDelayed(it, (until - now()).coerceAtLeast(0)) }
    }
    private fun stopObserving() {
        observing = false; inFlight = false
        pending?.let(handler::removeCallbacks); observationDeadline?.let(handler::removeCallbacks)
    }
    private fun confirmVisible(generation: Long) {
        if (observedGeneration == generation) return
        observedGeneration = generation
        native.put("content_observed_ms", elapsed())
        web.postVisualStateCallback(generation, object : WebView.VisualStateCallback() {
            override fun onComplete(requestId: Long) {
                if (!valid(generation) || state.failed) return
                state.visible(); native.put("visual_ready_ms", elapsed()); deadline?.let(handler::removeCallbacks)
                updateUi(); publish("content_visible", true)
                if (lastReadyGeneration != generation) { lastReadyGeneration = generation; onReadable() }
            }
        })
    }
    private fun updateUi() { onUi(state.waiting, state.message(now())) }
    private fun publish(stage: String, persist: Boolean = false) {
        val snapshot = JSONObject(native.toString()).put("stage", stage).put("elapsed_ms", elapsed())
            .put("readable", state.readable).put("network_finished", state.finished).put("failed", state.failed)
            .put("slow_notice", timeoutLogged).put("page", JSONObject(page.toString())).put("errors", JSONArray(errors.toString()))
        synchronized(recentLock) { recent[id] = snapshot; while (recent.size > 8) recent.remove(recent.keys.first()) }
        if (persist) {
            val serialized = snapshot.toString()
            traceWorker.execute { DebugTraceStore.record("external_reader_performance", mapOf("session_id" to id, "stage" to stage, "metrics" to serialized)) }
        }
    }
    private fun valid(generation: Long) = !destroyed && state.generation == generation
    private fun elapsed() = (now() - state.startedAt).coerceAtLeast(0)
    private fun cancelTimers() { stopObserving(); deadline?.let(handler::removeCallbacks); pending = null; deadline = null; observationDeadline = null }

    companion object {
        private val nextId = AtomicLong(System.currentTimeMillis())
        private val recentLock = Any()
        private val recent = linkedMapOf<Long, JSONObject>()
        private val traceWorker = Executors.newSingleThreadExecutor()
        @Volatile private var cachedScript: String? = null
        private fun now() = SystemClock.elapsedRealtime()
        private fun script(web: WebView): String = cachedScript ?: synchronized(recentLock) {
            cachedScript ?: web.context.assets.open("social_link_performance.js").bufferedReader().use { it.readText() }.also { cachedScript = it }
        }
        fun snapshots(): JSONArray = synchronized(recentLock) { JSONArray(recent.values.map { JSONObject(it.toString()) }) }
    }
}
