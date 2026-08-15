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

    fun dispose() = audioPermissionController.dispose()
}
