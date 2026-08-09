package com.elon.app.chatgptweb

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.MimeTypeMap
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity

internal class ChatGptWebFileChooserController(
    activity: AppCompatActivity,
) {
    private var pendingCallback: ValueCallback<Array<Uri>>? = null
    private val launcher = activity.registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        val callback = pendingCallback ?: return@registerForActivityResult
        pendingCallback = null
        callback.onReceiveValue(
            WebChromeClient.FileChooserParams.parseResult(result.resultCode, result.data),
        )
    }

    fun show(
        webView: WebView,
        callback: ValueCallback<Array<Uri>>,
        params: WebChromeClient.FileChooserParams,
    ): Boolean {
        pendingCallback?.onReceiveValue(null)
        pendingCallback = null
        if (!ChatGptWebNavigationPolicy.supportsEnhancedMode(webView.url)) {
            callback.onReceiveValue(null)
            return true
        }
        pendingCallback = callback
        return runCatching {
            launcher.launch(createPickerIntent(params))
            true
        }.getOrElse {
            pendingCallback = null
            callback.onReceiveValue(null)
            true
        }
    }

    fun dispose() {
        pendingCallback?.onReceiveValue(null)
        pendingCallback = null
    }

    private fun createPickerIntent(params: WebChromeClient.FileChooserParams): Intent {
        val mimeTypes = sanitizeMimeTypes(params.acceptTypes)
        return Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            type = mimeTypes.singleOrNull() ?: "*/*"
            if (mimeTypes.size > 1) putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
            putExtra(
                Intent.EXTRA_ALLOW_MULTIPLE,
                params.mode == WebChromeClient.FileChooserParams.MODE_OPEN_MULTIPLE,
            )
        }
    }

    internal companion object {
        fun sanitizeMimeTypes(rawTypes: Array<String>): List<String> = rawTypes
            .asSequence()
            .flatMap { it.split(',').asSequence() }
            .map(String::trim)
            .map { value ->
                when {
                    value.startsWith('.') -> MimeTypeMap.getSingleton()
                        .getMimeTypeFromExtension(value.removePrefix(".").lowercase())
                    else -> value.substringBefore(';').lowercase()
                }
            }
            .filterNotNull()
            .filter { it == "*/*" || MIME_TYPE.matches(it) }
            .distinct()
            .take(MAX_MIME_TYPES)
            .toList()
            .ifEmpty { listOf("*/*") }

        private const val MAX_MIME_TYPES = 12
        private val MIME_TYPE = Regex("[a-z0-9!#$&^_.+-]+/[a-z0-9!#$&^_.+*-]+")
    }
}
