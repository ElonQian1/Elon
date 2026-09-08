package com.elon.app.mcp

import android.content.Context
import com.elon.app.grid.host.BinanceHostRuntime
import org.json.JSONObject

/** Bounded diagnostics only: no account IDs, strategy IDs, amounts, grants or website content. */
internal fun mcpBinanceHostStatus(context: Context): JSONObject = runCatching {
    BinanceHostRuntime.onMain(context) { host ->
        toolResult("Binance Android host status.", JSONObject()
            .put("schema", "yilong.binance_host_status.v1")
            .put("source", "android_webview")
            .put("host_present", host.view != null)
            .put("main_session_current", host.live())
            .put("list_verified", host.live() && host.state.fresh())
            .put("row_count", if (host.live() && host.state.fresh()) host.state.count else 0)
            .put("generation", host.state.generation)
            .put("observed_at_ms", host.state.observed)
            .put("coverage", "observed_response_only")
            .put("trading_enabled", false))
    }
}.getOrElse { toolResult("Binance host did not respond.", JSONObject().put("error", "HOST_UNAVAILABLE"), isError = true) }
