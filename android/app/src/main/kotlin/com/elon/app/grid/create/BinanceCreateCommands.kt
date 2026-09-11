package com.elon.app.grid.create

import android.content.Context
import android.os.Bundle
import android.os.SystemClock
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import java.security.SecureRandom
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Native consumer command service. No Activity or arbitrary network/script entry point is owned here. */
internal class BinanceCreateCommands private constructor(private val context: Context, private val host: BinanceHostRuntime) {
    private val attempt = BinanceCreateAttempt(SystemClock::elapsedRealtime)
    private val permit = BinanceCreatePermit(SystemClock::elapsedRealtime)
    private val journal = BinanceCreateJournal(context)
    private val market = BinanceGridMarket()
    private val reference = BinanceCreateReference(host)
    private var session: BinanceCreateSession? = null
    private var operation = ""
    private var preparation = ""
    private var digest = ""
    private var ownsSlot = false
    private var failedRecovery = false
    private var validating = false
    private var generation = 0L
    private var touched = 0L
    private var note = ""
    private val expiry = Runnable { expire() }

    fun call(method: String, extras: Bundle): Bundle {
        if(method=="create_reference_capabilities_v1") {
            require(extras.isEmpty)
            return reply(mapOf("schema" to "yilong.binance_create_reference_capabilities.v1","status" to "supported","version" to "1","read_only" to true))
        }
        if (method == "create_capabilities_v2") {
            require(extras.isEmpty)
            return reply(mapOf("schema" to SCHEMA, "status" to "supported", "version" to "2",
                "operations" to listOf("create"), "confirmation_owner" to "com.elon.quant"))
        }
        val allowed = when (method) {
            "create_prepare_v2" -> setOf("operation", "draft")
            "create_reference_v1" -> setOf("operation", "request", "draft")
            "create_reference_poll_v1" -> setOf("operation", "request")
            "create_submit_v2" -> setOf("operation", "preparation", "digest")
            else -> setOf("operation")
        }
        require(extras.keySet() == allowed)
        val id = extras.getString("operation") ?: error("OPERATION_MISSING")
        require(Regex("[a-f0-9]{64}").matches(id))
        if (method == "create_open_v2") {
            if (operation.isNotEmpty() && operation != id) return unavailable("busy", "另一创建流程正在进行，请返回原流程处理")
            if (!ownsSlot) {
                if (!BinanceCreateSlot.shared.acquire(this)) return unavailable("busy", "已有创建或管理操作占用连接")
                ownsSlot = true; operation = id
                failedRecovery = runCatching { journal.read()?.let(attempt::restore) }.isFailure
                session = BinanceCreateSession(host, attempt, ::persist, ::changed, reference)
                host.onCreateObservation = { session?.observed(it) }
            }
            host.begin()
        } else require(ownsSlot && operation == id) { "创建连接已结束，请重新连接" }
        touch()
        if(method in setOf("create_reference_v1","create_reference_poll_v1")) {
            require(!attempt.unresolved && !validating && session?.busy==false)
            val request=extras.getString("request") ?: error("REQUEST_MISSING")
            if(method=="create_reference_v1")reference.start(request,extras.getString("draft") ?: error("DRAFT_MISSING"))
            return reply(reference.snapshot(request)+("operation" to operation))
        }
        if (method in setOf("create_open_v2", "create_poll_v2")) host.recoverConnection()
        when (method) {
            "create_open_v2", "create_poll_v2" -> Unit
            "create_prepare_v2" -> runCatching { prepare(extras.getString("draft") ?: error("DRAFT_MISSING")) }
                .onFailure { note = safeMessage(it) }.let { Unit }
            "create_submit_v2" -> {
                require(!failedRecovery && !validating && session?.canSubmit() == true)
                permit.consume(id, host.state.account, host.document.snapshot().documentToken,
                    extras.getString("digest").orEmpty(), extras.getString("preparation").orEmpty())
                preparation = ""; session!!.submit()
            }
            "create_detail_v2" -> { note = ""; session!!.detail() }
            "create_cancel_v2" -> {
                require(attempt.status != "submitting")
                generation++; validating = false; permit.clear(); preparation = ""
                session!!.cancelPreparation(); note = "参数已变更，请重新检查"
            }
            "create_ack_v2" -> {
                require(!validating && attempt.status != "submitting" && !failedRecovery &&
                    attempt.account == host.state.account && attempt.unresolved) { "请连接原账号并核对本次结果" }
                require(journal.save(null))
                release(persistState = false)
                return unavailable("closed", "已结束本机核对记录；未撤单、未结束网格")
            }
            "create_close_v2" -> {
                if (attempt.status != "submitting") release()
                return unavailable("closed", "已返回；已发送请求仍需核对结果，不会自动重发")
            }
            else -> error("METHOD_UNSUPPORTED")
        }
        return snapshot()
    }

    private fun prepare(raw: String) {
        require(raw.length <= 8192 && !failedRecovery && !validating && session?.busy == false && !attempt.unresolved)
        session!!.cancelPreparation(); permit.clear(); preparation = ""
        require(host.live() && host.state.fresh()) { "请先完成币安连接与账号确认" }
        require(BinanceCreateJournal(context, "binance-manage-attempt-v1.json").read() == null) { "有尚未核对的管理操作" }
        val data = StrictJson.parse(raw, 8192)
        require(data.values.all { it is String })
        val draft = BinanceGridDraft.parse(data.mapValues { it.value as String })
        validating = true
        note = "正在校验合约规则，尚未下单"
        val ticket = ++generation
        val account = host.state.account
        val document = host.document.snapshot().documentToken
        try { worker.execute {
            val result = runCatching {
                val rule = market.rules().find { it.symbol == draft.symbol } ?: error("当前合约不在可交易列表")
                rule.validate(draft)
            }
            host.handler.post {
                if (ticket != generation || !ownsSlot) return@post
                validating = false
                if (!host.live() || host.state.account != account || host.document.snapshot().documentToken != document) {
                    note = "账号或连接已变化，请重新检查"; return@post
                }
                result.fold({
                    note = ""
                    runCatching { session!!.prepare(draft) }.onFailure { note = safeMessage(it) }
                }, { note = safeMessage(it) })
            }
        } } catch (_: java.util.concurrent.RejectedExecutionException) { validating = false; note = "规则查询仍在完成，请稍后重试" }
    }

    private fun changed() {
        if (attempt.status == "prepared" && preparation.isEmpty()) {
            digest = BinanceHostState.digest(StrictJson.encode(attempt.draft!!.input.toSortedMap()))
            preparation = randomId()
            permit.bind(operation, attempt.account, attempt.document, digest, preparation)
        }
    }
    private fun snapshot(): Bundle {
        val same = host.state.account != null && host.state.account == attempt.account
        val ready = host.live() && host.state.fresh()
        val canSubmit = !validating && session?.canSubmit() == true && permit.valid(operation,
            host.state.account, host.document.snapshot().documentToken, digest, preparation)
        val phase = when {
            failedRecovery -> "recovery_error"
            attempt.unresolved && !same -> "account_required"
            attempt.unresolved -> attempt.status
            !ready -> "connecting"
            validating || session?.busy == true -> "preparing"
            attempt.status == "prepared" && !canSubmit -> "expired"
            else -> attempt.status
        }
        return reply(mapOf("schema" to SCHEMA, "operation" to operation, "status" to phase,
            "message" to if (note.isNotEmpty()) note else session?.message.orEmpty(),
            "connection" to host.status, "account" to host.state.account.orEmpty(), "account_kind" to host.state.accountKind,
            "can_prepare" to (ready && !failedRecovery && !validating && session?.busy == false && !attempt.unresolved),
            "can_submit" to canSubmit, "preparation" to if (canSubmit) preparation else "",
            "digest" to if (canSubmit) digest else "", "summary" to if (same) attempt.draft?.summary().orEmpty() else "",
            "strategy_id" to if (same) attempt.strategyId else "", "provider_status" to if (same) attempt.providerStatus else "",
            "client_id" to if (same) attempt.clientId else "", "can_query" to (same && attempt.strategyId.isNotEmpty() && session?.busy == false)))
    }
    private fun persist() = if (attempt.unresolved) journal.save(attempt.journal()) else if (!failedRecovery) journal.save(null) else false
    private fun touch() {
        touched = SystemClock.elapsedRealtime(); host.keepAlive()
        host.handler.removeCallbacks(expiry); host.handler.postDelayed(expiry, 90_000)
    }
    private fun expire() { if (SystemClock.elapsedRealtime() - touched >= 90_000 && attempt.status != "submitting") release() }
    private fun release(persistState: Boolean = true) {
        reference.close()
        generation++; validating = false; permit.clear(); preparation = ""
        session?.close(persistState)
        host.onCreateObservation = null; session = null
        host.handler.removeCallbacks(expiry); BinanceCreateSlot.shared.release(this); ownsSlot = false; operation = ""
        instance = null
    }
    private fun reply(value: Map<String, Any?>) = Bundle().apply { putString("result", StrictJson.encode(value)) }
    private fun unavailable(status: String, message: String) = reply(mapOf("schema" to SCHEMA, "status" to status, "message" to message))
    private fun safeMessage(error: Throwable) = (error as? IllegalArgumentException)?.message?.take(160)
        ?: "合约规则或连接暂不可用，请稍后重新检查；未下单"
    companion object {
        const val SCHEMA = "yilong.binance_create_command.v2"
        val methods = setOf("create_capabilities_v2", "create_open_v2", "create_poll_v2", "create_prepare_v2",
            "create_submit_v2", "create_detail_v2", "create_cancel_v2", "create_ack_v2", "create_close_v2",
            "create_reference_v1", "create_reference_poll_v1", "create_reference_capabilities_v1")
        private var instance: BinanceCreateCommands? = null
        private val worker = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS, ArrayBlockingQueue(1))
        fun dispatch(context: Context, host: BinanceHostRuntime, method: String, extras: Bundle): Bundle =
            (instance ?: BinanceCreateCommands(context.applicationContext, host).also { instance = it }).call(method, extras)
        private fun randomId() = ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
    }
}
