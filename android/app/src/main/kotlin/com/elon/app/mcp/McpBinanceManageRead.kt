package com.elon.app.mcp

import android.content.Context
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.manage.BinanceManageReadBridge
import com.elon.app.grid.manage.BinanceManageReadRequest
import com.elon.app.grid.manage.BinanceHostReadDebug
import org.json.JSONArray
import org.json.JSONObject

internal fun binanceManageReadTool():JSONObject = tool(
    name="binance_manage_read",title="Binance Manage Read Only",
    description="Inspect grid read state, select a current list index, read details, or reload the official list. Uses the open management page or an isolated read-only host session. Never prepares or submits trades. Poll status for the read outcome.",
    properties=JSONObject()
        .put("action",JSONObject().put("type","string").put("enum",JSONArray(BinanceManageReadRequest.actions.toList())).put("default","status"))
        .put("index",JSONObject().put("type","integer").put("minimum",0).put("maximum",499)
            .put("description","Zero-based current verified list index; required only for select.")),required=JSONArray())

internal fun mcpBinanceManageRead(context:Context,args:JSONObject):JSONObject {
    val request=runCatching{BinanceManageReadRequest.parse(args.keys().asSequence().associateWith{args.get(it)})}
        .getOrElse{return toolResult("Unsupported read-only request.",JSONObject().put("error","INVALID_READ_REQUEST"),isError=true)}
    return runCatching {
        BinanceHostRuntime.onMain(context) { host ->
            val result=if(BinanceManageReadBridge.facts()["page_open"]==true)BinanceManageReadBridge.execute(request)
                else BinanceHostReadDebug.execute(host,request)
            toolResult("Binance management read-only diagnostics.",JSONObject(result))
        }
    }.getOrElse {toolResult("Management page did not respond.",JSONObject().put("error","MANAGE_READ_UNAVAILABLE"),isError=true)}
}
