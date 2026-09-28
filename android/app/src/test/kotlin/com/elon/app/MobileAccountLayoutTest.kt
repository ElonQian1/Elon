package com.elon.app

import android.content.res.Configuration
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class, qualifiers = "mdpi")
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class MobileAccountLayoutTest {
    @Test fun loginAndRegistrationKeepInputsAndReadableActionsInCompactWindows() {
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(LoginActivity::class.java).use { controller ->
                val activity = controller.get()
                configure(activity, night, scale)
                controller.setup()
                val root = activity.findViewById<ViewGroup>(android.R.id.content).getChildAt(0)
                val account = root.findViewById<EditText>(R.id.loginAccountInput)
                val password = root.findViewById<EditText>(R.id.loginPasswordInput)
                val nickname = root.findViewById<EditText>(R.id.loginNicknameInput)
                account.setText("sample@example.invalid")
                password.setText("offline-fixture")
                nickname.setText("演示昵称")
                for (width in listOf(320, 411)) for (register in listOf(false, true, false)) {
                    root.findViewById<View>(if (register) R.id.loginTabRegister else R.id.loginTabLogin).performClick()
                    layout(root, width)
                    assertTrue(root is ScrollView)
                    assertEquals(register, root.findViewById<View>(R.id.loginTabRegister).isSelected)
                    assertEquals(if (register) View.VISIBLE else View.GONE, root.findViewById<View>(R.id.loginNicknameRow).visibility)
                    assertEquals(if (register) View.GONE else View.VISIBLE, root.findViewById<View>(R.id.loginGoogleButton).visibility)
                    assertEquals(if (register) View.GONE else View.VISIBLE, root.findViewById<View>(R.id.loginRecoveryButton).visibility)
                    assertEquals("sample@example.invalid", account.text.toString())
                    assertEquals("offline-fixture", password.text.toString())
                    assertEquals("演示昵称", nickname.text.toString())
                    assertTrue(password.inputType and android.text.InputType.TYPE_TEXT_VARIATION_PASSWORD != 0)
                    readable(root)
                    visibleNodes(root).filter { it.isClickable || it is EditText }.forEach {
                        assertTrue("Small touch target ${it.id}", it.width >= 48 && it.height >= 48)
                    }
                }
            }
        }
    }

    @Test fun securityFormAndSessionRowsGrowWithoutLosingTheSelectedSession() {
        for (night in listOf(false, true)) for (scale in listOf(1f, 2f)) {
            Robolectric.buildActivity(AppCompatActivity::class.java).use { controller ->
                val activity = controller.get(); configure(activity, night, scale); controller.setup()
                val root = LayoutInflater.from(activity).inflate(R.layout.activity_account_identities, null)
                val sessions = listOf(
                    AccountSecuritySession("current", "较长的当前设备名称", true, true, "2026-09-28 12:34", ""),
                    AccountSecuritySession("other", "另一个设备", false, false, null, "")
                )
                val revoked = mutableListOf<String>()
                val list = root.findViewById<LinearLayout>(R.id.accountSessionList)
                renderAccountSessions(activity, list, sessions) { revoked.add(it.id) }
                root.findViewById<TextView>(R.id.accountBindGoogleButton).text = "绑定 Google 到一个较长的演示账号"
                activity.setContentView(root)
                for (width in listOf(320, 411)) { layout(root, width); readable(root) }
                val controls = visibleNodes(list).filter { it.isClickable }
                assertEquals(2, controls.size)
                controls.forEach { assertTrue(it.height >= 48); it.performClick() }
                assertEquals(listOf("current", "other"), revoked)
                val labels = visibleNodes(list).filterIsInstance<TextView>().filter { !it.isClickable }
                assertTrue(labels.first().text.contains("当前设备"))
                labels.forEach { assertEquals(activity.getColor(R.color.mobile_on_surface), it.currentTextColor) }
            }
        }
    }

    private fun configure(activity: AppCompatActivity, night: Boolean, scale: Float) {
        activity.setTheme(R.style.Theme_ElonApp)
        @Suppress("DEPRECATION")
        activity.resources.updateConfiguration(Configuration(activity.resources.configuration).apply {
            fontScale = scale
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                if (night) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        }, activity.resources.displayMetrics)
    }
    private fun layout(root: View, width: Int) {
        root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(640, View.MeasureSpec.EXACTLY))
        root.layout(0, 0, width, 640)
    }
    private fun readable(root: View) {
        visibleNodes(root).filterIsInstance<TextView>().filter { it.text.isNotEmpty() }.forEach {
            assertTrue("Clipped ${it.text}", it.layout.height <= it.height - it.compoundPaddingTop - it.compoundPaddingBottom)
            for (line in 0 until it.layout.lineCount) assertEquals("Ellipsized ${it.text}", 0, it.layout.getEllipsisCount(line))
            assertTrue("Outside parent ${it.text}", it.right <= (it.parent as View).width)
        }
    }
    private fun visibleNodes(view: View): List<View> = if (view.visibility != View.VISIBLE) emptyList() else
        listOf(view) + if (view is ViewGroup) (0 until view.childCount).flatMap { visibleNodes(view.getChildAt(it)) } else emptyList()
}
