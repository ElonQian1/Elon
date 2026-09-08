package com.elon.app.grid.manage

internal data class BinanceManageReadGate(val current:Boolean, val count:Int, val selected:Boolean,
    val unresolved:Boolean, val sameAccount:Boolean, val blocked:Boolean, val busy:Boolean, val prepared:Boolean) {
    val listState get()=if(!current) "unverified" else if(count==0) "empty" else "available"
    val readEnabled get()=current && !blocked && !busy && if(unresolved) sameAccount else selected
    val prepareEnabled get()=current && !blocked && !busy && !unresolved && selected
    fun permits(action:String)=when(action) {
        "status"->true
        "read"->readEnabled && !prepared
        "select"->current && count>0 && !blocked && !busy && !unresolved && !prepared
        "reload"->!blocked && !busy && !unresolved && !prepared
        else->false
    }
}

internal data class BinanceManageReadRequest(val action:String,val index:Int?=null) {
    companion object {
        val actions=setOf("status","select","read","reload")
        fun parse(args:Map<String,Any?>):BinanceManageReadRequest {
            require(args.keys.all{it in setOf("action","index","auth_token")})
            val action=if(args.containsKey("action"))args["action"] as? String else "status"
            require(action in actions)
            if(action!="select") {require(!args.containsKey("index"));return BinanceManageReadRequest(action!!)}
            val index=when(val value=args["index"]) {is Int->value.toLong();is Long->value;else->null}
            require(index!=null && index in 0..499)
            return BinanceManageReadRequest(action!!,index.toInt())
        }
    }
}

/** Structural events only. A verified read is never a transaction-success receipt. */
internal class BinanceManageReadTrace {
    var sequence=0L;private set
    var outcome="idle";private set
    var reason="none";private set
    fun start(){sequence++;outcome="pending";reason="none"}
    fun finish(value:String,code:String="none") {
        require(value in setOf("verified","failed","cancelled","unavailable","timeout"))
        outcome=value
        reason=code.takeIf{it in setOf("none","settings_unavailable","not_working","identity_changed",
            "identity_unavailable","detail_unavailable","verification_failed","malformed_reply")} ?: "verification_failed"
    }
    fun cancel(){if(outcome=="pending")finish("cancelled")}
}
