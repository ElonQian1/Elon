package com.elon.app.googleweb

import com.elon.app.WebBridgeConnectionState
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebBridgeReadinessPolicy
import org.junit.Assert.assertEquals
import org.junit.Test

class GoogleWebBridgeReadinessPolicyTest {
    @Test
    fun pageFinishKeepsTheCurrentGoogleAdapterReady() {
        val session = WebBridgeDocumentSession { "doc_google_1" }
        val loading = session.beginPage()
        val current = session.accept(loading.documentToken)!!

        assertEquals(
            WebBridgeConnectionState.READY,
            WebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = true,
                pageSupported = true,
                document = current,
            ),
        )
    }

    @Test
    fun aNewDocumentCannotReuseThePreviousGoogleAdapter() {
        val session = WebBridgeDocumentSession { generation -> "doc_google_$generation" }
        val first = session.beginPage()
        session.accept(first.documentToken)
        val second = session.beginPage()

        assertEquals(
            WebBridgeConnectionState.CONNECTING,
            WebBridgeReadinessPolicy.stateAfterPageReady(
                listenerInstalled = true,
                pageSupported = true,
                document = second,
            ),
        )
    }
}
