package com.elon.app.grid.wallet

/** A single foreground human click; refreshing or opening a page never grants access. */
internal class BinanceWalletConsentGate(private val elapsed:()->Long) {
    data class Identity(val owner:String,val account:String,val kind:String)
    sealed interface Decision {
        data object None:Decision
        data object Refresh:Decision
        data class Approve(val identity:Identity):Decision
    }
    private data class Intent(val identity:Identity,val started:Long)
    private var displayed:Identity?=null
    private var intent:Intent?=null
    private var active=false
    var notice:String?=null;private set
    val waiting get()=intent!=null
    fun remaining():Long= intent?.let {
        val age=elapsed()-it.started
        if(age in 0 until TIMEOUT_MS)TIMEOUT_MS-age else 0
    }?:0

    fun setActive(value:Boolean) {
        active=value
        if(!value)cancel()
    }
    fun cancel(message:String?=null) {intent=null;notice=message}

    fun observe(identity:Identity?,fresh:Boolean,busy:Boolean,failed:Boolean):Decision {
        val pending=intent
        var decision:Decision=Decision.None
        if(pending!=null) {
            when {
                !active -> cancel()
                identity!=pending.identity -> cancel("账户发生变化，请核对后重新同意。")
                failed -> cancel("账户确认失败，请重试或检查币安登录。")
                remaining()==0L -> cancel("账户确认超时，请重试。")
                fresh&&!busy -> {cancel();decision=Decision.Approve(pending.identity)}
            }
        }
        if(identity==null)displayed=null
        else if(fresh&&!busy)displayed=identity
        return decision
    }

    fun click(identity:Identity?,fresh:Boolean,busy:Boolean):Decision {
        if(!active||waiting)return Decision.None
        notice=null
        val reviewed=displayed
        if(identity==null||reviewed==null||identity!=reviewed)return Decision.Refresh
        if(fresh&&!busy)return Decision.Approve(reviewed)
        intent=Intent(reviewed,elapsed())
        return Decision.Refresh
    }

    companion object {const val TIMEOUT_MS=31_000L}
}
