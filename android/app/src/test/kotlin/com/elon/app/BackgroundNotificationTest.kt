package com.elon.app

import android.app.Application
import android.app.Service
import android.content.Intent
import com.elon.app.mcp.McpDebugKeepAliveService
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], manifest = Config.NONE, application = Application::class)
class BackgroundNotificationTest {
    private val context get() = RuntimeEnvironment.getApplication()

    @Before fun reset() {
        AuthManager.prefs(context).edit().clear().commit()
        ChatBackgroundPrefs.setKeepAliveEnabled(context, true)
        while (shadowOf(context).nextStartedService != null) { /* drain */ }
    }

    private fun login() {
        AuthManager.prefs(context).edit().putString("auth_token", "test-token").putString("auth_user_id", "u1").commit()
    }

    @Test fun oldRealtimeEntryOnlyStartsUnifiedServiceWhenLoggedIn() {
        ChatRealtimeService.ensureRunning(context)
        assertNull(shadowOf(context).nextStartedService)
        login()
        ChatRealtimeService.ensureRunning(context)
        assertEquals(ChatBackgroundService::class.java.name, shadowOf(context).nextStartedService.component?.className)
        assertNull(shadowOf(context).nextStartedService)
    }

    @Test fun notificationStopPersistsAndRejectsStickyRestartAndResume() {
        login()
        val controller = Robolectric.buildService(ChatBackgroundService::class.java).create()
        val service = controller.get()
        assertEquals(Service.START_NOT_STICKY, service.onStartCommand(Intent(context, ChatBackgroundService::class.java)
            .setAction(ChatBackgroundService.ACTION_STOP), 0, 1))
        assertFalse(ChatBackgroundPrefs.isKeepAliveEnabled(context))
        assertEquals(Service.START_NOT_STICKY, service.onStartCommand(null, 0, 2))
        ChatRealtimeService.ensureRunning(context)
        ChatBackgroundService.start(context)
        assertNull(shadowOf(context).nextStartedService)
        controller.destroy()
    }

    @Test fun oldImplicitDebugStateDoesNotOptUserIntoBackgroundDebug() {
        AuthManager.prefs(context).edit().putBoolean(McpDebugKeepAliveService.PREF_ACTIVE, true).commit()
        assertFalse(McpDebugKeepAliveService.shouldAutoStart(context))
        McpDebugKeepAliveService.resumeIfEnabled(context)
        assertNull(shadowOf(context).nextStartedService)
    }

    @Test fun explicitDebugStartAndStopShareDurablePreference() {
        McpDebugKeepAliveService.requestStart(context)
        assertTrue(McpDebugKeepAliveService.shouldAutoStart(context))
        assertEquals(McpDebugKeepAliveService::class.java.name, shadowOf(context).nextStartedService.component?.className)
        McpDebugKeepAliveService.requestStop(context)
        assertFalse(McpDebugKeepAliveService.shouldAutoStart(context))
        McpDebugKeepAliveService.resumeIfEnabled(context)
        assertNull(shadowOf(context).nextStartedService)
    }

    @Test fun debugNotificationStopSurvivesNextResume() {
        McpDebugKeepAliveService.requestStart(context)
        shadowOf(context).nextStartedService
        val controller = Robolectric.buildService(McpDebugKeepAliveService::class.java).create()
        assertEquals(Service.START_NOT_STICKY, controller.get().onStartCommand(
            Intent(context, McpDebugKeepAliveService::class.java).setAction(McpDebugKeepAliveService.ACTION_STOP), 0, 1))
        McpDebugKeepAliveService.resumeIfEnabled(context)
        assertFalse(McpDebugKeepAliveService.shouldAutoStart(context))
        assertNull(shadowOf(context).nextStartedService)
        controller.destroy()
    }
}
