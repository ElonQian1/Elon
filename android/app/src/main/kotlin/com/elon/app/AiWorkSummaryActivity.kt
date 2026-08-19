package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

internal data class WorkSummaryItem(
    val project: String,
    val title: String,
    val reason: String,
    val suggestion: String,
    val primaryAction: String,
    val secondaryAction: String,
    val highPriority: Boolean = false,
)

class AiWorkSummaryActivity : AppCompatActivity() {
    private val regular = Typeface.create("sans-serif", Typeface.NORMAL)
    private val attentionItems = listOf(
        WorkSummaryItem("一龙网游加速器", "Windows 端末检测出新问题", "大卫提出了2个兼容性问题\n目前还没有负责人确认", "建议先确认系统兼容性问题", "交给 AI 处理", "进入项目", true),
        WorkSummaryItem("新项目4", "APK 构建已完成", "等待你是否进入测试阶段。", "建议先进入测试相关内容", "进入测试", "查看项目"),
        WorkSummaryItem("牛宝", "主页 UI 修改已完成但未发布", "等待你的发布确认", "建议发布新版本", "交给 AI 处理", "查看详情"),
    )
    private val progressItems = listOf(
        "杀蟑螂" to "完成了个人页面中心优化",
        "牛宝" to "修复了交易界面上下自动弹回问题",
    )
    private val confirmItems = listOf("大卫" to "大卫提交发布了新版本。")

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = elonColor(R.color.elon_bg_app)
        window.navigationBarColor = elonColor(R.color.elon_bg_app)
        window.decorView.systemUiVisibility = 0
        setContentView(createContent())
    }

    private fun createContent(): View = ScrollView(this).apply {
        isFillViewport = true
        clipToPadding = false
        setBackgroundColor(elonColor(R.color.elon_bg_app))
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(6), dp(20), dp(40))
            addView(createToolbar())
            addView(createDateRow())
            addView(createGreeting())
            addView(createMetrics())
            addView(sectionTitle("需要你关注", attentionItems.size))
            attentionItems.forEach { addView(attentionCard(it)) }
            addView(collapsibleSection("有新进展", progressItems))
            addView(collapsibleSection("待确认", confirmItems))
        })
    }

    private fun createToolbar(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(56))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(ImageButton(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_toolbar_back_custom)
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = "返回"
            scaleType = ImageView.ScaleType.CENTER
            setOnClickListener { finish() }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(label("AI 工作摘要", 20f, "#F0F8F7F4", regular).apply { gravity = Gravity.CENTER },
            LinearLayout.LayoutParams(0, MATCH, 1f))
        addView(View(this@AiWorkSummaryActivity), LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun createDateRow(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(48))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(48), 0, 0, 0)
        addView(label("今天  ⌄", 17f, "#E7ECEB", regular).apply { gravity = Gravity.CENTER_VERTICAL }, LinearLayout.LayoutParams(0, MATCH, 1f))
        addView(ImageButton(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_work_summary_calendar)
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = "选择摘要日期"
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(9), dp(9), dp(9), dp(9))
            setOnClickListener { toast("日期选择即将开放") }
        }, LinearLayout.LayoutParams(dp(52), dp(52)))
    }

    private fun createGreeting(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(94)).apply { topMargin = dp(18) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(ImageView(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_home_ai_avatar)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
        }, LinearLayout.LayoutParams(dp(72), dp(72)).apply { marginStart = dp(16) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            addView(label("早上好", 20f, "#F0F8F7F4", regular))
            addView(label("AI 已分析你的 21 个项目", 16f, "#CDD2D1", regular).apply {
                setPadding(0, dp(7), 0, 0)
            })
        }, LinearLayout.LayoutParams(0, WRAP, 1f).apply { marginStart = dp(14) })
    }

    private fun createMetrics(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(88)).apply {
            topMargin = dp(13)
            bottomMargin = dp(24)
        }
        orientation = LinearLayout.HORIZONTAL
        addView(metric("3", "需要你关注", "#8EAED0"), weighted())
        addView(metric("2", "有新进展", "#70BB7E"), weighted(dp(17)))
        addView(metric("1", "待确认", "#F08A3C"), weighted(dp(17)))
    }

    private fun metric(number: String, caption: String, color: String): View = LinearLayout(this).apply {
        background = rounded("#353A42", 12)
        gravity = Gravity.CENTER
        orientation = LinearLayout.VERTICAL
        addView(label(number, 18f, color, regular))
        addView(label(caption, 14f, "#E5E8E7", regular).apply { setPadding(0, dp(6), 0, 0) })
    }

    private fun sectionTitle(title: String, count: Int): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(50))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(label(title, 18f, "#F0F8F7F4", regular))
        addView(label(count.toString(), 13f, "#D7DDDC", regular).apply {
            background = rounded("#1A1F27", 13)
            gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(dp(30), dp(24)).apply { marginStart = dp(9) })
    }

    private fun attentionCard(item: WorkSummaryItem): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, WRAP).apply { bottomMargin = dp(17) }
        background = rounded("#181D25", 18)
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(22), dp(32), dp(22))
        if (item.highPriority) addView(label("高优先级", 12f, "#DE5A4A", regular).apply {
            background = rounded("#40201E", 5)
            gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(dp(60), dp(24)).apply { bottomMargin = dp(10) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(projectIcon(item.project), LinearLayout.LayoutParams(dp(52), dp(52)))
            addView(LinearLayout(this@AiWorkSummaryActivity).apply {
                orientation = LinearLayout.VERTICAL
                addView(label(item.project, 16f, "#F0F8F7F4", regular))
                addView(label(item.title, 15f, "#E2E6E5", regular).apply {
                    maxLines = 2
                    setPadding(0, dp(7), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, WRAP, 1f).apply { marginStart = dp(18) })
            addView(label("›", 31f, "#E1E5E4", regular).apply { gravity = Gravity.CENTER },
                LinearLayout.LayoutParams(dp(32), dp(48)))
        })
        addView(label(item.reason, 14f, "#9AA09F", regular).apply {
            setPadding(dp(5), dp(16), 0, 0); setLineSpacing(dp(3).toFloat(), 1f)
        })
        addView(label("AI 建议", 15f, "#8FAEC5", regular).apply { setPadding(dp(5), dp(18), 0, 0) })
        addView(label(item.suggestion, 14f, "#9AA09F", regular).apply { setPadding(dp(5), dp(8), 0, 0) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.HORIZONTAL
            addView(actionButton(item.secondaryAction, false) { toast("正在打开${item.project}") }, weighted())
            addView(actionButton(item.primaryAction, true) { toast("已将“${item.title}”交给 AI") }, weighted(dp(12)))
        }, LinearLayout.LayoutParams(MATCH, dp(48)).apply { topMargin = dp(9) })
    }

    private fun projectIcon(project: String): View = TextView(this).apply {
        background = rounded("#E8E8E7", 12)
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = project.take(1)
        textSize = 22f
        typeface = regular
        setTextColor(Color.parseColor("#312C2B"))
    }

    private fun actionButton(text: String, primary: Boolean, action: () -> Unit): View = FrameLayout(this).apply {
        isClickable = true
        isFocusable = true
        setOnClickListener { action() }
        addView(label(text, 14f, if (primary) "#111820" else "#E8ECEB", regular).apply {
            background = rounded(if (primary) "#8EA7D5" else "#353A42", 11)
            gravity = Gravity.CENTER
        }, FrameLayout.LayoutParams(MATCH, dp(38), Gravity.CENTER))
    }

    private fun collapsibleSection(title: String, items: List<Pair<String, String>>): View = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        val content = LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            items.forEach { (project, update) -> addView(updateRow(project, update)) }
        }
        val chevron = label("⌃", 21f, "#E1E5E4", regular).apply { gravity = Gravity.CENTER }
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            layoutParams = LinearLayout.LayoutParams(MATCH, dp(60)).apply { topMargin = dp(8) }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            addView(sectionTitle(title, items.size), LinearLayout.LayoutParams(0, MATCH, 1f))
            addView(chevron, LinearLayout.LayoutParams(dp(48), dp(48)))
            setOnClickListener {
                content.visibility = if (content.visibility == View.VISIBLE) View.GONE else View.VISIBLE
                chevron.text = if (content.visibility == View.VISIBLE) "⌃" else "⌄"
            }
        })
        addView(content)
    }

    private fun updateRow(project: String, update: String): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(78))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(16), 0, dp(4), 0)
        addView(projectIcon(project), LinearLayout.LayoutParams(dp(52), dp(52)))
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            addView(label(project, 16f, "#E8ECEB", regular))
            addView(label("•  $update", 14f, "#818786", regular).apply {
                maxLines = 1; setPadding(0, dp(8), 0, 0)
            })
        }, LinearLayout.LayoutParams(0, WRAP, 1f).apply { marginStart = dp(18) })
        addView(label("›", 29f, "#8FAEC5", regular).apply { gravity = Gravity.CENTER },
            LinearLayout.LayoutParams(dp(42), dp(48)))
    }

    private fun label(value: String, size: Float, color: String, face: Typeface) = TextView(this).apply {
        includeFontPadding = false
        text = value
        textSize = size
        typeface = face
        setTextColor(Color.parseColor(color))
    }

    private fun weighted(start: Int = 0) = LinearLayout.LayoutParams(0, MATCH, 1f).apply { marginStart = start }
    private fun rounded(color: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(color)); cornerRadius = dp(radius).toFloat()
    }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
    private fun toast(message: String) = Toast.makeText(this, message, Toast.LENGTH_SHORT).show()

    private companion object {
        const val MATCH = LinearLayout.LayoutParams.MATCH_PARENT
        const val WRAP = LinearLayout.LayoutParams.WRAP_CONTENT
    }
}
