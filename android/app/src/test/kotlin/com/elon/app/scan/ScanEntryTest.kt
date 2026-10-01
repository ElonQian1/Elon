package com.elon.app.scan

import android.app.Application
import android.content.Intent
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import com.elon.app.AuthManager
import com.elon.app.LoginActivity
import com.elon.app.PersonalQrCodeActivity
import com.elon.app.R
import com.elon.app.UserProfileStore
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ScanEntryTest {
    @Test fun imagePickerRemainsAvailableWithoutCameraPermission() {
        val lifecycle = Robolectric.buildActivity(ScanActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        lifecycle.setup()
        try {
            val root = activity.findViewById<View>(android.R.id.content)
            val button = descendants(root).filterIsInstance<TextView>().first { it.text.toString() == "从图片识别" }
            assertTrue(button.isEnabled); button.performClick()
            val intent = shadowOf(activity).nextStartedActivity
            assertEquals(Intent.ACTION_GET_CONTENT, intent.action)
            assertEquals("image/*", intent.type)
        } finally { lifecycle.pause().stop().destroy() }
    }
    @Test fun guestQrPageExplainsLoginAndDoesNotProduceSearchableDeviceCode() {
        val lifecycle = Robolectric.buildActivity(PersonalQrCodeActivity::class.java)
        val activity = lifecycle.get().apply { setTheme(R.style.Theme_ElonApp) }
        AuthManager.prefs(activity).edit().clear().commit()
        lifecycle.setup()
        try {
            assertTrue(runCatching { UserProfileStore.personalQrPayload(activity) }.isFailure)
            val texts = descendants(activity.findViewById(android.R.id.content)).filterIsInstance<TextView>().toList()
            assertTrue(texts.any { it.text.contains("游客设备身份") })
            texts.first { it.text.toString() == "登录 / 注册" }.performClick()
            assertEquals(LoginActivity::class.java.name, shadowOf(activity).nextStartedActivity.component?.className)
        } finally { lifecycle.pause().stop().destroy() }
    }
    private fun descendants(view: View): Sequence<View> = sequence {
        yield(view)
        if (view is ViewGroup) for (index in 0 until view.childCount) yieldAll(descendants(view.getChildAt(index)))
    }
}
