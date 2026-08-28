package com.elon.app.chatgptweb

import android.content.Context
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build

/** Owns the communication route only while the research native peer is alive. */
internal class ChatGptWebNativeVoiceAudioRoute(context: Context) {
    private val audioManager = context.getSystemService(AudioManager::class.java)
    private var acquired = false

    fun acquire(): Boolean {
        acquired = true
        audioManager.mode = AudioManager.MODE_IN_COMMUNICATION
        return routeCommunicationOutput()
    }

    fun release() {
        if (!acquired) return
        acquired = false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            runCatching { audioManager.clearCommunicationDevice() }
        } else {
            @Suppress("DEPRECATION")
            runCatching { audioManager.isSpeakerphoneOn = false }
        }
        runCatching { audioManager.mode = AudioManager.MODE_NORMAL }
    }

    private fun routeCommunicationOutput(): Boolean {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val devices = audioManager.availableCommunicationDevices
            val retainedExternal = audioManager.communicationDevice?.takeIf(::isExternalHeadset)
            val selected = retainedExternal
                ?: devices.firstOrNull(::isExternalHeadset)
                ?: devices.firstOrNull { it.type == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER }
            if (selected != null && runCatching {
                    audioManager.setCommunicationDevice(selected)
                }.getOrDefault(false)
            ) {
                return true
            }
        }
        @Suppress("DEPRECATION")
        return runCatching {
            audioManager.isSpeakerphoneOn = true
            audioManager.isSpeakerphoneOn
        }.getOrDefault(false)
    }

    private fun isExternalHeadset(device: AudioDeviceInfo): Boolean =
        device.type == AudioDeviceInfo.TYPE_BLUETOOTH_SCO ||
            device.type == AudioDeviceInfo.TYPE_BLE_HEADSET ||
            device.type == AudioDeviceInfo.TYPE_WIRED_HEADSET ||
            device.type == AudioDeviceInfo.TYPE_WIRED_HEADPHONES ||
            device.type == AudioDeviceInfo.TYPE_USB_HEADSET
}
