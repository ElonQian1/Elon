package com.elon.app

import android.os.SystemClock
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.util.ArrayDeque

object DebugTraceStore {
    private const val TAG = "ElonTrace"
    private const val MAX_EVENTS = 300
    private val lock = Any()
    private val events = ArrayDeque<TraceEvent>()

    data class TraceEvent(
        val wallTimeMs: Long,
        val elapsedMs: Long,
        val phase: String,
        val details: Map<String, String>
    )

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

    fun clear() {
        synchronized(lock) {
            events.clear()
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
}
