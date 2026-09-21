package com.elon.app.sociallinks

import android.app.Application
import android.os.Looper
import android.webkit.WebView
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.time.Duration

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class SocialLinkReaderPerformanceTest {
    @Test fun networkCompletionDoesNotHideTheBlankPageTimeout() {
        val state = SocialLinkReaderLoadState()
        state.start(100); state.finish()
        assertTrue(state.waiting)
        assertEquals("网页正在准备正文…", state.message(200))
        assertEquals("加载时间较长，可刷新或打开原文。", state.message(10100))
        state.visible(); assertFalse(state.waiting)
        state.start(20000); assertTrue(state.waiting); assertEquals(2L, state.generation)
        state.fail(); state.visible(); assertFalse(state.readable); assertFalse(state.waiting)
    }

    @Test fun pageDiagnosticsRejectContentUrlsAndUnboundedFields() {
        val raw = JSONObject().put("cookie", "secret").put("title", "private")
            .put("fcp_ms", -1).put("load_ms", 99999999).put("ttfb_ms", "123")
            .put("dns_ms", 12.0).put("content_visible", "true").put("content_state", "private")
            .put("slow_resources", JSONArray().put(JSONObject().put("host", "https://x.com/private?token=secret").put("duration_ms", 1))
                .put(JSONObject().put("host", "res.wx.qq.com").put("duration_ms", 123).put("kind", "script").put("url", "secret")))
            .put("resource_error_hosts", JSONArray().put("evil/path").put("res.wx.qq.com"))
        val result = SocialLinkPerformanceData.sanitize(raw)
        assertFalse(result.has("cookie")); assertFalse(result.has("fcp_ms")); assertFalse(result.has("load_ms")); assertFalse(result.has("ttfb_ms"))
        assertFalse(result.getBoolean("content_visible")); assertEquals("unknown", result.getString("content_state"))
        assertEquals(12.0, result.getDouble("dns_ms"), 0.0)
        assertEquals(1, result.getJSONArray("slow_resources").length())
        assertFalse(result.toString().contains("secret")); assertEquals(1, result.getJSONArray("resource_error_hosts").length())
        assertEquals("mp.weixin.qq.com", SocialLinkPerformanceData.host("https://mp.weixin.qq.com/s/private?token=secret"))
    }

    @Test fun successfulReadBackRunsOnceAndRepeatedStartsDoNotOverlap() {
        var calls = 0
        val task = SocialLinkReaderReadBack { current, complete -> assertTrue(current()); calls++; complete(true) }
        task.start(); task.start(); idle(25000)
        assertEquals(1, calls)
        task.start(); idle(1000); assertEquals(1, calls)
        task.reset()
    }

    @Test fun navigationInvalidatesPendingCallbacksAndCloseCancelsRetries() {
        val callbacks = mutableListOf<(Boolean) -> Unit>()
        val guards = mutableListOf<() -> Boolean>()
        val task = SocialLinkReaderReadBack { current, complete -> guards += current; callbacks += complete }
        task.start(); idle(800)
        assertTrue(guards[0]()); task.reset(); assertFalse(guards[0]())
        task.start(); callbacks[0](false); idle(800)
        assertEquals(2, callbacks.size)
        callbacks[1](false); task.reset(); idle(25000)
        assertEquals(2, callbacks.size)
    }

    @Test fun nonReturningRendererHasABoundedReadBackLifetime() {
        var calls = 0; var current: () -> Boolean = { false }
        val task = SocialLinkReaderReadBack { guard, _ -> calls++; current = guard }
        task.start(); idle(25000)
        assertEquals(1, calls); assertFalse(current())
        task.start(); idle(1000); assertEquals(1, calls)
        task.reset()
    }

    @Test fun nonReturningJavascriptStillReportsSlowAndStopsObservation() {
        val web = WebView(RuntimeEnvironment.getApplication())
        val diagnostics = SocialLinkPageDiagnostics(web, 2, { _, _ -> }, {})
        diagnostics.started("https://mp.weixin.qq.com/s/fixture?secret=private")
        diagnostics.committed(); diagnostics.finished(); idle(11000)
        var snapshot = SocialLinkPageDiagnostics.snapshots().getJSONObject(0)
        assertTrue(snapshot.getBoolean("slow_notice")); assertFalse(snapshot.getBoolean("readable"))
        idle(20000)
        snapshot = SocialLinkPageDiagnostics.snapshots().getJSONObject(0)
        assertEquals("observation_timeout", snapshot.getString("stage"))
        assertFalse(snapshot.toString().contains("private"))
        diagnostics.destroy(); web.destroy()
        assertEquals(0, SocialLinkPageDiagnostics.snapshots().length())
    }

    private fun idle(ms: Long) = shadowOf(Looper.getMainLooper()).idleFor(Duration.ofMillis(ms))
}
