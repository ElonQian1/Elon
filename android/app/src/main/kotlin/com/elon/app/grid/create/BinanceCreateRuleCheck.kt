package com.elon.app.grid.create

import com.elon.app.privateaccess.StrictJson

/** Bounded, read-only check before the existing preparation. No formula or trade transport lives here. */
internal class BinanceCreateRuleCheck(
    private val start:(String,String)->Unit,
    private val read:(String)->Map<String,Any?>,
    private val clear:()->Unit,
    private val schedule:(Runnable,Long)->Unit,
    private val unschedule:(Runnable)->Unit,
    private val elapsed:()->Long,
    private val wall:()->Long
) {
    private var generation=0L
    private var job:Runnable?=null

    fun begin(request:String,account:String,draft:BinanceGridDraft,complete:(Result<Long>)->Unit) {
        cancel()
        val ticket=generation;val began=elapsed()
        fun finish(result:Result<Long>) {
            if(ticket!=generation)return
            generation++;job?.let(unschedule);job=null
            complete(result)
        }
        val poll=Runnable {
            if(ticket==generation) {
                if(elapsed()-began !in 0 until 30_000L) {
                    finish(Result.failure(IllegalArgumentException("动态规则检查超时，请恢复连接后重新检查；未下单")))
                } else {
                    val result=runCatching {
                        val value=BinanceReferenceResult.parse(StrictJson.encode(read(request)),request,account,draft.symbol,wall())
                        when(value["status"]) {
                            "pending"->null
                            "ready"->validate(draft,value)
                            "expired"->throw IllegalArgumentException("动态规则已过期，请重新检查参数；未下单")
                            else->throw IllegalArgumentException("无法读取当前账号的币安动态规则，请恢复连接后重新检查；未下单")
                        }
                    }
                    if(result.isFailure)finish(Result.failure(publicFailure(result.exceptionOrNull())))
                    else result.getOrNull()?.let {finish(Result.success(it))} ?: job?.let {schedule(it,350)}
                }
            }
        }
        job=poll
        runCatching {
            val input=(BinanceCreateOptions.defaults+draft.input).filterKeys {it in BinanceReferenceInput.keys}
            start(request,StrictJson.encode(input))
        }.fold({schedule(poll,0)},{finish(Result.failure(publicFailure(it)))})
    }

    fun cancel() {generation++;job?.let(unschedule);job=null;clear()}

    private fun validate(draft:BinanceGridDraft,value:Map<String,Any?>):Long {
        val minimum=value["minimum_count"] as Long;val maximum=value["maximum_count"] as Long
        require(maximum>=minimum && value["code"]!="range_too_narrow") {"价格区间过窄，无法创建有效网格，请扩大区间；未下单"}
        val count=draft.input.getValue("count").toLong()
        require(count in minimum..maximum && value["code"]=="") {"当前价格区间可填 $minimum–$maximum 格，请调整网格数量；未下单"}
        val amount=(value["minimum_margin"] as String).toBigDecimal()
        require(draft.margin.toBigDecimal()>=amount) {"当前参数至少投入 ${amount.stripTrailingZeros().toPlainString()} USDT，请调整保证金或网格参数；未下单"}
        return value["observed_at"] as Long
    }

    private fun publicFailure(error:Throwable?):IllegalArgumentException {
        val message=(error as? IllegalArgumentException)?.message?.takeIf {it.endsWith("未下单")}
        return IllegalArgumentException(message ?: "动态规则回复未能验证，请重新检查参数；未下单")
    }

    companion object {
        fun fresh(observed:Long?,now:Long)=observed!=null && now-observed in -5000..120000
    }
}
