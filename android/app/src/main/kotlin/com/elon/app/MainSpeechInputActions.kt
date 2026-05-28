package com.elon.app

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.util.Locale

internal class MainSpeechInputActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val speechPermissionRequest: Int,
    private val userId: () -> String,
    private val selectedAgent: () -> String?,
    private val activeConversation: () -> AppConversation,
    private val voiceHoldButton: () -> TextView,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit
) {
    private var speechRecognizer: SpeechRecognizer? = null
    private var isListeningForSpeech = false
    private var isSpeechCanceled = false
    private var speechSessionId = 0
    private var translationGeneration = 0

    fun startSpeechToText() {
        if (activeConversation().ended) return
        if (ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.RECORD_AUDIO), speechPermissionRequest)
            return
        }
        if (!SpeechRecognizer.isRecognitionAvailable(activity)) {
            Toast.makeText(activity, "当前设备不可用语音识别", Toast.LENGTH_SHORT).show()
            return
        }
        resetSpeechRecognizer()
        translationGeneration += 1
        val sessionId = ++speechSessionId
        isSpeechCanceled = false
        isListeningForSpeech = true
        voiceHoldButton().text = "松开 转文字"
        speechRecognizer = SpeechRecognizer.createSpeechRecognizer(activity).apply {
            setRecognitionListener(createSpeechRecognitionListener(sessionId))
        }
        runCatching {
            speechRecognizer?.startListening(recognizerIntent())
        }.onFailure { error ->
            if (sessionId != speechSessionId) return
            DebugTraceStore.record("speech_start_failed", mapOf("error" to error.message))
            isListeningForSpeech = false
            voiceHoldButton().text = "按住 说话"
            resetSpeechRecognizer()
            Toast.makeText(activity, "语音识别启动失败，请重试", Toast.LENGTH_SHORT).show()
        }
    }

    fun stopSpeechToText() {
        if (!isListeningForSpeech) return
        isListeningForSpeech = false
        voiceHoldButton().text = "识别中..."
        runCatching {
            speechRecognizer?.stopListening()
        }.onFailure { error ->
            DebugTraceStore.record("speech_stop_failed", mapOf("error" to error.message))
            voiceHoldButton().text = "按住 说话"
            resetSpeechRecognizer()
            Toast.makeText(activity, "语音识别失败，请重试", Toast.LENGTH_SHORT).show()
        }
    }

    fun cancelSpeechToText() {
        if (!isListeningForSpeech && speechRecognizer == null) return
        speechSessionId += 1
        translationGeneration += 1
        isSpeechCanceled = true
        isListeningForSpeech = false
        voiceHoldButton().text = "按住 说话"
        runCatching { speechRecognizer?.cancel() }
        resetSpeechRecognizer()
    }

    fun destroy() {
        speechSessionId += 1
        translationGeneration += 1
        resetSpeechRecognizer()
        isListeningForSpeech = false
    }

    private fun createSpeechRecognitionListener(sessionId: Int): RecognitionListener {
        return object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                if (!isCurrentSpeechSession(sessionId)) return
                voiceHoldButton().text = "正在听..."
            }
            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() {
                if (!isCurrentSpeechSession(sessionId)) return
                voiceHoldButton().text = "识别中..."
            }
            override fun onError(error: Int) {
                if (!isCurrentSpeechSession(sessionId)) return
                DebugTraceStore.record("speech_error", mapOf("code" to error))
                isListeningForSpeech = false
                voiceHoldButton().text = "按住 说话"
                resetSpeechRecognizer()
                if (!isSpeechCanceled && shouldShowSpeechError(error)) {
                    Toast.makeText(activity, "语音识别失败，请重试", Toast.LENGTH_SHORT).show()
                }
            }
            override fun onResults(results: Bundle?) {
                if (!isCurrentSpeechSession(sessionId)) return
                isListeningForSpeech = false
                voiceHoldButton().text = "按住 说话"
                resetSpeechRecognizer()
                if (isSpeechCanceled) return
                val spoken = results
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    ?.firstOrNull()
                    .orEmpty()
                    .trim()
                if (spoken.isNotBlank()) {
                    handleRecognizedSpeech(spoken)
                }
            }
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }
    }

    private fun recognizerIntent(): Intent {
        return Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.SIMPLIFIED_CHINESE.toLanguageTag())
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_PREFERENCE, Locale.SIMPLIFIED_CHINESE.toLanguageTag())
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
            putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 3)
        }
    }

    private fun handleRecognizedSpeech(spoken: String) {
        setVoiceMode(false)
        applyVoiceMode()
        setInputText(spoken)
        translateSpeechText(spoken)
    }

    private fun translateSpeechText(source: String) {
        val generation = ++translationGeneration
        Thread {
            val result = runCatching { requestSimplifiedChinese(source) }
            activity.runOnUiThread {
                if (generation != translationGeneration) return@runOnUiThread
                result.onSuccess { translated ->
                    val clean = translated.trim()
                    if (clean.isNotBlank() && binding.inputEdit.text.toString() == source) {
                        setInputText(clean)
                    }
                }.onFailure { error ->
                    DebugTraceStore.record("speech_translate_failed", mapOf("error" to error.message))
                    Toast.makeText(activity, "翻译暂不可用，已保留识别文字", Toast.LENGTH_SHORT).show()
                }
            }
        }.start()
    }

    private fun requestSimplifiedChinese(source: String): String {
        val payload = JSONObject().apply {
            put("text", source)
            selectedAgent()?.takeIf { it.isNotBlank() }?.let { put("agent_name", it) }
        }
        val body = payload.toString().toRequestBody("application/json; charset=utf-8".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/user/${urlPart(userId())}/speech/translate")
                .post(body)
        ).build()
        http.newCall(request).execute().use { response ->
            val responseBody = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })
            return JSONObject(responseBody).optString("text", source).ifBlank { source }
        }
    }

    private fun setInputText(text: String) {
        binding.inputEdit.setText(text)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }

    private fun isCurrentSpeechSession(sessionId: Int): Boolean {
        return sessionId == speechSessionId
    }

    private fun shouldShowSpeechError(error: Int): Boolean {
        return error != SpeechRecognizer.ERROR_NO_MATCH &&
            error != SpeechRecognizer.ERROR_SPEECH_TIMEOUT &&
            error != SpeechRecognizer.ERROR_CLIENT
    }

    private fun resetSpeechRecognizer() {
        speechRecognizer?.let { recognizer ->
            runCatching { recognizer.destroy() }
        }
        speechRecognizer = null
    }
}
