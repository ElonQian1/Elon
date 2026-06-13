// infrastructure/floating/FloatingInputActivity.kt
// module: infrastructure/floating | layer: infrastructure | role: text-input-activity
// summary: 文字输入透明Activity - 从悬浮球双击触发

package com.elon.app.agent.infrastructure.floating

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

/**
 * ⌨️ 文字输入透明Activity
 * 
 * 从悬浮球双击触发，显示文字输入界面
 */
class FloatingInputActivity : AppCompatActivity() {
    
    private lateinit var inputField: EditText
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // 设置透明窗口
        window.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND)
            setDimAmount(0.6f)
            setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_VISIBLE)
        }
        
        // 创建UI
        createUI()
        
        // 自动聚焦输入框
        inputField.postDelayed({
            inputField.requestFocus()
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            imm.showSoftInput(inputField, InputMethodManager.SHOW_IMPLICIT)
        }, 200)
    }
    
    private fun createUI() {
        val density = resources.displayMetrics.density
        
        val rootLayout = FrameLayout(this).apply {
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { finish() }  // 点击背景关闭
        }
        
        // 中央卡片
        val card = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(
                (24 * density).toInt(),
                (24 * density).toInt(),
                (24 * density).toInt(),
                (24 * density).toInt()
            )
            
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#222222"))
                cornerRadius = 20 * density
            }
            background = bg
            elevation = 16 * density
            
            // 阻止点击穿透
            setOnClickListener { }
        }
        
        // 标题
        val titleText = TextView(this).apply {
            text = "⌨️ 输入任务"
            textSize = 20f
            setTextColor(Color.parseColor("#D6D6D6"))
            gravity = Gravity.CENTER
        }
        card.addView(titleText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        
        // 输入框
        inputField = EditText(this).apply {
            hint = "请输入任务目标...\n例如：打开微信，给张三发消息"
            textSize = 16f
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#777777"))
            minLines = 3
            maxLines = 5
            gravity = Gravity.TOP or Gravity.START
            
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#151515"))
                cornerRadius = 12 * density
                setStroke((1 * density).toInt(), Color.parseColor("#2E2E2E"))
            }
            background = bg
            setPadding(
                (16 * density).toInt(),
                (12 * density).toInt(),
                (16 * density).toInt(),
                (12 * density).toInt()
            )
            
            // 回车提交
            imeOptions = EditorInfo.IME_ACTION_SEND
            setOnEditorActionListener { _, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_SEND) {
                    submitTask()
                    true
                } else {
                    false
                }
            }
        }
        card.addView(inputField, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = (16 * density).toInt()
        })
        
        // 按钮行
        val buttonRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.END
        }
        
        // 取消按钮
        val cancelButton = Button(this).apply {
            text = "取消"
            textSize = 14f
            setTextColor(Color.parseColor("#A8A8A8"))
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { finish() }
        }
        buttonRow.addView(cancelButton, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        
        // 发送按钮
        val sendButton = Button(this).apply {
            text = "🚀 执行"
            textSize = 14f
            setTextColor(Color.parseColor("#101010"))
            
            val bg = GradientDrawable().apply {
                setColor(Color.parseColor("#58BE6A"))
                cornerRadius = 8 * density
            }
            background = bg
            setPadding(
                (20 * density).toInt(),
                (8 * density).toInt(),
                (20 * density).toInt(),
                (8 * density).toInt()
            )
            
            setOnClickListener { submitTask() }
        }
        buttonRow.addView(sendButton, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = (12 * density).toInt()
        })
        
        card.addView(buttonRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = (16 * density).toInt()
        })
        
        // 快捷任务
        val quickTasksLabel = TextView(this).apply {
            text = "快捷任务："
            textSize = 12f
            setTextColor(Color.parseColor("#777777"))
        }
        card.addView(quickTasksLabel, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = (20 * density).toInt()
        })
        
        // 快捷按钮
        val quickTasksRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
        }
        
        val quickTasks = listOf(
            "打开微信",
            "打开小红书",
            "截图保存"
        )
        
        for (task in quickTasks) {
            val chip = TextView(this).apply {
                text = task
                textSize = 12f
                setTextColor(Color.parseColor("#D6D6D6"))
                
                val bg = GradientDrawable().apply {
                    setColor(Color.parseColor("#2A2A2A"))
                    cornerRadius = 16 * density
                }
                background = bg
                setPadding(
                    (12 * density).toInt(),
                    (6 * density).toInt(),
                    (12 * density).toInt(),
                    (6 * density).toInt()
                )
                
                setOnClickListener {
                    inputField.setText(task)
                    inputField.setSelection(task.length)
                }
            }
            quickTasksRow.addView(chip, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginEnd = (8 * density).toInt()
            })
        }
        
        card.addView(quickTasksRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = (8 * density).toInt()
        })
        
        // 添加卡片到根布局
        rootLayout.addView(card, FrameLayout.LayoutParams(
            (320 * density).toInt(),
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            gravity = Gravity.CENTER
        })
        
        setContentView(rootLayout)
    }
    
    private fun submitTask() {
        val goal = inputField.text.toString().trim()
        
        if (goal.isBlank()) {
            Toast.makeText(this, "请输入任务目标", Toast.LENGTH_SHORT).show()
            return
        }
        
        // 隐藏键盘
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.hideSoftInputFromWindow(inputField.windowToken, 0)
        
        // 检查无障碍服务是否运行
        if (!com.elon.app.agent.AgentService.isRunning()) {
            Toast.makeText(this, "❌ 请先开启无障碍服务", Toast.LENGTH_LONG).show()
            finish()
            return
        }
        
        // 直接调用 AgentService 执行任务
        com.elon.app.agent.AgentService.executeTask(goal)
        
        Toast.makeText(this, "🚀 任务已提交: $goal", Toast.LENGTH_SHORT).show()
        
        finish()
    }
}
