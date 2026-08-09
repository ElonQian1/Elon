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
    private var pendingAction: (() -> Unit)? = null
    private var pendingWebRequest: PermissionRequest? = null
    private val launcher = activity.registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        val webRequest = pendingWebRequest
        val action = pendingAction
        pendingWebRequest = null
        pendingAction = null
        if (granted) {
            webRequest?.grant(arrayOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE))
            action?.invoke()
        } else {
            webRequest?.deny()
            onDenied()
        }
    }

    fun runWithMicrophone(action: () -> Unit) {
        if (hasPermission()) {
            action()
            return
        }
        pendingAction = action
        launcher.launch(Manifest.permission.RECORD_AUDIO)
    }

    fun handle(request: PermissionRequest) {
        val allowedOrigin = request.origin.scheme == "https" &&
            request.origin.host == "chatgpt.com" &&
            request.origin.port == -1
        val audioOnly = request.resources.toSet() == setOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE)
        if (!allowedOrigin || !audioOnly) {
            request.deny()
            return
        }
        if (hasPermission()) {
            request.grant(arrayOf(PermissionRequest.RESOURCE_AUDIO_CAPTURE))
            return
        }
        pendingWebRequest?.deny()
        pendingWebRequest = request
        launcher.launch(Manifest.permission.RECORD_AUDIO)
    }

    fun cancel(request: PermissionRequest) {
        if (pendingWebRequest === request) pendingWebRequest = null
    }

    fun dispose() {
        pendingWebRequest?.deny()
        pendingWebRequest = null
        pendingAction = null
    }

    private fun hasPermission(): Boolean = ContextCompat.checkSelfPermission(
        activity,
        Manifest.permission.RECORD_AUDIO,
    ) == PackageManager.PERMISSION_GRANTED
}
