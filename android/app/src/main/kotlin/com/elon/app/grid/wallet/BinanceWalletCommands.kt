package com.elon.app.grid.wallet

import android.os.Bundle
import com.elon.app.grid.host.BinanceHostRuntime

internal object BinanceWalletCommands {
    val methods=setOf("wallet_capabilities_v1","wallet_resume_v1","wallet_read_v1","wallet_refresh_v1","wallet_disconnect_v1")
    fun dispatch(host:BinanceHostRuntime,method:String,extras:Bundle):Bundle {
        val keys=when(method){"wallet_read_v1"->setOf("grant");"wallet_refresh_v1"->setOf("grant","request");else->emptySet()}
        require(method in methods&&extras.keySet()==keys)
        return when(method) {
            "wallet_capabilities_v1"->Bundle().apply {
                putString("schema","yilong.binance_wallet_capabilities.v1");putString("status","supported")
                putString("read_schema",BinanceWalletState.SCHEMA);putString("purpose",BinanceWalletState.PURPOSE)
                putString("consent_activity","com.elon.app.grid.wallet.BinanceWalletConsentActivity")
                putBoolean("trading_enabled",false)
            }
            "wallet_resume_v1"->host.wallet.resume()
            "wallet_disconnect_v1"->{host.wallet.disconnect();Bundle().apply {putString("status","revoked")}}
            "wallet_read_v1"->Bundle().apply {putString("result",host.wallet.read(extras.getString("grant")?:error("GRANT_MISSING")))}
            else->Bundle().apply {
                val request=host.wallet.refresh(extras.getString("grant")?:error("GRANT_MISSING"),extras.getString("request")?:error("REQUEST_MISSING"))
                putString("status","pending");putString("request",request)
            }
        }
    }
}
