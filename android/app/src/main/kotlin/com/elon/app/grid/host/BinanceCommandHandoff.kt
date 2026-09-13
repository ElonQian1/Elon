package com.elon.app.grid.host

/** Consumer ownership can move; trade authority and unresolved request identity cannot. */
internal object BinanceCommandHandoff {
    fun adopt(kind:String,current:String,incoming:String,events:BinanceHostEvents,revokePreparation:()->Unit):Boolean {
        require(kind in setOf("create","manage"))
        require(Regex("[a-f0-9]{64}").matches(current) && Regex("[a-f0-9]{64}").matches(incoming))
        if(current==incoming || events.holds(kind,current) || !events.holds(kind,incoming))return false
        // Both checks and revocation run on the host's main thread before its operation changes.
        revokePreparation()
        return true
    }
}
