package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.app.DatePickerDialog
import android.content.Intent
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
import com.google.gson.Gson

internal data class WorkSummaryItem(
    val project: String,
    val title: String,
    val reason: String,
    val suggestion: String,
    val primaryAction: String,
    val secondaryAction: String,
    val highPriority: Boolean = false,
    val highlightPrimary: Boolean = true,
)

internal data class WorkSummaryUpdate(val project: String, val update: String, val date: String)

class AiWorkSummaryActivity : AppCompatActivity() {
    private val regular = Typeface.create("sans-serif", Typeface.NORMAL)
    private val storedProjects by lazy {
        loadStoredProjects(AuthManager.userDataPrefs(this), Gson(), {}, null).projects
    }
    private val attentionItems = listOf(
        WorkSummaryItem("一龙网游加速器", "Windows 端末检测出新问题", "大卫提出了2个兼容性问题\n目前还没有负责人确认", "建议先确认系统兼容性问题", "交给 AI 处理", "进入项目", true, true),
        WorkSummaryItem("新项目4", "APK 构建已完成", "等待你是否进入测试阶段。", "建议先进入测试相关内容", "查看项目", "进入测试", highlightPrimary = false),
        WorkSummaryItem("牛宝", "主页 UI 修改已完成但未发布", "等待你的发布确认", "建议发布新版本", "交给 AI 处理", "查看详情"),
    )
    private val progressItems = listOf(
        WorkSummaryUpdate("杀蟑螂", "完成了个人页面中心优化", "8月17号"),
        WorkSummaryUpdate("牛宝", "修复了交易界面上下自动弹回问题", "8月18号"),
    )
    private val confirmItems = listOf(WorkSummaryUpdate("大卫", "大卫提交发布了新版本。", "8月17号"))

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
            setPadding(dp(15), 0, dp(19), dp(40))
            addView(createToolbar())
            addView(createDateRow())
            addView(createGreeting())
            addView(createMetrics())
            addView(sectionTitle("需要你关注", attentionItems.size))
            attentionItems.forEachIndexed { index, item -> addView(attentionCard(item, index == attentionItems.lastIndex)) }
            addView(collapsibleSection("有新进展", progressItems))
            addView(collapsibleSection("待确认", confirmItems))
        })
    }

    private fun createToolbar(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(52))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(ImageButton(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_toolbar_back_custom)
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = "返回"
            scaleType = ImageView.ScaleType.CENTER
            setOnClickListener { finish() }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(label("AI 工作摘要", 18f, "#F0F8F7F4", regular).apply { gravity = Gravity.CENTER },
            LinearLayout.LayoutParams(0, MATCH, 1f))
        addView(View(this@AiWorkSummaryActivity), LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun createDateRow(): View = FrameLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(38))
        val dateLabel = label("今天", 15f, "#E7ECEB", regular).apply {
            gravity = Gravity.CENTER_VERTICAL
        }
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            contentDescription = "选择摘要日期"
            setOnClickListener { showDatePicker(dateLabel) }
            addView(dateLabel, LinearLayout.LayoutParams(WRAP, MATCH))
            addView(ImageView(this@AiWorkSummaryActivity).apply {
                setImageResource(R.drawable.ic_input_model_chevron)
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(14), dp(14)).apply { marginStart = dp(6) })
        }, FrameLayout.LayoutParams(dp(96), dp(48), Gravity.CENTER))
        addView(ImageButton(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_work_summary_calendar)
            setBackgroundColor(Color.TRANSPARENT)
            contentDescription = "选择摘要日期"
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(12), dp(12), dp(12), dp(12))
            setOnClickListener { showDatePicker(dateLabel) }
        }, FrameLayout.LayoutParams(dp(48), dp(48), Gravity.CENTER_VERTICAL or Gravity.END))
    }

    private fun createGreeting(): View = FrameLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(94)).apply { topMargin = dp(11) }
        addView(ImageView(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_home_ai_avatar)
            scaleType = ImageView.ScaleType.CENTER_INSIDE
        }, FrameLayout.LayoutParams(dp(86), dp(86), Gravity.CENTER_VERTICAL).apply { marginStart = dp(16) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            addView(label("早上好", 17f, "#F0F8F7F4", regular))
            addView(label("AI 已分析你的 21 个项目", 14f, "#CDD2D1", regular).apply {
                setPadding(0, dp(6), 0, 0)
            })
        }, FrameLayout.LayoutParams(WRAP, WRAP, Gravity.CENTER_VERTICAL).apply { marginStart = dp(94) })
    }

    private fun createMetrics(): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(82)).apply {
            topMargin = dp(13)
            bottomMargin = dp(22)
        }
        orientation = LinearLayout.HORIZONTAL
        addView(metric("3", "需要你关注", "#8EAED0"), weighted())
        addView(metric("2", "有新进展", "#70BB7E"), weighted(dp(16)))
        addView(metric("1", "待确认", "#F08A3C"), weighted(dp(16)))
    }

    private fun metric(number: String, caption: String, color: String): View = LinearLayout(this).apply {
        background = rounded("#353A42", 12)
        gravity = Gravity.CENTER
        orientation = LinearLayout.VERTICAL
        addView(label(number, 16f, color, regular))
        addView(label(caption, 13f, "#E5E8E7", regular).apply { setPadding(0, dp(6), 0, 0) })
    }

    private fun sectionTitle(title: String, count: Int): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, dp(34))
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        addView(label(title, 16f, "#F0F8F7F4", regular))
        addView(label(count.toString(), 12f, "#D7DDDC", regular).apply {
            background = rounded("#1A1F27", 13)
            gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(dp(20), dp(21)).apply { marginStart = dp(9) })
    }

    private fun attentionCard(item: WorkSummaryItem, isLast: Boolean): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, WRAP).apply { bottomMargin = if (isLast) 0 else dp(16) }
        minimumHeight = dp(if (item.highPriority) 268 else 258)
        background = rounded("#181D25", 18)
        orientation = LinearLayout.VERTICAL
        setPadding(dp(30), dp(21), dp(30), dp(21))
        if (item.highPriority) addView(label("高优先级", 11f, "#DE5A4A", regular).apply {
            background = rounded("#40201E", 5)
            gravity = Gravity.CENTER
        }, LinearLayout.LayoutParams(dp(60), dp(20)).apply { bottomMargin = dp(7) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(projectIcon(item.project), LinearLayout.LayoutParams(dp(48), dp(48)))
            addView(LinearLayout(this@AiWorkSummaryActivity).apply {
                orientation = LinearLayout.VERTICAL
                addView(label(item.project, 14f, "#F0F8F7F4", regular))
                addView(label(item.title, 14f, "#E2E6E5", regular).apply {
                    setPadding(0, dp(7), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, WRAP, 1f).apply { marginStart = dp(17) })
            addView(ImageButton(this@AiWorkSummaryActivity).apply {
                setImageResource(R.drawable.ic_project_space_chevron_right)
                setBackgroundColor(Color.TRANSPARENT)
                contentDescription = "进入${item.project}"
                scaleType = ImageView.ScaleType.CENTER
                setOnClickListener { openProject(item.project) }
            },
                LinearLayout.LayoutParams(dp(32), dp(48)))
        })
        addView(label(item.reason, 13f, "#737877", regular).apply {
            setPadding(dp(4), dp(15), 0, 0); setLineSpacing(dp(3).toFloat(), 1f)
        })
        addView(label("AI 建议", 14f, "#8FAEC5", regular).apply {
            setPadding(dp(4), if (item.highPriority) dp(18) else dp(36), 0, 0)
        })
        addView(label(item.suggestion, 13f, "#737877", regular).apply { setPadding(dp(4), dp(7), 0, 0) })
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.HORIZONTAL
            addView(actionButton(item.secondaryAction, false) { runAction(item, item.secondaryAction) }, weighted())
            addView(actionButton(item.primaryAction, item.highlightPrimary) { runAction(item, item.primaryAction) }, weighted(dp(12)))
        }, LinearLayout.LayoutParams(MATCH, dp(42)).apply { topMargin = dp(8) })
    }

    private fun projectIcon(project: String): View = FrameLayout(this).apply {
        background = rounded("#E8E8E7", 12)
        clipToOutline = true
        val bitmap = storedProjects.firstOrNull { it.title == project }?.iconDataUrl
            ?.let(UserProfileStore::decodeAvatar)
        if (bitmap != null) {
            addView(ImageView(this@AiWorkSummaryActivity).apply {
                setImageBitmap(bitmap)
                scaleType = ImageView.ScaleType.CENTER_CROP
                contentDescription = "$project 项目图标"
            }, FrameLayout.LayoutParams(MATCH, MATCH))
        } else {
            addView(label(project.take(1), 21f, "#312C2B", regular).apply { gravity = Gravity.CENTER },
                FrameLayout.LayoutParams(MATCH, MATCH))
        }
    }

    private fun actionButton(text: String, primary: Boolean, action: () -> Unit): View = FrameLayout(this).apply {
        isClickable = true
        isFocusable = true
        setOnClickListener { action() }
        addView(label(text, 13f, if (primary) "#F2F5F4" else "#E8ECEB", regular).apply {
            background = rounded(if (primary) "#3D83ED" else "#353A42", 11)
            gravity = Gravity.CENTER
        }, FrameLayout.LayoutParams(MATCH, dp(36), Gravity.CENTER))
    }

    private fun collapsibleSection(title: String, items: List<WorkSummaryUpdate>): View = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        val content = LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            items.forEach { item -> addView(updateRow(item)) }
            addView(LinearLayout(this@AiWorkSummaryActivity).apply {
                gravity = Gravity.CENTER_VERTICAL or Gravity.END
                orientation = LinearLayout.HORIZONTAL
                setPadding(0, 0, dp(22), 0)
                isClickable = true
                isFocusable = true
                contentDescription = "查看${title}全部内容"
                setOnClickListener { toast("已展开${title}全部内容") }
                addView(label("查看全部", 15f, "#8FAEC5", regular))
                addView(ImageView(this@AiWorkSummaryActivity).apply {
                    setImageResource(R.drawable.ic_project_space_chevron_right)
                    setColorFilter(Color.parseColor("#8FAEC5"))
                    contentDescription = null
                }, LinearLayout.LayoutParams(dp(18), dp(18)).apply { marginStart = dp(8) })
            }, LinearLayout.LayoutParams(MATCH, dp(48)))
        }
        val chevron = ImageView(this@AiWorkSummaryActivity).apply {
            setImageResource(R.drawable.ic_input_model_chevron)
            rotation = 180f
            contentDescription = null
        }
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            layoutParams = LinearLayout.LayoutParams(MATCH, dp(55)).apply { topMargin = dp(5) }
            setPadding(dp(13), 0, dp(9), 0)
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            contentDescription = "收起$title"
            addView(sectionTitle(title, items.size), LinearLayout.LayoutParams(0, MATCH, 1f))
            addView(chevron, LinearLayout.LayoutParams(dp(48), dp(48)))
            setOnClickListener {
                content.visibility = if (content.visibility == View.VISIBLE) View.GONE else View.VISIBLE
                chevron.rotation = if (content.visibility == View.VISIBLE) 180f else 0f
                contentDescription = if (content.visibility == View.VISIBLE) "收起$title" else "展开$title"
            }
        })
        addView(content)
    }

    private fun updateRow(item: WorkSummaryUpdate): View = LinearLayout(this).apply {
        layoutParams = LinearLayout.LayoutParams(MATCH, WRAP)
        minimumHeight = dp(76)
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(29), dp(14), dp(4), dp(14))
        addView(projectIcon(item.project), LinearLayout.LayoutParams(dp(48), dp(48)))
        addView(LinearLayout(this@AiWorkSummaryActivity).apply {
            orientation = LinearLayout.VERTICAL
            addView(label(item.project, 16f, "#E8ECEB", regular))
            addView(label("•  ${item.update}", 14f, "#818786", regular).apply {
                setPadding(0, dp(8), 0, 0)
            })
        }, LinearLayout.LayoutParams(0, WRAP, 1f).apply { marginStart = dp(16) })
        addView(label(item.date, 13f, "#707574", regular).apply { gravity = Gravity.TOP or Gravity.END },
            LinearLayout.LayoutParams(dp(68), dp(48)))
    }

    private fun label(value: String, size: Float, color: String, face: Typeface) = TextView(this).apply {
        includeFontPadding = false
        text = value
        textSize = size
        typeface = face
        setTextColor(Color.parseColor(color))
    }

    private fun weighted(start: Int = 0) = LinearLayout.LayoutParams(0, MATCH, 1f).apply { marginStart = start }

    private fun showDatePicker(dateLabel: TextView) {
        val today = java.util.Calendar.getInstance()
        DatePickerDialog(this, { _, year, month, day ->
            dateLabel.text = if (year == today.get(java.util.Calendar.YEAR) && month == today.get(java.util.Calendar.MONTH) && day == today.get(java.util.Calendar.DAY_OF_MONTH)) {
                "今天"
            } else {
                "${month + 1}月${day}日"
            }
        }, today.get(java.util.Calendar.YEAR), today.get(java.util.Calendar.MONTH), today.get(java.util.Calendar.DAY_OF_MONTH)).show()
    }

    private fun openProject(project: String) {
        startActivity(Intent(this, MainActivity::class.java).apply {
            putExtra(EXTRA_OPEN_WORK_SUMMARY_PROJECT_TITLE, project)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        })
    }

    private fun runAction(item: WorkSummaryItem, action: String) {
        if (action == "进入项目" || action == "查看项目" || action == "查看详情") {
            openProject(item.project)
            return
        }
        val prompt = if (action == "进入测试") {
            "请为${item.project}进入测试阶段，并先检查 APK 构建结果。"
        } else {
            "请处理${item.project}的事项：${item.title}。${item.suggestion}。"
        }
        startActivity(Intent(this, MainActivity::class.java).apply {
            putExtra(EXTRA_WORK_SUMMARY_AI_PROMPT, prompt)
            putExtra(EXTRA_WORK_SUMMARY_AUTO_SEND, true)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        })
    }
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
