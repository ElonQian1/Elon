package com.elon.app

import android.content.Context
import android.util.Base64

private const val ELON_SELF_PROJECT_ICON_MIME = "image/png"

internal fun defaultElonSelfProjectIconDataUrl(context: Context): String? {
    return runCatching {
        context.resources.openRawResource(R.mipmap.ic_launcher).use { stream ->
            val encoded = Base64.encodeToString(stream.readBytes(), Base64.NO_WRAP)
            "data:$ELON_SELF_PROJECT_ICON_MIME;base64,$encoded"
        }
    }.getOrNull()
}
