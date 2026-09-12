package com.elon.app.grid.host

/** Subscription ownership is separate from a trade/read grant and never authorizes business data. */
internal class BinanceEventSubscriptions<T> {
    data class Entry<T>(val uid:Int,val operation:String,val kind:String,val sink:T)
    private val entries=linkedMapOf<String,Entry<T>>()
    val size get()=entries.size
    fun snapshot():Map<String,Entry<T>> = entries.toMap()
    fun holds(kind:String,operation:String)=entries.values.any {it.kind==kind && it.operation==operation}
    fun add(id:String,uid:Int,operation:String,kind:String,sink:T) {
        require(Regex("[a-f0-9]{64}").matches(id) && Regex("[a-f0-9]{64}").matches(operation))
        require(kind in setOf("create","manage","read") && uid>=0 && id !in entries && entries.size<16)
        entries[id]=Entry(uid,operation,kind,sink)
    }
    fun remove(id:String,uid:Int?=null):Entry<T>? {
        require(uid==null || entries[id]?.uid in setOf(null,uid))
        return entries.remove(id)
    }
}
