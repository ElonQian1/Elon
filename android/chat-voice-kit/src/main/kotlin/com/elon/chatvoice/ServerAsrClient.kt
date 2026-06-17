package com.elon.chatvoice

import android.os.Handler
import android.os.Looper
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.Response
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

class ServerAsrClient(
    private val config: ChatVoiceConfig,
    private val client: OkHttpClient = defaultClient(),
) {
    private val main = Handler(Looper.getMainLooper())

    fun transcribe(
        audioFile: File,
        options: ServerAsrOptions = ServerAsrOptions(),
        callback: (Result<ServerAsrResult>) -> Unit,
    ): Call? {
        if (!audioFile.isFile || audioFile.length() <= 0L) {
            callback(Result.failure(IllegalArgumentException("audio file is empty")))
            return null
        }
        val token = config.bearerTokenProvider()?.trim().orEmpty()
        val request = Request.Builder()
            .url("${config.normalizedBaseUrl}/api/voice/asr")
            .apply {
                if (token.isNotEmpty()) header("Authorization", "Bearer $token")
            }
            .post(multipartBody(audioFile, options))
            .build()
        return client.newCall(request).also { call ->
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) {
                    deliver(callback, Result.failure(e))
                }

                override fun onResponse(call: Call, response: Response) {
                    response.use {
                        val raw = it.body?.string().orEmpty()
                        if (!it.isSuccessful) {
                            deliver(callback, Result.failure(IOException("ASR ${it.code}: $raw")))
                            return
                        }
                        val text = runCatching { JSONObject(raw).optString("text") }.getOrDefault("")
                        if (text.isBlank()) {
                            deliver(callback, Result.failure(IOException("ASR returned empty text")))
                        } else {
                            deliver(callback, Result.success(ServerAsrResult(text.trim(), raw)))
                        }
                    }
                }
            })
        }
    }

    private fun multipartBody(audioFile: File, options: ServerAsrOptions): MultipartBody =
        MultipartBody.Builder()
            .setType(MultipartBody.FORM)
            .addFormDataPart(
                "audio",
                audioFile.name,
                audioFile.asRequestBody("audio/mp4".toMediaType()),
            )
            .addTextPart("format", "m4a")
            .apply {
                options.language?.let { addTextPart("language", it) }
                options.beamSize?.let { addTextPart("beam_size", it.toString()) }
                options.vadFilter?.let { addTextPart("vad_filter", it.toString()) }
                options.conditionOnPreviousText?.let {
                    addTextPart("condition_on_previous_text", it.toString())
                }
            }
            .build()

    private fun MultipartBody.Builder.addTextPart(name: String, value: String): MultipartBody.Builder =
        addFormDataPart(name, value)

    private fun deliver(
        callback: (Result<ServerAsrResult>) -> Unit,
        result: Result<ServerAsrResult>,
    ) {
        main.post { callback(result) }
    }

    companion object {
        fun defaultClient(): OkHttpClient =
            OkHttpClient.Builder()
                .connectTimeout(10, TimeUnit.SECONDS)
                .readTimeout(60, TimeUnit.SECONDS)
                .writeTimeout(60, TimeUnit.SECONDS)
                .build()
    }
}
