package com.elon.app.mcp

import android.content.Context
import com.elon.app.grid.chat.BinanceGridReadRequest
import com.elon.app.grid.chat.BinanceGridReader
import com.elon.app.grid.host.BinanceHostRuntime
import org.json.JSONArray
import org.json.JSONObject

internal fun binanceGridReadTool(): JSONObject = tool(
    name = "binance_grid_read", title = "Binance Grid Read",
    description = "Read actual grid facts from this phone's authenticated Binance session. Explicit start=true begins a new read; poll the same request_id without start. List supports immutable pages; detail requires an exact current owned strategy_id. No trades, credentials, arbitrary requests or automatic chat sends.",
    properties = JSONObject()
        .put("kind", JSONObject().put("type", "string").put("enum", JSONArray(listOf("list", "detail"))))
        .put("request_id", JSONObject().put("type", "string").put("description", "Unique 8-96 character request ID. Reuse only for this snapshot."))
        .put("start", JSONObject().put("type", "boolean").put("default", false))
        .put("strategy_id", JSONObject().put("type", "string").put("description", "Exact positive decimal strategy ID, required for detail only."))
        .put("offset", JSONObject().put("type", "integer").put("minimum", 0).put("maximum", 500))
        .put("limit", JSONObject().put("type", "integer").put("minimum", 1).put("maximum", 50)),
    required = JSONArray(listOf("kind", "request_id")))

internal fun mcpBinanceGridRead(context: Context, args: JSONObject): JSONObject {
    val request = runCatching { BinanceGridReadRequest.parse(args.keys().asSequence().associateWith { args.get(it) }) }
        .getOrElse { return toolResult("Invalid grid read request.", JSONObject().put("error", "invalid_request"), isError = true) }
    return runCatching {
        BinanceHostRuntime.onMain(context) { host ->
            toolResult("Phone Binance grid read receipt.", JSONObject(BinanceGridReader.execute(host, request)))
        }
    }.getOrElse { toolResult("Grid reader unavailable.", JSONObject().put("error", "grid_reader_unavailable"), isError = true) }
}

internal fun binanceGridAttachmentTool(): JSONObject = tool(
    name = "binance_grid_attachment", title = "Binance Grid Chat Attachment",
    description = "Open the same explicit Attach Grid dialog as the active native ChatGPT composer, remove its draft snapshot, or inspect structural status. Never sends a chat or a trade. Opening initiates a read of this phone's own Binance session.",
    properties = JSONObject().put("action", JSONObject().put("type", "string").put("enum", JSONArray(listOf("status", "open", "remove")))),
    required = JSONArray(listOf("action")))

internal fun mcpBinanceGridAttachment(context: Context, args: JSONObject): JSONObject = runCatching {
    require(args.keys().asSequence().all { it in setOf("action", "auth_token") })
    val action = args.getString("action"); require(action in setOf("status", "open", "remove"))
    BinanceHostRuntime.onMain(context) {
        val controller = com.elon.app.grid.chat.GridChatAttachmentController.current
        if (controller == null) toolResult("Open native ChatGPT chat first.", JSONObject().put("error", "chat_inactive"), isError = true)
        else {
            if (action == "open") controller.open() else if (action == "remove") controller.remove()
            toolResult("Grid attachment status.", JSONObject(controller.status()))
        }
    }
}.getOrElse { toolResult("Grid attachment action unavailable.", JSONObject().put("error", "attachment_unavailable"), isError = true) }
