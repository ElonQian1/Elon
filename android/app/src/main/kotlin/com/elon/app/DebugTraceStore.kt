package com.elon.app

import android.content.Context
import android.os.SystemClock
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.util.ArrayDeque

object DebugTraceStore {
    private const val TAG = "ElonTrace"
    private const val MAX_EVENTS = 300
    private const val PREFS_NAME = "elon_debug_trace"
    private const val PREF_EVENTS = "events"
    private val lock = Any()
    private val events = ArrayDeque<TraceEvent>()
    @Volatile private var appContext: Context? = null
    @Volatile private var initialized = false

    data class TraceEvent(
        val wallTimeMs: Long,
        val elapsedMs: Long,
        val phase: String,
        val details: Map<String, String>
    )

    fun init(context: Context) {
        synchronized(lock) {
            if (initialized) return
            appContext = context.applicationContext
            loadPersistedLocked()
            initialized = true
        }
    }

    fun record(phase: String, details: Map<String, Any?> = emptyMap()) {
        val cleanDetails = details
            .filterValues { it != null }
            .mapValues { it.value.toString() }
        val event = TraceEvent(
            wallTimeMs = System.currentTimeMillis(),
            elapsedMs = SystemClock.elapsedRealtime(),
            phase = phase,
            details = cleanDetails
        )
        synchronized(lock) {
            events.addLast(event)
            while (events.size > MAX_EVENTS) {
                events.removeFirst()
            }
            persistLocked()
        }
        Log.i(TAG, formatLogLine(event))
    }

    fun recent(limit: Int = 80): JSONArray {
        val snapshot = synchronized(lock) {
            events.toList().takeLast(limit.coerceIn(1, MAX_EVENTS))
        }
        return JSONArray().apply {
            snapshot.forEach { put(it.toJson()) }
        }
    }

    fun count(): Int = synchronized(lock) { events.size }

    fun clear() {
        synchronized(lock) {
            events.clear()
            persistLocked()
        }
        Log.i(TAG, "phase=trace_clear")
    }

    private fun formatLogLine(event: TraceEvent): String {
        val details = event.details.entries.joinToString(" ") { (key, value) ->
            "$key=${value.replace('\n', ' ')}"
        }
        return buildString {
            append("phase=").append(event.phase)
            append(" elapsed_ms=").append(event.elapsedMs)
            if (details.isNotBlank()) {
                append(' ').append(details)
            }
        }
    }

    private fun TraceEvent.toJson(): JSONObject {
        return JSONObject().apply {
            put("wall_time_ms", wallTimeMs)
            put("elapsed_ms", elapsedMs)
            put("phase", phase)
            put("details", JSONObject().apply {
                details.forEach { (key, value) -> put(key, value) }
            })
        }
    }

    private fun loadPersistedLocked() {
        val stored = prefs()?.getString(PREF_EVENTS, null) ?: return
        val json = runCatching { JSONArray(stored) }.getOrNull() ?: return
        events.clear()
        for (index in 0 until json.length()) {
            val item = json.optJSONObject(index) ?: continue
            val phase = item.optString("phase").takeIf { it.isNotBlank() } ?: continue
            val detailsJson = item.optJSONObject("details") ?: JSONObject()
            val details = mutableMapOf<String, String>()
            val keys = detailsJson.keys()
            while (keys.hasNext()) {
                val key = keys.next()
                details[key] = detailsJson.optString(key)
            }
            events.addLast(
                TraceEvent(
                    wallTimeMs = item.optLong("wall_time_ms", 0L),
                    elapsedMs = item.optLong("elapsed_ms", 0L),
                    phase = phase,
                    details = details
                )
            )
            while (events.size > MAX_EVENTS) events.removeFirst()
        }
    }

    private fun persistLocked() {
        val prefs = prefs() ?: return
        val json = JSONArray().apply {
            events.forEach { put(it.toJson()) }
        }
        prefs.edit().putString(PREF_EVENTS, json.toString()).apply()
    }

    private fun prefs() =
        appContext?.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
}
