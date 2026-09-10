package com.elon.app.mcp

import android.content.Context
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.manage.BinanceManageReadBridge
import com.elon.app.grid.manage.BinanceHostReadDebug
import org.json.JSONObject

/** Bounded diagnostics only: no account IDs, strategy IDs, amounts, grants or website content. */
internal fun mcpBinanceHostStatus(context: Context): JSONObject = runCatching {
    BinanceHostRuntime.onMain(context) { host ->
        host.inspectPage()
        toolResult("Binance Android host status.", JSONObject()
            .put("schema", "yilong.binance_host_status.v1")
            .put("source", "android_webview")
            .put("host_present", host.view != null)
            .put("page_phase", host.pagePhase)
            .put("page_progress", host.view?.progress ?: 0)
            .put("adapter_bound", host.adapterBound)
            .put("read_recovery", JSONObject()
                .put("reload_used", host.readRecovery.reloadUsed)
                .put("reload_count", host.readRecovery.reloadCount))
            .put("page_diagnostics", JSONObject(host.diagnostics.facts))
            .put("diagnostics_observed_at_ms", host.diagnostics.observedAt)
            .put("page_origin", when {
                host.view?.url?.startsWith("${BinanceHostRuntime.ORIGIN}/") == true -> "binance"
                host.view?.url?.startsWith("https://accounts.binance.com/") == true -> "binance_login"
                else -> "none_or_other"
            })
            .put("main_session_current", host.live())
            .put("list_verified", host.live() && host.state.fresh())
            .put("row_count", if (host.live() && host.state.fresh()) host.state.count else 0)
            .put("detail_count", if (host.live() && host.state.fresh()) host.state.detailCount else 0)
            .put("active_grant_count", if (host.live()) host.state.activeGrantCount else 0)
            .put("account_kind", if (host.live()) host.state.accountKind else "unknown")
            .put("generation", host.state.generation)
            .put("observed_at_ms", host.state.observed)
            .put("coverage", "observed_response_only")
            .put("user_create_entry_available", true)
            .put("create_confirmation_open", host.onCreateObservation != null && BinanceManageReadBridge.facts()["page_open"] != true && !BinanceHostReadDebug.active(host))
            .put("read_debug_active", BinanceHostReadDebug.active(host))
            .put("manage_page", JSONObject(BinanceManageReadBridge.facts()))
            .put("mcp_trading_enabled", false)
            .put("trading_enabled", false))
    }
}.getOrElse { toolResult("Binance host did not respond.", JSONObject().put("error", "HOST_UNAVAILABLE"), isError = true) }
