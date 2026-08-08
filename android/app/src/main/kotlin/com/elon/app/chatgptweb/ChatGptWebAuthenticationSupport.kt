package com.elon.app.chatgptweb

import android.webkit.WebSettings
import androidx.webkit.WebSettingsCompat
import androidx.webkit.WebViewFeature

internal object ChatGptWebAuthenticationSupport {
    enum class Status {
        ENABLED,
        UNSUPPORTED,
    }

    fun configure(settings: WebSettings): Status {
        if (!WebViewFeature.isFeatureSupported(WebViewFeature.WEB_AUTHENTICATION)) {
            return Status.UNSUPPORTED
        }
        return runCatching {
            WebSettingsCompat.setWebAuthenticationSupport(
                settings,
                WebSettingsCompat.WEB_AUTHENTICATION_SUPPORT_FOR_BROWSER,
            )
            if (
                WebSettingsCompat.getWebAuthenticationSupport(settings) ==
                WebSettingsCompat.WEB_AUTHENTICATION_SUPPORT_FOR_BROWSER
            ) {
                Status.ENABLED
            } else {
                Status.UNSUPPORTED
            }
        }.getOrDefault(Status.UNSUPPORTED)
    }
}
