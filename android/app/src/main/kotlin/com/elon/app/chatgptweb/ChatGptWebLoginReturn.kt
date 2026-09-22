package com.elon.app.chatgptweb

import android.app.Activity
import android.content.Intent
import android.view.Gravity
import android.view.View
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R

/** Opt-in login navigation; ordinary official-page browsing keeps its existing back stack. */
internal class ChatGptWebLoginReturn(private val activity: AppCompatActivity) {
    val enabled = activity.intent.getBooleanExtra(EXTRA_RETURN_AFTER_LOGIN, false)
    private val completion = ChatGptWebLoginCompletion(enabled)

    fun onSnapshot(snapshot: ChatGptWebSnapshot) {
        if (!completion.accept(snapshot) || activity.isFinishing || activity.isDestroyed) return
        activity.setResult(Activity.RESULT_OK)
        Toast.makeText(activity, "ChatGPT 已接入", Toast.LENGTH_SHORT).show()
        activity.finish()
    }

    fun wrap(page: View): View {
        if (!enabled) return page
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(activity.getColor(R.color.elon_bg_app))
            addView(LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                addView(ImageButton(activity).apply {
                    setImageResource(android.R.drawable.ic_menu_close_clear_cancel)
                    setColorFilter(activity.getColor(R.color.elon_text_primary))
                    setBackgroundColor(android.graphics.Color.TRANSPARENT)
                    contentDescription = "取消接入，返回群聊"
                    setOnClickListener { activity.finish() }
                }, LinearLayout.LayoutParams(dp(48), dp(48)))
                addView(TextView(activity).apply {
                    text = "接入我的 ChatGPT"
                    textSize = 18f
                    setTextColor(activity.getColor(R.color.elon_text_primary))
                    setPadding(0, dp(8), dp(16), dp(8))
                }, LinearLayout.LayoutParams(0, -2, 1f))
            }, LinearLayout.LayoutParams(-1, -2))
            addView(page, LinearLayout.LayoutParams(-1, 0, 1f))
        }
    }

    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()

    companion object {
        private const val EXTRA_RETURN_AFTER_LOGIN = "chatgpt_return_after_login"
        fun intent(activity: android.content.Context): Intent =
            ChatGptWebOfficialFallbackIntent.createLogin(activity).putExtra(EXTRA_RETURN_AFTER_LOGIN, true)
    }
}

internal class ChatGptWebLoginCompletion(private val enabled: Boolean) {
    private var consumed = false
    fun accept(snapshot: ChatGptWebSnapshot): Boolean {
        if (!enabled || consumed || ChatGptWebAccountConnection.observedState(snapshot) !=
            ChatGptWebAccountConnection.State.CONNECTED) return false
        consumed = true
        return true
    }
}
