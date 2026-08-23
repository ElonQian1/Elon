package com.elon.app.chatgptweb

import android.annotation.SuppressLint
import android.net.Uri
import android.webkit.PermissionRequest
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebSettings
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.configureWebChatBackgroundSurface

@SuppressLint("SetJavaScriptEnabled")
internal fun createChatGptBackgroundWebView(
    activity: AppCompatActivity,
    audioPermissionController: ChatGptWebAudioPermissionController,
    onPageProgress: (Int) -> Unit,
    onFileChooser: (ValueCallback<Array<Uri>>) -> Unit,
): WebView = WebView(activity).apply {
    configureWebChatBackgroundSurface()
    settings.apply {
        javaScriptEnabled = true
        domStorageEnabled = true
        allowFileAccess = false
        allowContentAccess = true
        javaScriptCanOpenWindowsAutomatically = false
        setSupportMultipleWindows(false)
        mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
        safeBrowsingEnabled = true
        mediaPlaybackRequiresUserGesture = true
        builtInZoomControls = false
        displayZoomControls = false
    }
    webChromeClient = object : WebChromeClient() {
        override fun onProgressChanged(view: WebView, newProgress: Int) = onPageProgress(newProgress)

        override fun onShowFileChooser(
            webView: WebView,
            filePathCallback: ValueCallback<Array<Uri>>,
            fileChooserParams: FileChooserParams,
        ): Boolean {
            onFileChooser(filePathCallback)
            return true
        }

        override fun onPermissionRequest(request: PermissionRequest) {
            activity.runOnUiThread { audioPermissionController.handle(request) }
        }

        override fun onPermissionRequestCanceled(request: PermissionRequest) {
            activity.runOnUiThread { audioPermissionController.cancel(request) }
        }
    }
}
