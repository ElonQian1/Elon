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
import java.util.Locale

internal class MainSpeechInputActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val speechPermissionRequest: Int,
    private val activeConversation: () -> AppConversation,
    private val voiceHoldButton: () -> TextView,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit
) {
    private var speechRecognizer: SpeechRecognizer? = null
    private var isListeningForSpeech = false

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
        if (speechRecognizer == null) {
            speechRecognizer = SpeechRecognizer.createSpeechRecognizer(activity).apply {
                setRecognitionListener(createSpeechRecognitionListener())
            }
        }
        isListeningForSpeech = true
        voiceHoldButton().text = "松开 转文字"
        speechRecognizer?.startListening(Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.CHINA.toLanguageTag())
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
        })
    }

    fun stopSpeechToText() {
        if (!isListeningForSpeech) return
        isListeningForSpeech = false
        voiceHoldButton().text = "识别中..."
        speechRecognizer?.stopListening()
    }

    fun destroy() {
        speechRecognizer?.destroy()
        speechRecognizer = null
        isListeningForSpeech = false
    }

    private fun createSpeechRecognitionListener(): RecognitionListener {
        return object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                voiceHoldButton().text = "正在听..."
            }
            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() {
                voiceHoldButton().text = "识别中..."
            }
            override fun onError(error: Int) {
                isListeningForSpeech = false
                voiceHoldButton().text = "按住 说话"
                if (error != SpeechRecognizer.ERROR_NO_MATCH && error != SpeechRecognizer.ERROR_SPEECH_TIMEOUT) {
                    Toast.makeText(activity, "语音识别失败，请重试", Toast.LENGTH_SHORT).show()
                }
            }
            override fun onResults(results: Bundle?) {
                isListeningForSpeech = false
                voiceHoldButton().text = "按住 说话"
                val spoken = results
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    ?.firstOrNull()
                    .orEmpty()
                    .trim()
                if (spoken.isNotBlank()) {
                    setVoiceMode(false)
                    applyVoiceMode()
                    binding.inputEdit.setText(spoken)
                    binding.inputEdit.setSelection(binding.inputEdit.text.length)
                }
            }
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }
    }
}
