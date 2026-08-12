package com.elon.app.chatgptweb

import android.Manifest
import android.content.pm.PackageManager
import android.webkit.PermissionRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

internal class ChatGptWebAudioPermissionController(
    private val activity: AppCompatActivity,
    private val onDenied: () -> Unit,
) {
    private val state = ChatGptWebAudioPermissionState()
    private var pendingAction: (() -> Unit)? = null
    private var pendingDeniedAction: (() -> Unit)? = null
    private var pendingWebRequest: PermissionRequest? = null
    private val launcher = activity.registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        val webRequest = pendingWebRequest
        val action = pendingAction
        val deniedAction = pendingDeniedAction
        pendingWebRequest = null
        pendingAction = null
        pendingDeniedAction = null
        if (granted) {
            webRequest?.grant(arrayOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE))
            when {
                webRequest != null -> state.webPermissionGranted()
                action != null -> state.localActionReady()
            }
            action?.invoke()
        } else if (webRequest != null || action != null || deniedAction != null) {
            webRequest?.deny()
            state.permissionDenied()
            (deniedAction ?: onDenied).invoke()
        }
    }

    fun runWithMicrophone(
        action: () -> Unit,
        onPermissionDenied: () -> Unit = onDenied,
    ) {
        if (hasPermission()) {
            state.localActionReady()
            action()
            return
        }
        pendingAction = action
        pendingDeniedAction = onPermissionDenied
        state.localPermissionPending()
        launcher.launch(Manifest.permission.RECORD_AUDIO)
    }

    fun handle(request: PermissionRequest) {
        val allowedOrigin = request.origin.scheme == "https" &&
            request.origin.host == "chatgpt.com" &&
            request.origin.port == -1
        val audioOnly = request.resources.toSet() == setOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE)
        if (!allowedOrigin || !audioOnly) {
            request.deny()
            state.webRequestRejected()
            return
        }
        if (hasPermission()) {
            request.grant(arrayOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE))
            state.webPermissionGranted()
            return
        }
        pendingWebRequest?.deny()
        pendingWebRequest = request
        pendingDeniedAction = null
        state.webPermissionPending()
        launcher.launch(Manifest.permission.RECORD_AUDIO)
    }

    fun cancel(request: PermissionRequest) {
        if (pendingWebRequest === request) {
            pendingWebRequest = null
            state.webRequestCanceled()
        }
    }

    fun snapshot(): ChatGptWebAudioPermissionState.Snapshot = state.snapshot(hasPermission())

    fun dispose() {
        pendingWebRequest?.deny()
        pendingWebRequest = null
        pendingAction = null
        pendingDeniedAction = null
        state.dispose()
    }

    private fun hasPermission(): Boolean = ContextCompat.checkSelfPermission(
        activity,
        Manifest.permission.RECORD_AUDIO,
    ) == PackageManager.PERMISSION_GRANTED
}
