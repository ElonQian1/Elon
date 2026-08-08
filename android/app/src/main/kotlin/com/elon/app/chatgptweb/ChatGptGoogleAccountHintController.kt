package com.elon.app.chatgptweb

import android.accounts.AccountManager
import android.app.Activity
import android.os.Handler
import android.os.Looper
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.google.android.material.button.MaterialButton

internal class ChatGptGoogleAccountHintController(
    activity: AppCompatActivity,
    private val accountButton: MaterialButton,
    private val onBeginAuthentication: () -> Unit,
    private val onRequestGoogleProvider: () -> Unit,
    private val onStatusMessage: (Int) -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private var selectedAccountName: String? = null
    private var providerClickPending = false
    private var providerClickAttempts = 0
    private val requestProviderClick = Runnable {
        if (!providerClickPending) return@Runnable
        providerClickAttempts += 1
        onRequestGoogleProvider()
    }
    private val accountChooser = activity.registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        if (result.resultCode != Activity.RESULT_OK) {
            onStatusMessage(R.string.chatgpt_google_account_cancelled)
            return@registerForActivityResult
        }
        val accountType = result.data?.getStringExtra(AccountManager.KEY_ACCOUNT_TYPE)
        val accountName = ChatGptGoogleLoginHintPolicy.normalizeAccountName(
            result.data?.getStringExtra(AccountManager.KEY_ACCOUNT_NAME),
        )
        if (accountType != GOOGLE_ACCOUNT_TYPE || accountName == null) {
            onStatusMessage(R.string.chatgpt_google_account_invalid)
            return@registerForActivityResult
        }

        selectedAccountName = accountName
        accountButton.text = activity.getString(
            R.string.chatgpt_google_account_selected,
            maskAccountName(accountName),
        )
        providerClickPending = true
        providerClickAttempts = 0
        onBeginAuthentication()
    }

    init {
        accountButton.setOnClickListener {
            val chooserIntent = AccountManager.newChooseAccountIntent(
                null,
                null,
                arrayOf(GOOGLE_ACCOUNT_TYPE),
                activity.getString(R.string.chatgpt_google_account_picker_title),
                null,
                null,
                null,
            )
            runCatching { accountChooser.launch(chooserIntent) }
                .onFailure { onStatusMessage(R.string.chatgpt_google_account_unavailable) }
        }
    }

    fun rewriteGoogleAuthorization(rawUrl: String): String? =
        ChatGptGoogleLoginHintPolicy.rewriteAuthorizationUrl(rawUrl, selectedAccountName)

    fun onPageReady(url: String) {
        if (!providerClickPending || !ChatGptWebNavigationPolicy.isAuthenticationPage(url)) return
        scheduleProviderClick(FIRST_ATTEMPT_DELAY_MS)
    }

    fun onCommandResult(event: ChatGptWebEvent.CommandResult): Boolean {
        if (event.action != GOOGLE_LOGIN_ACTION) return false
        handler.removeCallbacks(requestProviderClick)
        if (event.ok) {
            providerClickPending = false
            providerClickAttempts = 0
            return true
        }
        if (providerClickPending && providerClickAttempts < MAX_PROVIDER_CLICK_ATTEMPTS) {
            scheduleProviderClick(RETRY_DELAY_MS)
        } else {
            providerClickPending = false
            onStatusMessage(R.string.chatgpt_google_provider_unavailable)
        }
        return true
    }

    fun onAuthenticated() {
        providerClickPending = false
        handler.removeCallbacks(requestProviderClick)
    }

    fun reset() {
        selectedAccountName = null
        providerClickPending = false
        providerClickAttempts = 0
        handler.removeCallbacks(requestProviderClick)
        accountButton.setText(R.string.chatgpt_google_account_login)
    }

    fun dispose() = handler.removeCallbacks(requestProviderClick)

    private fun scheduleProviderClick(delayMillis: Long) {
        handler.removeCallbacks(requestProviderClick)
        handler.postDelayed(requestProviderClick, delayMillis)
    }

    private fun maskAccountName(accountName: String): String {
        val localPart = accountName.substringBefore('@')
        val domain = accountName.substringAfter('@')
        val visiblePrefix = localPart.take(if (localPart.length > 1) 2 else 1)
        return "$visiblePrefix***@$domain"
    }

    private companion object {
        const val GOOGLE_ACCOUNT_TYPE = "com.google"
        const val GOOGLE_LOGIN_ACTION = "start_google_login"
        const val FIRST_ATTEMPT_DELAY_MS = 350L
        const val RETRY_DELAY_MS = 700L
        const val MAX_PROVIDER_CLICK_ATTEMPTS = 4
    }
}
