package com.elon.app.update

import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

internal class AppUpdateRepository(
    private val http: OkHttpClient = defaultClient(),
) {
    fun fetchLatest(): AppUpdateVersion? = runCatching {
        http.newCall(
            Request.Builder()
                .url(VERSION_URL)
                .addHeader("Cache-Control", "no-cache, no-store")
                .build()
        ).execute().use { response ->
            if (!response.isSuccessful) return null
            AppUpdateVersion.parse(response.body?.string().orEmpty())
        }
    }.getOrNull()

    companion object {
        const val VERSION_URL = "http://43.139.149.158:8080/app/version.json"

        fun defaultClient(readTimeoutSeconds: Long = 30L): OkHttpClient =
            OkHttpClient.Builder()
                .connectTimeout(10L, TimeUnit.SECONDS)
                .readTimeout(readTimeoutSeconds, TimeUnit.SECONDS)
                .build()
    }
}
