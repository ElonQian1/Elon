package com.elon.app.grid.host

import android.os.Binder
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Message
import android.os.Messenger

/** Authenticated, payload-free invalidations. Every business read still checks its own authority. */
internal class BinanceHostEvents(private val handler: Handler, private val membershipChanged: () -> Unit) {
    private data class Endpoint(val callback:IBinder,val death:IBinder.DeathRecipient)
    private val subscriptions=BinanceEventSubscriptions<Endpoint>()
    private val identity=Binder()
    private var revision=0L
    private val pending=linkedSetOf<String>()
    private val deliver=Runnable {
        val topics=pending.toList();pending.clear()
        for(topic in topics) {
            val next=++revision
            for((id,s) in subscriptions.snapshot()) runCatching {
                Messenger(s.sink.callback).send(Message.obtain().apply {
                    what=1;data=Bundle().apply {putString("subscription",id);putString("topic",topic);putLong("revision",next)}
                })
            }.onFailure {remove(id)}
        }
    }
    val active get()=subscriptions.size>0
    fun holds(kind:String,operation:String)=subscriptions.holds(kind,operation)
    fun call(method:String,uid:Int,extras:Bundle):Bundle {
        if(method=="events_capabilities_v1") {
            require(extras.isEmpty)
            return Bundle().apply {putString("schema",SCHEMA);putString("status","supported")}
        }
        val id=extras.getString("subscription").orEmpty();require(ID.matches(id))
        if(method=="events_unsubscribe_v1") {
            require(extras.keySet()==setOf("subscription"))
            remove(id,uid)
            return Bundle().apply {putString("status","closed")}
        }
        require(method=="events_subscribe_v1" && extras.keySet()==setOf("subscription","operation","kind","callback"))
        val operation=extras.getString("operation").orEmpty();val kind=extras.getString("kind").orEmpty()
        require(ID.matches(operation) && kind in setOf("create","manage","read"))
        val callback=extras.getBinder("callback") ?: error("CALLBACK_MISSING")
        require(id !in subscriptions.snapshot() && subscriptions.size<16)
        val death=IBinder.DeathRecipient {handler.post {remove(id)}}
        callback.linkToDeath(death,0)
        subscriptions.add(id,uid,operation,kind,Endpoint(callback,death));membershipChanged()
        return Bundle().apply {putString("schema",SCHEMA);putString("status","subscribed");putBinder("host",identity);putLong("revision",revision)}
    }
    fun changed(topic:String) {
        require(topic in setOf("state","read","reports","reference","funds"))
        if(!active)return
        pending.add(topic);handler.removeCallbacks(deliver);handler.post(deliver)
    }
    private fun remove(id:String,uid:Int?=null) {
        subscriptions.remove(id,uid)?.let {runCatching {it.sink.callback.unlinkToDeath(it.sink.death,0)};membershipChanged()}
    }
    companion object {
        const val SCHEMA="yilong.binance_host_events.v1"
        private val ID=Regex("[a-f0-9]{64}")
        val methods=setOf("events_capabilities_v1","events_subscribe_v1","events_unsubscribe_v1")
    }
}
