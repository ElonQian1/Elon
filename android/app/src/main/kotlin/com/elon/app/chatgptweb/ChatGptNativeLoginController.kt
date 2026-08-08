package com.elon.app.chatgptweb

import android.content.Context
import android.os.SystemClock
import android.view.View
import android.widget.TextView
import com.elon.app.R
import com.google.android.material.button.MaterialButton
import java.util.Locale

internal class ChatGptNativeLoginController(
    private val context: Context,
    private val stageView: TextView,
    private val elapsedView: TextView,
    private val primaryButton: MaterialButton,
    officialButton: MaterialButton,
    private val onOpenAuthentication: () -> Unit,
    private val onOpenOfficialPage: () -> Unit,
    private val onOpenNativeConversation: () -> Unit,
    elapsedRealtime: () -> Long = SystemClock::elapsedRealtime,
) {
    private val tracker = ChatGptLoginFlowTracker(elapsedRealtime)
    private var authenticated = false
    private var autoOpenNativeAfterLogin = false
    private val elapsedTicker = object : Runnable {
        override fun run() {
            val snapshot = tracker.snapshot()
            render(snapshot)
            if (snapshot.isRunning) elapsedView.postDelayed(this, TICK_INTERVAL_MS)
        }
    }

    init {
        primaryButton.setOnClickListener {
            if (authenticated) {
                onOpenNativeConversation()
            } else {
                beginAuthentication()
            }
        }
        officialButton.setOnClickListener { onOpenOfficialPage() }
        render(tracker.snapshot())
    }

    fun beginAuthentication() {
        autoOpenNativeAfterLogin = true
        render(tracker.begin())
        startTicker()
        onOpenAuthentication()
    }

    fun onPageStarted(url: String) {
        render(tracker.onPageStarted(url))
    }

    fun onPageReady(url: String) {
        render(tracker.onPageReady(url))
    }

    fun onAuthenticated(): Boolean {
        if (authenticated) return false
        authenticated = true
        render(tracker.markAuthenticated())
        stopTicker()
        return autoOpenNativeAfterLogin.also { autoOpenNativeAfterLogin = false }
    }

    fun onPageError() {
        render(tracker.fail())
        stopTicker()
    }

    fun reset() {
        authenticated = false
        autoOpenNativeAfterLogin = false
        stopTicker()
        render(tracker.reset())
    }

    fun dispose() = stopTicker()

    private fun render(snapshot: ChatGptLoginFlowSnapshot) {
        stageView.setText(
            when (snapshot.stage) {
                ChatGptLoginStage.READY -> R.string.chatgpt_quick_stage_ready
                ChatGptLoginStage.OPENING_OFFICIAL_AUTH -> R.string.chatgpt_quick_stage_opening
                ChatGptLoginStage.WAITING_FOR_USER -> R.string.chatgpt_quick_stage_waiting
                ChatGptLoginStage.COMPLETING -> R.string.chatgpt_quick_stage_completing
                ChatGptLoginStage.AUTHENTICATED -> R.string.chatgpt_quick_stage_authenticated
                ChatGptLoginStage.FAILED -> R.string.chatgpt_quick_stage_failed
            },
        )
        elapsedView.visibility = if (snapshot.elapsedMillis > 0L) View.VISIBLE else View.INVISIBLE
        elapsedView.text = context.getString(
            R.string.chatgpt_quick_elapsed,
            String.format(Locale.ROOT, "%.1f", snapshot.elapsedMillis / 1_000.0),
        )
        primaryButton.setText(
            if (authenticated) R.string.chatgpt_quick_enter_native else R.string.chatgpt_quick_login,
        )
    }

    private fun startTicker() {
        stopTicker()
        elapsedView.postDelayed(elapsedTicker, TICK_INTERVAL_MS)
    }

    private fun stopTicker() = elapsedView.removeCallbacks(elapsedTicker)

    private companion object {
        const val TICK_INTERVAL_MS = 250L
    }
}
