package com.elon.app.chatgptweb

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.webkit.WebChromeClient

internal object ChatGptWebFileSelectionResult {
    fun parse(resultCode: Int, data: Intent?): Array<Uri>? {
        if (resultCode != Activity.RESULT_OK) return null
        WebChromeClient.FileChooserParams.parseResult(resultCode, data)
            ?.filterDistinct()
            ?.takeIf(Array<Uri>::isNotEmpty)
            ?.let { return it }

        val values = buildList {
            data?.data?.let(::add)
            data?.clipData?.let { clip ->
                for (index in 0 until minOf(clip.itemCount, MAX_SELECTED_URIS)) {
                    clip.getItemAt(index).uri?.let(::add)
                }
            }
            data?.streamUris()?.let(::addAll)
        }
        return values.toTypedArray().filterDistinct().takeIf(Array<Uri>::isNotEmpty)
    }

    @Suppress("DEPRECATION")
    private fun Intent.streamUris(): List<Uri> = when (val stream = extras?.get(Intent.EXTRA_STREAM)) {
        is Uri -> listOf(stream)
        is ArrayList<*> -> stream.filterIsInstance<Uri>().take(MAX_SELECTED_URIS)
        else -> emptyList()
    }

    private fun Array<Uri>.filterDistinct(): Array<Uri> {
        val allowed = ChatGptWebFileSelectionPolicy.filter(map(Uri::toString))
        return allowed.map(Uri::parse).toTypedArray()
    }

    private const val MAX_SELECTED_URIS = 10
}

internal object ChatGptWebFileSelectionPolicy {
    fun filter(values: List<String>): List<String> = values.asSequence()
        .filter { it.startsWith(CONTENT_PREFIX, ignoreCase = true) }
        .distinct()
        .take(MAX_SELECTED_URIS)
        .toList()

    private const val CONTENT_PREFIX = "content://"
    private const val MAX_SELECTED_URIS = 10
}
