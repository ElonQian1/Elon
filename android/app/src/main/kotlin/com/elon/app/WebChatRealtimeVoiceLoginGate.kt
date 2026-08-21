package com.elon.app

import android.app.AlertDialog
import android.graphics.Typeface
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal enum class WebChatRealtimeVoiceAuthenticationState {
    UNKNOWN,
    GUEST,
    AUTHENTICATED,
}

internal object WebChatRealtimeVoiceAuthenticationPolicy {
    fun resolve(authenticated: Boolean, sessionState: String): WebChatRealtimeVoiceAuthenticationState =
        when {
            authenticated -> WebChatRealtimeVoiceAuthenticationState.AUTHENTICATED
            sessionState.trim() in KNOWN_GUEST_STATES -> WebChatRealtimeVoiceAuthenticationState.GUEST
            else -> WebChatRealtimeVoiceAuthenticationState.UNKNOWN
        }

    private val KNOWN_GUEST_STATES = setOf("ready", "login_required")
}

internal data class WebChatRealtimeVoiceLoginMethod(
    val id: String,
    val label: String,
)

internal data class WebChatRealtimeVoiceLoginPresentation(
    val methods: List<WebChatRealtimeVoiceLoginMethod>,
) {
    companion object {
        val DEFAULT = WebChatRealtimeVoiceLoginPresentation(
            methods = listOf(
                WebChatRealtimeVoiceLoginMethod("google", "Google 账号"),
                WebChatRealtimeVoiceLoginMethod("apple", "Apple 账号"),
                WebChatRealtimeVoiceLoginMethod("phone", "电话号码"),
                WebChatRealtimeVoiceLoginMethod("email", "电子邮箱"),
            ),
        )
    }
}

internal interface WebChatRealtimeVoiceLoginGate {
    fun show(onOfficialLogin: () -> Unit, onCancel: () -> Unit)
    fun dismiss()
    fun isVisible(): Boolean
}

internal class WebChatRealtimeVoiceLoginDialog(
    private val activity: AppCompatActivity,
    private val presentation: WebChatRealtimeVoiceLoginPresentation =
        WebChatRealtimeVoiceLoginPresentation.DEFAULT,
) : WebChatRealtimeVoiceLoginGate {
    private var dialog: AlertDialog? = null

    override fun show(onOfficialLogin: () -> Unit, onCancel: () -> Unit) {
        if (dialog?.isShowing == true) return
        var actionHandled = false
        val created = AlertDialog.Builder(activity)
            .setTitle(R.string.web_chat_realtime_voice_login_title)
            .setView(buildContent())
            .setPositiveButton(R.string.web_chat_realtime_voice_login_open) { _, _ ->
                actionHandled = true
                onOfficialLogin()
            }
            .setNegativeButton(android.R.string.cancel) { _, _ ->
                actionHandled = true
                onCancel()
            }
            .create()
        created.setOnCancelListener {
            if (!actionHandled) {
                actionHandled = true
                onCancel()
            }
        }
        created.setOnDismissListener { dialog = null }
        dialog = created
        created.show()
        created.getButton(AlertDialog.BUTTON_POSITIVE).contentDescription =
            WebChatProductionSelectors.REALTIME_VOICE_LOGIN_OFFICIAL
        created.getButton(AlertDialog.BUTTON_NEGATIVE).contentDescription =
            WebChatProductionSelectors.REALTIME_VOICE_LOGIN_CANCEL
    }

    override fun dismiss() {
        dialog?.dismiss()
        dialog = null
    }

    override fun isVisible(): Boolean = dialog?.isShowing == true

    private fun buildContent(): View = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(24), dp(4), dp(24), dp(4))
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_LOGIN_SURFACE
        addView(text(activity.getString(R.string.web_chat_realtime_voice_login_message)))
        addView(text(activity.getString(R.string.web_chat_realtime_voice_login_methods)).apply {
            setTypeface(typeface, Typeface.BOLD)
            setPadding(0, dp(18), 0, dp(4))
        })
        presentation.methods.forEach { method ->
            addView(text("\u2022 ${method.label}").apply {
                setPadding(0, dp(8), 0, dp(8))
                contentDescription = "${WebChatProductionSelectors.REALTIME_VOICE_LOGIN_METHOD}:${method.id}"
            })
        }
        addView(text(activity.getString(R.string.web_chat_realtime_voice_login_privacy)).apply {
            alpha = 0.72f
            textSize = 13f
            setPadding(0, dp(12), 0, 0)
        })
    }

    private fun text(value: String) = TextView(activity).apply {
        text = value
        textSize = 16f
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
