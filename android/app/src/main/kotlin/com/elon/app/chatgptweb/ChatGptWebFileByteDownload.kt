package com.elon.app.chatgptweb

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

internal class ChatGptWebFileByteDownload(
    private val context: Context,
    private val isCurrent: (ChatGptWebFileDownloadLease.Value) -> Boolean,
) {
    private class Job(val lease: ChatGptWebFileDownloadLease.Value, val expectedBytes: Long, val reply: (String) -> Unit) {
        val cancelled = AtomicBoolean()
        val startedAt = SystemClock.elapsedRealtime()
        var transfer: ChatGptWebFileByteTransfer? = null
        var received = 0L
        var savedUri: Uri? = null
        lateinit var notice: ChatGptWebFileByteNotifications
    }
    private val handler = Handler(Looper.getMainLooper())
    private val worker = Executors.newSingleThreadExecutor { task ->
        Thread(task, "chatgpt-file-save").apply { isDaemon = true }
    }
    private var active: Job? = null
    private var busy = false
    private var disposed = false
    private var expiry: Runnable? = null

    fun accept(
        value: JSONObject,
        lease: ChatGptWebFileDownloadLease.Value?,
        reply: (String) -> Unit,
    ) {
        val id = value.optString("leaseId")
        val operation = value.optString("byteOperation")
        val sequence = integer(value, "sequence")
        fun respond(state: String) {
            reply(JSONObject().put("leaseId", id).put("byteOperation", operation)
                .put("sequence", sequence ?: -1).put("state", state).toString())
        }
        if (disposed || busy || sequence == null || sequence < 0) { respond("failed"); return }
        if (operation == "begin") {
            val expectedBytes = integer(value, "expectedBytes")
            if (active != null || lease == null || sequence != 0L ||
                expectedBytes == null || expectedBytes !in -1L..ChatGptWebFileByteTransfer.MAX_BYTES) {
                respond("failed"); return
            }
            active = Job(lease, expectedBytes, reply)
            val job = checkNotNull(active)
            job.notice = ChatGptWebFileByteNotifications(context, lease) { cancel(lease.id) }
            job.notice.start()
            expiry = Runnable { if (active === job) cancel() }.also {
                handler.postDelayed(it, ChatGptWebFileByteTransfer.TIMEOUT_MS)
            }
        }
        val job = active
        if (job == null || job.lease.id != id || value.optString("documentToken") != job.lease.token ||
            !isCurrent(job.lease) || job.cancelled.get()) {
            if (job != null && job.lease.id == id && !isCurrent(job.lease)) cancel(id)
            respond("failed"); return
        }
        busy = true
        worker.execute {
            val state = runCatching {
                check(!job.cancelled.get() && SystemClock.elapsedRealtime() - job.startedAt < ChatGptWebFileByteTransfer.TIMEOUT_MS)
                when (operation) {
                    "begin" -> {
                        job.transfer = ChatGptWebFileByteTransfer(ChatGptWebFileByteStorage.open(context, job.lease) {
                            job.savedUri = it
                        }, job.expectedBytes)
                        "ready"
                    }
                    "chunk" -> {
                        job.received = checkNotNull(job.transfer).append(sequence, value.getString("data"))
                        "written"
                    }
                    "commit" -> {
                        checkNotNull(job.transfer).finish(sequence, checkNotNull(integer(value, "totalBytes")))
                        "saved"
                    }
                    else -> error("invalid_file_transfer_operation")
                }
            }.getOrElse {
                job.cancelled.set(true)
                runCatching { job.transfer?.cancel() }
                "failed"
            }
            handler.post {
                busy = false
                val current = !disposed && active === job && !job.cancelled.get() && isCurrent(job.lease)
                if (active === job) {
                    if (state == "saved") job.notice.saved(job.savedUri)
                    else if (state == "failed" || !current) job.notice.failed()
                    else job.notice.progress(job.received, job.expectedBytes)
                }
                if (state == "saved" || state == "failed" || !current) {
                    if (active === job) clearActive()
                    if (state != "saved") {
                        job.cancelled.set(true)
                        if (!worker.isShutdown) worker.execute { runCatching { job.transfer?.cancel() } }
                    }
                }
                // A saved receipt is only emitted after the storage provider publishes the complete file.
                respond(if (current) state else if (state == "saved") "saved" else "failed")
            }
        }
    }

    fun cancel(id: String? = null) {
        val job = active ?: return
        if (id != null && id != job.lease.id) return
        job.cancelled.set(true)
        job.notice.cancel()
        runCatching { job.reply(JSONObject().put("leaseId", job.lease.id).put("state", "cancelled").toString()) }
        clearActive()
        worker.execute { runCatching { job.transfer?.cancel() } }
    }

    private fun clearActive() {
        active = null
        expiry?.let(handler::removeCallbacks)
        expiry = null
    }

    fun dispose() {
        cancel()
        disposed = true
        worker.shutdown()
    }

    private fun integer(value: JSONObject, key: String): Long? = when (val number = value.opt(key)) {
        is Int -> number.toLong()
        is Long -> number
        else -> null
    }
}
