// infrastructure/floating/FloatingVoiceActivity.kt
// module: infrastructure/floating | layer: infrastructure | role: voice-input-activity
// summary: 语音输入透明Activity - 使用AI意图分析判断是否说完

package com.elon.app.agent.infrastructure.floating

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.WindowManager
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.agent.AgentConfigActivity
import com.elon.app.agent.application.IntentAnalyzer
import com.elon.app.agent.infrastructure.voice.VoiceRecognitionHelper
import kotlinx.coroutines.*

/**
 * 🎤 语音输入透明Activity
 * 
 * 使用 AI 意图分析判断用户是否说完：
 * - 语音停顿后调用意图分析
 * - 如果 isComplete=false，提示"继续说..."
 * - 如果 isComplete=true，自动执行
 */
class FloatingVoiceActivity : AppCompatActivity() {
    
    private lateinit var voiceHelper: VoiceRecognitionHelper
    private lateinit var statusText: TextView
    private lateinit var resultText: TextView
    private lateinit var voiceIndicator: TextView
    private lateinit var cancelButton: Button
    
    private val handler = Handler(Looper.getMainLooper())
    private val scope = CoroutineScope(Dispatchers.Main + Job())
    
    // 累积的识别文本
    private var accumulatedText: StringBuilder = StringBuilder()
    
    // 最大续录次数
    private val maxContinueCount = 3
    private var continueCount = 0
    
    // 轻量级意图分析器
    private var intentAnalyzer: IntentAnalyzer? = null
    
    private val requestPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        if (isGranted) {
            startListening()
        } else {
            Toast.makeText(this, "需要麦克风权限", Toast.LENGTH_SHORT).show()
            finish()
        }
    }
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // 设置透明窗口
        window.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setDimAmount(0.6f)
        }
        
        // 初始化语音助手
        voiceHelper = VoiceRecognitionHelper(this)
        
        // 初始化意图分析器
        val apiKey = AgentConfigActivity.getApiKey(this)
        if (apiKey.isNotEmpty()) {
            intentAnalyzer = IntentAnalyzer(apiKey)
        }
        
        // 创建UI
        createUI()
        
        // 检查权限并开始
        checkPermissionAndStart()
    }
    
    private fun createUI() {
        val density = resources.displayMetrics.density
        
        val rootLayout = FrameLayout(this).apply {
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { cancelAndClose() }
        }
        
        // 中央卡片
        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(
                (32 * density).toInt(),
                (32 * density).toInt(),
                (32 * density).toInt(),
                (24 * density).toInt()
            )
            
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#181B20"))
                cornerRadius = 24 * density
            }
            background = bg
            elevation = 16 * density
            setOnClickListener { }
        }
        
        // 语音指示器
        voiceIndicator = TextView(this).apply {
            text = "🎤"
            textSize = 56f
            gravity = Gravity.CENTER
        }
        card.addView(voiceIndicator, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { gravity = Gravity.CENTER })
        
        // 状态文字
        statusText = TextView(this).apply {
            text = "正在准备..."
            textSize = 16f
            setTextColor(Color.parseColor("#F2F5FA"))
            gravity = Gravity.CENTER
        }
        card.addView(statusText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (12 * density).toInt()
        })
        
        // 识别结果
        resultText = TextView(this).apply {
            text = ""
            textSize = 18f
            setTextColor(Color.parseColor("#6091CF"))
            gravity = Gravity.CENTER
            maxLines = 5
            minHeight = (60 * density).toInt()
        }
        card.addView(resultText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (16 * density).toInt()
        })
        
        // 取消按钮
        cancelButton = Button(this).apply {
            text = "❌ 取消"
            textSize = 14f
            setTextColor(Color.parseColor("#F2F5FA"))
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#283140"))
                cornerRadius = 20 * density
            }
            background = bg
            setPadding((24 * density).toInt(), (10 * density).toInt(), (24 * density).toInt(), (10 * density).toInt())
            setOnClickListener { cancelAndClose() }
        }
        card.addView(cancelButton, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (20 * density).toInt()
        })
        
        // 提示
        val tipText = TextView(this).apply {
            text = "说完会自动执行，点击空白处取消"
            textSize = 11f
            setTextColor(Color.parseColor("#6F7785"))
            gravity = Gravity.CENTER
        }
        card.addView(tipText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
            topMargin = (12 * density).toInt()
        })
        
        rootLayout.addView(card, FrameLayout.LayoutParams(
            (320 * density).toInt(),
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply { gravity = Gravity.CENTER })
        
        setContentView(rootLayout)
    }
    
    private fun cancelAndClose() {
        voiceHelper.stopListening()
        handler.removeCallbacksAndMessages(null)
        scope.cancel()
        finish()
    }
    
    private fun checkPermissionAndStart() {
        when {
            ContextCompat.checkSelfPermission(
                this, Manifest.permission.RECORD_AUDIO
            ) == PackageManager.PERMISSION_GRANTED -> {
                startListening()
            }
            else -> {
                requestPermissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
            }
        }
    }
    
    private fun startListening() {
        statusText.text = "请说话..."
        voiceIndicator.text = "🔴"
        
        voiceHelper.apply {
            onListeningStateChanged = { isListening ->
                runOnUiThread {
                    if (isListening) {
                        statusText.text = "正在聆听..."
                        voiceIndicator.text = "🔴"
                    }
                }
            }
            
            onPartialResult = { partial ->
                runOnUiThread {
                    val display = if (accumulatedText.isNotEmpty()) {
                        accumulatedText.toString() + partial
                    } else {
                        partial
                    }
                    resultText.text = display
                }
            }
            
            onResult = { result ->
                runOnUiThread {
                    handleRecognitionResult(result)
                }
            }
            
            onError = { error ->
                runOnUiThread {
                    if (accumulatedText.isNotEmpty()) {
                        // 有内容就直接提交
                        submitTask(accumulatedText.toString())
                    } else {
                        statusText.text = "识别出错: $error"
                        voiceIndicator.text = "❌"
                        handler.postDelayed({ finish() }, 1500)
                    }
                }
            }
        }
        
        voiceHelper.startListening()
    }
    
    /**
     * 处理识别结果，调用意图分析判断是否说完
     */
    private fun handleRecognitionResult(result: String) {
        if (result.isBlank()) {
            if (accumulatedText.isNotEmpty()) {
                // 空结果但有累积内容，直接提交
                submitTask(accumulatedText.toString())
            } else {
                statusText.text = "没听清，请再说一遍..."
                voiceIndicator.text = "🎤"
                handler.postDelayed({ startListening() }, 500)
            }
            return
        }
        
        // 累加结果
        accumulatedText.append(result)
        val fullText = accumulatedText.toString()
        resultText.text = fullText
        
        // 检查是否有分析器
        val analyzer = intentAnalyzer
        if (analyzer == null) {
            // 没有分析器，直接执行
            submitTask(fullText)
            return
        }
        
        // 调用意图分析判断是否完整
        statusText.text = "分析中..."
        voiceIndicator.text = "🧠"
        
        scope.launch {
            try {
                val analysisResult = analyzer.analyze(fullText)
                
                if (analysisResult.isComplete) {
                    // 表述完整，自动执行
                    statusText.text = "正在执行..."
                    voiceIndicator.text = "🚀"
                    handler.postDelayed({
                        submitTask(analysisResult.goal)
                    }, 200)
                } else {
                    // 表述不完整，继续录音
                    continueCount++
                    if (continueCount < maxContinueCount) {
                        statusText.text = "继续说..."
                        voiceIndicator.text = "🟡"
                        handler.postDelayed({ startListening() }, 600)
                    } else {
                        // 续录次数用完，直接提交
                        submitTask(fullText)
                    }
                }
            } catch (e: Exception) {
                // 分析失败，直接提交
                submitTask(fullText)
            }
        }
    }
    
    private fun submitTask(goal: String) {
        handler.removeCallbacksAndMessages(null)
        voiceHelper.stopListening()
        
        if (!com.elon.app.agent.AgentService.isRunning()) {
            Toast.makeText(this, "❌ 请先开启无障碍服务", Toast.LENGTH_LONG).show()
            finish()
            return
        }
        
        com.elon.app.agent.AgentService.executeTask(goal)
        Toast.makeText(this, "🚀 $goal", Toast.LENGTH_SHORT).show()
        finish()
    }
    
    override fun onDestroy() {
        super.onDestroy()
        handler.removeCallbacksAndMessages(null)
        scope.cancel()
        voiceHelper.destroy()
    }
}
