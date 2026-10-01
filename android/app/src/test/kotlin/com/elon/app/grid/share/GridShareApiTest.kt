package com.elon.app.grid.share

import android.app.Application
import com.elon.app.AuthManager
import com.elon.app.socialSession
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.ResponseBody.Companion.toResponseBody
import okio.Buffer
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class GridShareApiTest {
    @Test fun requestUsesOnlyPublicDataAndRetriesSameSnapshotWithSameKey() {
        val context = RuntimeEnvironment.getApplication()
        AuthManager.saveSession(context, "synthetic-token", "owner", null, "Owner", null)
        val requests = mutableListOf<JSONObject>()
        val http = OkHttpClient.Builder().dns(object : okhttp3.Dns {
            override fun lookup(hostname: String): List<java.net.InetAddress> = throw AssertionError("No real HTTP allowed")
        }).addInterceptor { chain ->
            val request = chain.request(); val buffer = Buffer(); request.body!!.writeTo(buffer)
            requests.add(JSONObject(buffer.readUtf8()))
            Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(200).message("OK")
                .body("{\"snapshot_id\":\"ai_snapshot_test\"}".toResponseBody("application/json".toMediaType())).build()
        }.build()
        try {
            val api = GridShareApi(context, http, "https://synthetic.invalid")
            val session = socialSession(context)
            val grid = GridShareModel.project(mapOf("symbol" to "TESTUSDT", "id" to "private-strategy", "investment" to "1000"), 1000)
            api.publish("group_test", grid, session); api.publish("group_test", grid, session)
            assertEquals(requests[0].getString("idempotency_key"), requests[1].getString("idempotency_key"))
            assertFalse(requests[0].toString().contains("private-strategy"))
            val publicGrid = requests[0].getJSONObject("document").getJSONObject("grid")
            assertTrue(publicGrid.getBoolean("show_amounts"))
            assertEquals("1000", publicGrid.getJSONObject("fields").getString("investment"))
            AuthManager.saveSession(context, "next-token", "other", null, "Other", null)
            assertThrows(IllegalStateException::class.java) { api.publish("group_test", grid, session) }
            assertEquals(2, requests.size)
        } finally { AuthManager.prefs(context).edit().clear().commit(); http.connectionPool.evictAll(); http.dispatcher.executorService.shutdownNow() }
    }
    @Test fun positionProjectionNeverMixesSymbolsDirectionsOrRiskAccountBalances() {
        val report = JSONObject("""{"status":"ready","coverage":"strategy_position","rows":[{"symbol":"TESTUSDT","quantity":"-10.00","entry":"0.12","liquidation":"0","mark":"0.13","pnl":"-0.1","accountMarginBalance":"SECRET"}]}""")
        val result = GridSharePositions.project(report, "TESTUSDT")
        assertEquals("-10.00", result["positionQty"]); assertFalse(result.containsKey("liquidationPrice"))
        assertFalse(result.values.contains("SECRET"))
        assertThrows(IllegalStateException::class.java) { GridSharePositions.project(report, "OTHERUSDT") }
        report.getJSONArray("rows").put(report.getJSONArray("rows").getJSONObject(0))
        assertThrows(IllegalStateException::class.java) { GridSharePositions.project(report, "TESTUSDT") }
    }
}
