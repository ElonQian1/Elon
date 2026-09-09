package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSessionRecoveryFailureTest {
    @Test
    fun finishedDocumentWithoutComposerIsNotReportedAsNetworkFailure() {
        val fixture = Fixture()
        fixture.coordinator.onPageFinished()
        fixture.expireThenReload()
        fixture.coordinator.onPageFinished()
        fixture.expire()

        val failure = fixture.failures.single()
        assertEquals(WebChatSessionRecoveryFailure.Kind.BRIDGE_TIMEOUT, failure.kind)
        assertTrue(failure.userMessage().contains("输入器尚未就绪"))
        assertFalse(failure.userMessage().contains("网络"))
    }

    @Test
    fun stalledMainDocumentReportsLoadingTimeout() {
        val fixture = Fixture()
        fixture.coordinator.onNavigationStarted()
        fixture.expireThenReload()
        fixture.expire()

        assertEquals(WebChatSessionRecoveryFailure.Kind.NAVIGATION_TIMEOUT, fixture.failures.single().kind)
    }

    @Test
    fun finishedErrorDocumentPreservesTheActualPageError() {
        val fixture = Fixture(repair = { true })
        fixture.coordinator.onFailure()
        fixture.expire()
        fixture.coordinator.onPageFinished()
        fixture.coordinator.onPageFailure("页面返回 HTTP 403")
        fixture.coordinator.onPageFinished()
        fixture.expire()

        assertEquals(WebChatSessionRecoveryFailure.Kind.PAGE_ERROR, fixture.failures.single().kind)
        assertEquals("页面返回 HTTP 403", fixture.failures.single().userMessage())
    }

    @Test
    fun newNavigationDoesNotReuseAnEarlierPageError() {
        val fixture = Fixture()
        fixture.coordinator.onPageFailure("页面证书校验失败")
        fixture.expire()
        fixture.coordinator.onNavigationStarted()
        fixture.coordinator.onPageFinished()
        fixture.expire()

        assertEquals(WebChatSessionRecoveryFailure.Kind.BRIDGE_TIMEOUT, fixture.failures.single().kind)
    }

    @Test
    fun retryThatCannotStartIsNotReportedAsNetworkFailure() {
        val fixture = Fixture(retry = { false })
        fixture.coordinator.onFailure()
        fixture.expire()

        assertEquals(WebChatSessionRecoveryFailure.Kind.RETRY_NOT_STARTED, fixture.failures.single().kind)
    }

    @Test
    fun readyAndManualRetryDiscardAnEarlierFailure() {
        val fixture = Fixture()
        fixture.coordinator.onPageFailure("页面返回 HTTP 403")
        fixture.coordinator.onReady()
        assertTrue(fixture.tasks.isEmpty())
        assertTrue(fixture.coordinator.retryNow())
        fixture.coordinator.onPageFinished()
        fixture.expire()

        assertEquals(WebChatSessionRecoveryFailure.Kind.BRIDGE_TIMEOUT, fixture.failures.single().kind)
    }

    @Test
    fun pausedOrFinishedRecoveryDoesNotEmitLateErrors() {
        val fixture = Fixture()
        fixture.coordinator.onNavigationStarted()
        val oldWatchdog = fixture.tasks.single()
        fixture.coordinator.deactivate()
        fixture.coordinator.onPageFailure("页面返回 HTTP 403")
        oldWatchdog.run()

        assertTrue(fixture.failures.isEmpty())
        assertTrue(fixture.tasks.isEmpty())
    }

    private class Fixture(retry: () -> Boolean = { true }, repair: () -> Boolean = { false }) {
        val tasks = mutableListOf<Runnable>()
        val failures = mutableListOf<WebChatSessionRecoveryFailure>()
        val coordinator = WebChatSessionRecoveryCoordinator(
            schedule = { task, _ -> tasks.add(task); Unit },
            cancel = { task -> tasks.removeAll { it === task }; Unit },
            retry = retry,
            repair = repair,
            onExhausted = { failures.add(it); Unit },
        ).also { it.activate() }

        fun expire() = tasks.removeAt(0).run()

        fun expireThenReload() {
            expire()
            expire()
        }
    }
}
