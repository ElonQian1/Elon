package com.elon.app.chatgptweb

import org.json.JSONObject

internal class ChatGptWebAudioPermissionState {
    enum class RequestState(val wireName: String) {
        IDLE("idle"),
        ANDROID_PERMISSION_PENDING("android_permission_pending"),
        LOCAL_ACTION_READY("local_action_ready"),
        WEB_PERMISSION_PENDING("web_permission_pending"),
        WEB_PERMISSION_GRANTED("web_permission_granted"),
        PERMISSION_DENIED("permission_denied"),
        WEB_REQUEST_REJECTED("web_request_rejected"),
        WEB_REQUEST_CANCELED("web_request_canceled"),
        DISPOSED("disposed"),
    }

    data class Snapshot(
        val androidPermissionGranted: Boolean?,
        val requestState: RequestState,
        val localRequestPending: Boolean,
        val webRequestPending: Boolean,
        val webPermissionGrantRevision: Long,
    )

    private var requestState = RequestState.IDLE
    private var localRequestPending = false
    private var webRequestPending = false
    private var webPermissionGrantRevision = 0L

    fun snapshot(androidPermissionGranted: Boolean): Snapshot = Snapshot(
        androidPermissionGranted = androidPermissionGranted,
        requestState = requestState,
        localRequestPending = localRequestPending,
        webRequestPending = webRequestPending,
        webPermissionGrantRevision = webPermissionGrantRevision,
    )

    fun localPermissionPending() {
        localRequestPending = true
        requestState = RequestState.ANDROID_PERMISSION_PENDING
    }

    fun localActionReady() {
        localRequestPending = false
        requestState = RequestState.LOCAL_ACTION_READY
    }

    fun webPermissionPending() {
        webRequestPending = true
        requestState = RequestState.WEB_PERMISSION_PENDING
    }

    fun webPermissionGranted() {
        webRequestPending = false
        webPermissionGrantRevision += 1
        requestState = RequestState.WEB_PERMISSION_GRANTED
    }

    fun permissionDenied() {
        localRequestPending = false
        webRequestPending = false
        requestState = RequestState.PERMISSION_DENIED
    }

    fun webRequestRejected() {
        if (webRequestPending) return
        requestState = RequestState.WEB_REQUEST_REJECTED
    }

    fun webRequestCanceled() {
        webRequestPending = false
        requestState = RequestState.WEB_REQUEST_CANCELED
    }

    fun dispose() {
        localRequestPending = false
        webRequestPending = false
        requestState = RequestState.DISPOSED
    }

    companion object {
        const val SCHEMA = "elon.chatgpt_web.audio_permission.v1"

        val UNOBSERVED = Snapshot(
            androidPermissionGranted = null,
            requestState = RequestState.IDLE,
            localRequestPending = false,
            webRequestPending = false,
            webPermissionGrantRevision = 0L,
        )
    }
}

internal object ChatGptWebAudioPermissionJson {
    fun encode(value: ChatGptWebAudioPermissionState.Snapshot): JSONObject = JSONObject()
        .put("schema", ChatGptWebAudioPermissionState.SCHEMA)
        .put(
            "android_permission",
            when (value.androidPermissionGranted) {
                true -> "granted"
                false -> "not_granted"
                null -> "unknown"
            },
        )
        .put("request_state", value.requestState.wireName)
        .put("local_request_pending", value.localRequestPending)
        .put("web_request_pending", value.webRequestPending)
        .put("audio_capture_state", "unobserved")
}
