package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAudioPermissionStateTest {
    @Test
    fun tracksLocalPermissionWithoutClaimingAudioCapture() {
        val state = ChatGptWebAudioPermissionState()

        state.localPermissionPending()
        val pending = state.snapshot(androidPermissionGranted = false)
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.ANDROID_PERMISSION_PENDING,
            pending.requestState,
        )
        assertTrue(pending.localRequestPending)
        assertFalse(pending.webRequestPending)

        state.localActionReady()
        val ready = state.snapshot(androidPermissionGranted = true)
        assertEquals(ChatGptWebAudioPermissionState.RequestState.LOCAL_ACTION_READY, ready.requestState)
        assertEquals(true, ready.androidPermissionGranted)
        assertFalse(ready.localRequestPending)
    }

    @Test
    fun distinguishesWebGrantRejectionCancellationAndDenial() {
        val state = ChatGptWebAudioPermissionState()

        state.webPermissionPending()
        assertTrue(state.snapshot(false).webRequestPending)
        state.webRequestRejected()
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.WEB_PERMISSION_PENDING,
            state.snapshot(false).requestState,
        )
        state.webPermissionGranted()
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.WEB_PERMISSION_GRANTED,
            state.snapshot(true).requestState,
        )

        state.webRequestRejected()
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.WEB_REQUEST_REJECTED,
            state.snapshot(true).requestState,
        )
        state.webPermissionPending()
        state.webRequestCanceled()
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.WEB_REQUEST_CANCELED,
            state.snapshot(true).requestState,
        )
        state.permissionDenied()
        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.PERMISSION_DENIED,
            state.snapshot(false).requestState,
        )
    }

    @Test
    fun disposeClearsPendingRequests() {
        val state = ChatGptWebAudioPermissionState()
        state.localPermissionPending()
        state.webPermissionPending()

        state.dispose()

        val snapshot = state.snapshot(androidPermissionGranted = true)
        assertEquals(ChatGptWebAudioPermissionState.RequestState.DISPOSED, snapshot.requestState)
        assertFalse(snapshot.localRequestPending)
        assertFalse(snapshot.webRequestPending)
    }

    @Test
    fun reportsRejectedRequestWhenNoTrustedRequestIsPending() {
        val state = ChatGptWebAudioPermissionState()

        state.webRequestRejected()

        assertEquals(
            ChatGptWebAudioPermissionState.RequestState.WEB_REQUEST_REJECTED,
            state.snapshot(androidPermissionGranted = false).requestState,
        )
    }

    @Test
    fun jsonExportsOnlyStructuralPermissionEvidence() {
        val json = ChatGptWebAudioPermissionJson.encode(
            ChatGptWebAudioPermissionState.Snapshot(
                androidPermissionGranted = false,
                requestState = ChatGptWebAudioPermissionState.RequestState.WEB_PERMISSION_PENDING,
                localRequestPending = false,
                webRequestPending = true,
            ),
        )

        assertEquals(ChatGptWebAudioPermissionState.SCHEMA, json.getString("schema"))
        assertEquals("not_granted", json.getString("android_permission"))
        assertEquals("web_permission_pending", json.getString("request_state"))
        assertFalse(json.getBoolean("local_request_pending"))
        assertTrue(json.getBoolean("web_request_pending"))
        assertEquals("unobserved", json.getString("audio_capture_state"))
        assertEquals(6, json.length())
    }

    @Test
    fun unobservedJsonDoesNotClaimAndroidPermissionWasDenied() {
        val json = ChatGptWebAudioPermissionJson.encode(ChatGptWebAudioPermissionState.UNOBSERVED)

        assertEquals("unknown", json.getString("android_permission"))
        assertEquals("idle", json.getString("request_state"))
        assertEquals("unobserved", json.getString("audio_capture_state"))
    }
}
