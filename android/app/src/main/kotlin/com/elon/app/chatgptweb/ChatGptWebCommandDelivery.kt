package com.elon.app.chatgptweb

internal class ChatGptWebCommandDelivery(
    private val binding: () -> Binding?,
    private val invoke: (String, Binding, () -> Boolean, (String?) -> Unit) -> Unit,
    private val repair: (Binding, () -> Boolean, (Boolean) -> Unit) -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val cancel: (Runnable) -> Unit,
    private val observe: (String) -> Unit = {},
) {
    data class Binding(val token: String, val href: String)

    private class Delivery(
        val command: String,
        val binding: Binding,
        val epoch: Long,
        val unsent: (String) -> Unit,
    ) {
        var active = true
        var waitingRepair = false
        var invokedAgain = false
        var attempt = 0
        var timer: Runnable? = null
    }

    private class Repair(val binding: Binding, val epoch: Long) {
        val waiters = mutableListOf<Delivery>()
        var timer: Runnable? = null
    }

    private var epoch = 0L
    private val deliveries = mutableSetOf<Delivery>()
    private var repairing: Repair? = null

    fun send(command: String, unsent: (String) -> Unit) {
        val current = binding() ?: return unsent("unavailable")
        if (deliveries.size >= MAX_PENDING) return unsent("busy")
        val delivery = Delivery(command, current, epoch, unsent)
        deliveries += delivery
        val timer = Runnable {
            if (delivery.waitingRepair) reject(delivery, "repair_timeout")
            else finish(delivery, "unconfirmed")
        }
        delivery.timer = timer
        schedule(timer, TIMEOUT_MS)
        attempt(delivery)
    }

    fun invalidate() {
        epoch++
        repairing?.timer?.let(cancel)
        repairing = null
        deliveries.toList().forEach { finish(it, "invalidated") }
    }

    private fun current(delivery: Delivery): Boolean =
        delivery.active && delivery.epoch == epoch && binding() == delivery.binding

    private fun attempt(delivery: Delivery) {
        if (!current(delivery)) { finish(delivery, "stale"); return }
        val attempt = ++delivery.attempt
        try {
            invoke(delivery.command, delivery.binding, { current(delivery) }) { result ->
                if (!delivery.active || delivery.attempt != attempt) return@invoke
                delivery.attempt++ // A duplicate callback cannot trigger another command.
                when (result) {
                    "\"entered\"" -> finish(delivery, "entered")
                    "\"missing\"" -> {
                        if (!current(delivery)) finish(delivery, "stale")
                        else if (delivery.invokedAgain) reject(delivery, "missing")
                        else joinRepair(delivery)
                    }
                    "\"stale\"", "\"cancelled\"" -> finish(delivery, "stale")
                    else -> finish(delivery, "unconfirmed")
                }
            }
        } catch (_: Exception) {
            // An evaluator exception cannot prove whether the command ran.
            finish(delivery, "unconfirmed")
        }
    }

    private fun joinRepair(delivery: Delivery) {
        delivery.waitingRepair = true
        repairing?.let { job ->
            if (job.binding == delivery.binding && job.epoch == epoch) {
                job.waiters += delivery
                return
            }
            reject(delivery, "context_changed")
            return
        }
        val job = Repair(delivery.binding, epoch)
        job.waiters += delivery
        repairing = job
        job.timer = Runnable { repaired(job, false) }.also { schedule(it, TIMEOUT_MS) }
        observe("repair_started")
        try { repair(job.binding, {
            repairing === job && job.epoch == epoch && binding() == job.binding && job.waiters.any { current(it) }
        }) { ok -> repaired(job, ok) } }
        catch (_: Exception) { repaired(job, false) }
    }

    private fun repaired(job: Repair, ok: Boolean) {
        if (repairing !== job) return
        repairing = null
        job.timer?.let(cancel)
        job.waiters.toList().forEach { delivery ->
            if (!delivery.active) return@forEach
            if (!current(delivery)) { finish(delivery, "stale"); return@forEach }
            if (!ok) { reject(delivery, "repair_failed"); return@forEach }
            delivery.waitingRepair = false
            delivery.invokedAgain = true
            attempt(delivery)
        }
    }

    private fun reject(delivery: Delivery, reason: String) {
        if (!delivery.active) return
        val same = current(delivery)
        finish(delivery, reason)
        // Old navigation failures must not restore over a newly selected conversation.
        if (same) delivery.unsent(reason)
    }

    private fun finish(delivery: Delivery, reason: String) {
        if (!delivery.active) return
        delivery.active = false
        delivery.timer?.let(cancel)
        deliveries -= delivery
        if (reason != "entered") observe(reason)
        else if (delivery.invokedAgain) observe("repair_delivered")
    }

    private companion object {
        const val MAX_PENDING = 32
        const val TIMEOUT_MS = 5_000L
    }
}
