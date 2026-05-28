// interface/AgentActivity.kt
package com.elon.app.agent

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import com.google.android.material.snackbar.Snackbar

/**
 * Agent 配置界面
 */
class AgentActivity : Activity() {
    
    private lateinit var statusText: TextView
    private lateinit var goalInput: EditText
    private lateinit var startButton: Button
    private lateinit var stopButton: Button
    private lateinit var settingsButton: Button
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // 简单布局（实际应该用 XML）
        setContentView(createSimpleLayout())
        
        statusText = findViewById(R.id.status_text)
        goalInput = findViewById(R.id.goal_input)
        startButton = findViewById(R.id.start_button)
        stopButton = findViewById(R.id.stop_button)
        settingsButton = findViewById(R.id.settings_button)
        
        setupListeners()
        checkAccessibilityService()
    }
    
    private fun setupListeners() {
        startButton.setOnClickListener {
            val goal = goalInput.text.toString()
            if (goal.isBlank()) {
                Snackbar.make(it, "请输入目标", Snackbar.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            
            // TODO: 调用 AgentService.executeGoal(goal)
            Snackbar.make(it, "开始执行: $goal", Snackbar.LENGTH_SHORT).show()
        }
        
        stopButton.setOnClickListener {
            // TODO: 调用 AgentService.stop()
            Snackbar.make(it, "Agent 已停止", Snackbar.LENGTH_SHORT).show()
        }
        
        settingsButton.setOnClickListener {
            openAccessibilitySettings()
        }
    }
    
    /**
     * 检查无障碍服务是否已启用
     */
    private fun checkAccessibilityService(): Boolean {
        val enabled = try {
            val accessibilityEnabled = Settings.Secure.getInt(
                contentResolver,
                Settings.Secure.ACCESSIBILITY_ENABLED
            )
            accessibilityEnabled == 1
        } catch (e: Exception) {
            false
        }
        
        statusText.text = if (enabled) {
            "✅ 无障碍服务已启用"
        } else {
            "⚠️ 请启用无障碍服务"
        }
        
        return enabled
    }
    
    /**
     * 打开无障碍设置
     */
    private fun openAccessibilitySettings() {
        val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
        startActivity(intent)
    }
    
    /**
     * 创建简单布局（临时方案）
     */
    private fun createSimpleLayout(): android.view.View {
        val layout = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
        }
        
        // 状态文本
        statusText = TextView(this).apply {
            id = R.id.status_text
            text = "初始化中..."
            textSize = 16f
        }
        layout.addView(statusText)
        
        // 目标输入
        goalInput = EditText(this).apply {
            id = R.id.goal_input
            hint = "输入目标，如：打开微信"
        }
        layout.addView(goalInput)
        
        // 开始按钮
        startButton = Button(this).apply {
            id = R.id.start_button
            text = "开始执行"
        }
        layout.addView(startButton)
        
        // 停止按钮
        stopButton = Button(this).apply {
            id = R.id.stop_button
            text = "停止"
        }
        layout.addView(stopButton)
        
        // 设置按钮
        settingsButton = Button(this).apply {
            id = R.id.settings_button
            text = "无障碍设置"
        }
        layout.addView(settingsButton)
        
        return layout
    }
}

// 临时 ID 定义（实际应该在 res/values/ids.xml）
object R {
    object id {
        const val status_text = 1
        const val goal_input = 2
        const val start_button = 3
        const val stop_button = 4
        const val settings_button = 5
    }
}
