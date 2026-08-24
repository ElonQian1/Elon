package com.elon.app

import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity

internal object WebChatRealtimeVoiceOverlayHost {
    fun resolve(activity: AppCompatActivity): FrameLayout =
        requireNotNull(activity.findViewById(android.R.id.content)) {
            "MainActivity content overlay host is unavailable"
        }
}
