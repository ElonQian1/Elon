package com.elon.app.grid.manage

import android.content.Context
import android.os.Bundle
import android.os.SystemClock
import com.elon.app.grid.create.BinanceCreateJournal
import com.elon.app.grid.create.BinanceCreatePermit
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.privateaccess.StrictJson
import java.security.SecureRandom

/** Main-thread service for the authenticated quant UI; never opens a product Activity. */
internal class BinanceManageCommands private constructor(context: Context, private val host: BinanceHostRuntime) {
    private val state = BinanceManageState(SystemClock::elapsedRealtime)
    private val permit = BinanceCreatePermit(SystemClock::elapsedRealtime)
    private val journal = BinanceCreateJournal(context,"binance-manage-attempt-v1.json")
    private val creation = BinanceCreateJournal(context)
    private var session: BinanceManageSession? = null
    private var operation = ""
    private var preparation = ""
    private var digest = ""
    private var selected = ""
    private var note = ""
    private var corrupt = false
    private var creationPending = false
    private var touched = 0L
    private val expiry = Runnable { expire() }

    fun call(method: String, extras: Bundle): Bundle {
        if(method == "manage_capabilities_v2") {
            require(extras.isEmpty)
            return reply(mapOf("schema" to SCHEMA,"status" to "supported","version" to "2",
                "operations" to listOf("settings","close","investment","range"),"confirmation_owner" to "com.elon.quant"))
        }
        val fields = when(method) {
            "manage_read_v2" -> setOf("operation","id")
            "manage_prepare_v2" -> setOf("operation","draft")
            "manage_submit_v2" -> setOf("operation","digest","preparation")
            else -> setOf("operation")
        }
        require(extras.keySet() == fields && fields.all { extras.get(it) is String })
        val id = extras.getString("operation").orEmpty(); require(Regex("[a-f0-9]{64}").matches(id))
        if(method == "manage_open_v2") {
            if(operation.isNotEmpty() && operation != id) return short("busy","已有管理流程，请回原页面处理")
            if(operation.isEmpty()) {
                if(!BinanceCreateSlot.shared.acquire(this)) return short("busy","创建或管理连接正在使用中")
                operation = id
                corrupt = runCatching { journal.read()?.let(state::restore) }.isFailure
                creationPending = runCatching { creation.read() != null }.getOrDefault(true)
                session = BinanceManageSession(host,state,::persist,::changed)
                host.onCreateObservation = { session?.observed(it) }
            }
        } else require(operation == id && session != null)
        touch()
        if(method in setOf("manage_open_v2","manage_poll_v2")) {
            creationPending = runCatching { creation.read() != null }.getOrDefault(true)
            host.recoverConnection()
        }
        when(method) {
            "manage_open_v2","manage_poll_v2" -> Unit
            "manage_read_v2" -> read(extras.getString("id").orEmpty())
            "manage_prepare_v2" -> {
                require(!state.unresolved && session?.busy == false)
                cancel()
                runCatching {
                    require(available()) { "请先连接并授权读取当前账号" }
                    val draft = BinanceManageCommandDraft.parse(extras.getString("draft").orEmpty())
                    selected = draft.id
                    session!!.prepare(draft.id,draft.action,draft.cps,draft.amount,draft.range)
                }.onFailure { note = (it as? IllegalArgumentException)?.message?.take(180) ?: "参数检查未完成，未提交操作" }
            }
            "manage_submit_v2" -> {
                require(available() && session?.canSubmit() == true)
                permit.consume(id,host.state.account,host.document.snapshot().documentToken,
                    extras.getString("digest").orEmpty(),extras.getString("preparation").orEmpty())
                preparation = ""; session!!.submit()
            }
            "manage_cancel_v2" -> { require(!state.unresolved); cancel() }
            "manage_ack_v2" -> {
                require(available() && state.unresolved && state.status != "submitting" && state.account == host.state.account)
                require(journal.save(null)); release(false)
                return short("closed","仅结束本机核对记录；未撤单、未平仓、未结束策略")
            }
            "manage_close_v2" -> {
                if(state.status != "submitting") release()
                return short("closed","已退出；此前已发送操作仍须查询核对")
            }
            else -> error("METHOD_UNSUPPORTED")
        }
        return snapshot()
    }
    private fun available() = !corrupt && !creationPending && host.readConsentCurrent()
    private fun cancel() { permit.clear(); preparation = ""; digest = ""; session?.cancel(); note = "" }
    private fun read(id: String) {
        require(available() && session?.busy == false && state.status != "submitting")
        require(BinanceManageSnapshot.validId(id))
        require(!state.unresolved || state.id == id)
        cancel(); selected = id; session!!.read(id)
    }
    private fun changed() {
        if(state.status == "prepared" && preparation.isEmpty()) {
            digest = BinanceManageCommandView.digest(state); preparation = randomId()
            permit.bind(operation,state.account,state.document,digest,preparation)
        }
    }
    private fun snapshot(): Bundle {
        val authorized = host.readConsentCurrent()
        val same = authorized && (state.account == host.state.account || (!state.unresolved && session?.detailCurrent(selected) == true))
        val canSubmit = available() && session?.canSubmit() == true && permit.valid(operation,host.state.account,
            host.document.snapshot().documentToken,digest,preparation)
        val phase = when {
            corrupt -> "recovery_error"
            creationPending -> "creation_pending"
            state.unresolved && state.account != host.state.account -> "account_required"
            !host.live() || !host.state.fresh() -> "connecting"
            !authorized -> "consent_required"
            state.unresolved -> state.status
            session?.busy == true -> "reading"
            state.status == "prepared" && !canSubmit -> "expired"
            else -> state.status
        }
        val currentDetail = same && (state.unresolved || state.status == "prepared" || session?.detailCurrent(selected) == true)
        val current = state.snapshot?.takeIf { currentDetail }
        val choices = if(authorized) host.state.managementChoices().filter { BinanceManageSnapshot.validId(it.first) } else emptyList()
        return reply(linkedMapOf("schema" to SCHEMA,"operation" to operation,"status" to phase,
            "message" to note.ifEmpty { session?.message.orEmpty() },"connection" to host.status,
            "account" to if(authorized) host.state.account.orEmpty() else "","account_kind" to if(authorized) host.state.accountKind else "",
            "choices" to choices.map { mapOf("id" to it.first,"label" to it.second) },
            "can_read" to (available() && session?.busy == false && (!state.unresolved || same)),
            "can_prepare" to (available() && session?.busy == false && !state.unresolved && choices.isNotEmpty()),
            "can_submit" to canSubmit,"preparation" to if(canSubmit) preparation else "","digest" to if(canSubmit) digest else "",
            "summary" to if(current != null) BinanceManageCommandView.summary(state) else "",
            "detail" to current?.let(BinanceManageCommandView::detail),
            "strategy_id" to if(same && state.unresolved) state.id else "",
            "action" to if(same) state.action else "","effect_observed" to (current != null && state.unresolved && state.effectObserved())))
    }
    private fun persist() = if(state.unresolved) journal.save(state.journal()) else if(!corrupt) journal.save(null) else false
    private fun touch() { touched=SystemClock.elapsedRealtime(); host.keepAlive(); host.handler.removeCallbacks(expiry); host.handler.postDelayed(expiry,120_000) }
    private fun expire() {
        if(SystemClock.elapsedRealtime()-touched < 120_000) return
        if(state.status == "submitting") host.handler.postDelayed(expiry,15_000) else release()
    }
    private fun release(persistState: Boolean = true) {
        permit.clear(); session?.close(persistState); host.onCreateObservation=null; session=null
        host.handler.removeCallbacks(expiry); BinanceCreateSlot.shared.release(this); operation=""; instance=null
    }
    private fun reply(values: Map<String,Any?>) = Bundle().apply { putString("result",StrictJson.encode(values)) }
    private fun short(status: String,message: String) = reply(mapOf("schema" to SCHEMA,"status" to status,"message" to message))
    companion object {
        const val SCHEMA="yilong.binance_manage_command.v2"
        val methods=setOf("manage_capabilities_v2","manage_open_v2","manage_poll_v2","manage_read_v2","manage_prepare_v2",
            "manage_submit_v2","manage_cancel_v2","manage_ack_v2","manage_close_v2")
        private var instance: BinanceManageCommands?=null
        fun dispatch(context: Context,host: BinanceHostRuntime,method: String,extras: Bundle): Bundle =
            (instance ?: BinanceManageCommands(context.applicationContext,host).also { instance=it }).call(method,extras)
        private fun randomId()=ByteArray(32).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
    }
}
