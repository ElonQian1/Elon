package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceBackgroundContractTest {
    @Test
    fun pausedStateIsDistinctFromStandby() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "paused",
            turn = WebChatRealtimeVoiceTurn.IDLE,
            paused = true,
        )

        assertEquals(WebChatRealtimeVoiceVisibleState.PAUSED, WebChatRealtimeVoiceStatePolicy.visibleState(state))
        assertEquals(
            WebChatRealtimeVoiceBackgroundStatus.PAUSED,
            WebChatRealtimeVoiceBackgroundStatusPolicy.from(state),
        )
    }

    @Test
    fun unconfirmedHangupRemainsAnOngoingVoiceSession() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.HANGUP_UNCONFIRMED,
            detail = "still connected",
        )

        assertEquals(
            WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED,
            WebChatRealtimeVoiceStatePolicy.visibleState(state),
        )
        assertEquals(
            WebChatRealtimeVoiceBackgroundStatus.LISTENING,
            WebChatRealtimeVoiceBackgroundStatusPolicy.from(state),
        )
    }

    @Test
    fun stalePageObservationDoesNotMarkAnActiveCallAsAnError() {
        val state = WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "syncing",
            observation = WebChatRealtimeVoiceObservation.STALE,
        )

        assertEquals(
            WebChatRealtimeVoiceBackgroundStatus.LISTENING,
            WebChatRealtimeVoiceBackgroundStatusPolicy.from(state),
        )
    }

    @Test
    fun manifestAndServiceKeepBackgroundMicrophoneExplicitAndUserControlled() {
        val manifest = readRepositoryFile("android/app/src/main/AndroidManifest.xml")
        val service = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceBackgroundService.kt",
        )
        val session = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val overlay = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceSystemOverlay.kt",
        )

        assertTrue(manifest.contains("android.permission.FOREGROUND_SERVICE_MICROPHONE"))
        assertTrue(manifest.contains(".WebChatRealtimeVoiceBackgroundService"))
        assertTrue(manifest.contains("android:foregroundServiceType=\"microphone\""))
        assertTrue(service.contains("FOREGROUND_SERVICE_TYPE_MICROPHONE"))
        assertTrue(service.contains("AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK"))
        assertTrue(service.contains("requestPause(MEDIA)"))
        assertTrue(service.contains("requestResume(MEDIA)"))
        assertTrue(service.contains("ACTION_HANG_UP"))
        assertTrue(session.contains("if (realtimeVoiceBacking.isActive()) cookieManager.flush()"))
        assertTrue(overlay.indexOf("root.addView(panel)") < overlay.indexOf("root.addView(orb"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
