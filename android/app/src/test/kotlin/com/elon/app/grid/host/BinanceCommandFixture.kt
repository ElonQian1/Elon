package com.elon.app.grid.host

import android.content.Context
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.Messenger
import com.elon.app.grid.create.BinanceCreateAttempt
import com.elon.app.grid.create.BinanceCreateCommands
import com.elon.app.grid.create.BinanceCreateJournal
import com.elon.app.grid.create.BinanceCreatePermit
import com.elon.app.grid.create.BinanceGridDraft
import com.elon.app.grid.manage.BinanceManageCommands
import com.elon.app.grid.manage.BinanceManageState
import com.elon.app.grid.manage.BinanceManageSnapshot
import com.elon.app.privateaccess.StrictJson
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows
import java.lang.reflect.InvocationTargetException

/** Real host/command objects with an empty local account store: no website or financial dispatch. */
internal class BinanceCommandFixture(val kind:String,val version:Int=2):AutoCloseable {
    val context:Context=RuntimeEnvironment.getApplication()
    val host=BinanceHostRuntime::class.java.getDeclaredConstructor(Context::class.java).apply{isAccessible=true}.newInstance(context)
    private val type=if(kind=="create")BinanceCreateCommands::class.java else BinanceManageCommands::class.java
    val controller:Any=type.getDeclaredConstructor(Context::class.java,BinanceHostRuntime::class.java).apply{isAccessible=true}.newInstance(context,host)
    val old="a".repeat(64)
    val next="b".repeat(64)
    val account="c".repeat(64)
    val document="doc_test_handoff"
    val topics=mutableListOf<String>()
    private val endpoints=mutableMapOf<String,Messenger>()
    val journal=BinanceCreateJournal(context,if(kind=="create")"binance-create-attempt-v1.json" else "binance-manage-attempt-v1.json")
    fun field(name:String):Any?=type.getDeclaredField(name).apply{isAccessible=true}.get(controller)
    fun set(name:String,value:Any)=type.getDeclaredField(name).apply{isAccessible=true}.set(controller,value)
    fun call(action:String,id:String=old,v:Int=version):Map<String,Any?> {
        val extras=Bundle().apply{putString("operation",id)}
        val result=try{type.getDeclaredMethod("call",String::class.java,Bundle::class.java).invoke(controller,"${kind}_${action}_v$v",extras) as Bundle}
        catch(e:InvocationTargetException){throw e.targetException}
        return StrictJson.parse(result.getString("result")!!,196608)
    }
    fun subscribe(id:String,subscriberKind:String=kind) {
        val messenger=Messenger(Handler(Looper.getMainLooper()){message->topics.add(message.data.getString("topic").orEmpty());true})
        endpoints[id]=messenger
        host.events.call("events_subscribe_v1",123,Bundle().apply{
            putString("subscription",id);putString("operation",id);putString("kind",subscriberKind);putBinder("callback",messenger.binder)
        })
    }
    fun unsubscribe(id:String){host.events.call("events_unsubscribe_v1",123,Bundle().apply{putString("subscription",id)});endpoints.remove(id)}
    fun idle()=Shadows.shadowOf(Looper.getMainLooper()).idle()
    fun seedCreate(status:String):BinanceCreateAttempt {
        val attempt=field("attempt") as BinanceCreateAttempt
        val draft=BinanceGridDraft.parse(mapOf("symbol" to "NEARUSDT","direction" to "LONG","spacing" to "ARITH","marginType" to "ISOLATED",
            "lower" to "1.01","upper" to "2.02","margin" to "12","leverage" to "1","count" to "10","autoInit" to "true","closeOnStop" to "true"))
        attempt.prepare(account,document,draft,"test_client_handoff",10)
        if(status!="prepared")attempt.start(account,document)
        if(status=="unknown")attempt.unknown()
        if(status in setOf("accepted","observed"))attempt.accepted("123","WORKING")
        if(status=="observed")attempt.detail("123","WORKING")
        if(attempt.unresolved)check(journal.save(attempt.journal()))
        return attempt
    }
    fun seedManage(status:String):BinanceManageState {
        val state=field("state") as BinanceManageState
        val snapshot=BinanceManageSnapshot("123","NEARUSDT","WORKING",false,true,false,false,false)
        state.prepare(account,document,"settings",true,snapshot)
        if(status!="prepared")state.start(account,document)
        if(status=="unknown")state.unknown()
        if(status in setOf("accepted","observed"))state.outcome("accepted","WORKING")
        if(status=="observed")state.observe(account,snapshot.copy(cps=true))
        if(state.unresolved)check(journal.save(state.journal()))
        return state
    }
    fun setBusy(value:Boolean){val session=field("session")!!;session.javaClass.getDeclaredField("busy").apply{isAccessible=true}.set(session,value)}
    fun busy():Boolean{val session=field("session")!!;return session.javaClass.getDeclaredField("busy").apply{isAccessible=true}.get(session) as Boolean}
    fun seedPermit() {
        (field("permit") as BinanceCreatePermit).bind(old,account,document,"d".repeat(64),"e".repeat(64))
        set("preparation","e".repeat(64));set("digest","d".repeat(64))
    }
    fun permitValid(id:String)=(field("permit") as BinanceCreatePermit).valid(id,account,document,"d".repeat(64),"e".repeat(64))
    override fun close() {
        endpoints.keys.toList().forEach(::unsubscribe)
        type.getDeclaredMethod("release",Boolean::class.javaPrimitiveType).apply{isAccessible=true}.invoke(controller,false)
        journal.save(null);host.invalidate("test finished");host.handler.removeCallbacksAndMessages(null)
    }
}
