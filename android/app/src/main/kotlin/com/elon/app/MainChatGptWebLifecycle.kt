package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebAudioPermissionController

internal class MainChatGptWebLifecycle(activity: AppCompatActivity) {
    val audioPermissionController = ChatGptWebAudioPermissionController(activity) {
        Toast.makeText(
            activity,
            R.string.chatgpt_native_microphone_denied,
            Toast.LENGTH_LONG,
        ).show()
    }

    fun realtimeVoiceActivationEvidence(): WebChatRealtimeVoiceActivationEvidence =
        audioPermissionController.snapshot().let { snapshot ->
            WebChatRealtimeVoiceActivationEvidence(
                androidPermissionGranted = snapshot.androidPermissionGranted,
                webPermissionGrantRevision = snapshot.webPermissionGrantRevision,
                webRequestPending = snapshot.webRequestPending,
                requestState = snapshot.requestState.wireName,
            )
        }

    fun dispose() = audioPermissionController.dispose()
}
