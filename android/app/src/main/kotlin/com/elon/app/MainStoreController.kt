package com.elon.app

import android.graphics.Color
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

/**
 * 项目商店 UI 控制器。
 *
 * 用法：`MainStoreController(...).showStoreDialog()`
 *
 * 核心流程：
 *   1. 显示 AlertDialog + 搜索栏 + 滚动列表
 *   2. 后台线程拉取公开项目列表
 *   3. 点击行 → 详情 + "加入" 按钮
 *   4. 加入后本地写入 AppProject，切换到该项目
 */
internal class MainStoreController(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val tokenProvider: () -> String?,
    private val isLoggedIn: () -> Boolean,
    private val addJoinedProject: (StoreProject) -> Unit,
    private val dp: (Int) -> Int
) {

    fun showStoreDialog() {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(4), dp(8), dp(4), dp(4))
            setBackgroundColor(Color.parseColor("#1C1C1C"))
        }

        // ── 搜索栏 ──────────────────────────────────────────────────────────
        val searchField = EditText(activity).apply {
            hint = "搜索项目名称..."
            setHintTextColor(Color.parseColor("#777777"))
            setTextColor(Color.parseColor("#D6D6D6"))
            textSize = 14f
            setPadding(dp(12), dp(10), dp(12), dp(10))
            background = createRoundedBg(8, "#2A2A2A")
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { setMargins(dp(8), dp(4), dp(8), dp(10)) }
            maxLines = 1
            imeOptions = android.view.inputmethod.EditorInfo.IME_ACTION_SEARCH
        }
        root.addView(searchField)

        // ── 状态提示 / 列表容器 ──────────────────────────────────────────────
        val statusText = TextView(activity).apply {
            text = "加载中..."
            setTextColor(Color.parseColor("#777777"))
            textSize = 13f
            gravity = Gravity.CENTER
            setPadding(dp(16), dp(24), dp(16), dp(24))
            visibility = View.VISIBLE
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        val listContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            visibility = View.GONE
        }

        val scroll = ScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(400)
            )
            addView(listContainer)
        }

        root.addView(statusText)
        root.addView(scroll)

        val dialog = AlertDialog.Builder(activity, android.R.style.Theme_DeviceDefault_Dialog_NoActionBar)
            .setTitle("发现项目")
            .setView(root)
            .setNegativeButton("关闭", null)
            .create()
        dialog.show()
        dialog.window?.setLayout(
            (activity.resources.displayMetrics.widthPixels * 0.93f).toInt(),
            android.view.WindowManager.LayoutParams.WRAP_CONTENT
        )

        // ── 搜索防抖（500 ms） ─────────────────────────────────────────────
        var searchDebounce: Runnable? = null
        searchField.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, st: Int, c: Int, a: Int) = Unit
            override fun onTextChanged(s: CharSequence?, st: Int, b: Int, c: Int) = Unit
            override fun afterTextChanged(s: Editable?) {
                searchDebounce?.let { activity.window.decorView.removeCallbacks(it) }
                searchDebounce = Runnable {
                    loadProjects(s?.toString(), statusText, listContainer, dialog)
                }.also { activity.window.decorView.postDelayed(it, 500) }
            }
        })

        // ── 初始加载 ──────────────────────────────────────────────────────
        loadProjects(null, statusText, listContainer, dialog)
    }

    // ─── 加载并渲染列表 ────────────────────────────────────────────────────

    private fun loadProjects(
        search: String?,
        statusText: TextView,
        listContainer: LinearLayout,
        dialog: AlertDialog
    ) {
        statusText.text = "加载中..."
        statusText.visibility = View.VISIBLE
        listContainer.visibility = View.GONE
        listContainer.removeAllViews()

        Thread {
            try {
                val projects = fetchStoreProjects(http, serverUrl, search?.trim()?.ifBlank { null })
                activity.runOnUiThread {
                    if (!dialog.isShowing) return@runOnUiThread
                    statusText.visibility = View.GONE
                    if (projects.isEmpty()) {
                        statusText.text = if (search.isNullOrBlank()) "暂无公开项目" else "没有匹配的项目"
                        statusText.visibility = View.VISIBLE
                    } else {
                        listContainer.visibility = View.VISIBLE
                        projects.forEachIndexed { i, p ->
                            listContainer.addView(createProjectRow(i, p, dialog))
                        }
                    }
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    if (dialog.isShowing) {
                        statusText.text = "加载失败：${e.message}"
                        statusText.visibility = View.VISIBLE
                        listContainer.visibility = View.GONE
                    }
                }
            }
        }.start()
    }

    // ─── 单行渲染 ──────────────────────────────────────────────────────────

    private fun createProjectRow(index: Int, project: StoreProject, dialog: AlertDialog): View {
        val row = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(12), dp(12), dp(12))
            setBackgroundColor(if (index % 2 == 0) Color.parseColor("#222222") else Color.parseColor("#222222"))
            isClickable = true
            setOnClickListener { showProjectDetail(project, dialog) }
        }

        row.addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL

            addView(TextView(activity).apply {
                text = project.name
                setTextColor(Color.parseColor("#D6D6D6"))
                textSize = 15f
                setTypeface(typeface, android.graphics.Typeface.BOLD)
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })

            val statusColor = when (project.lastTaskStatus) {
                "done" -> "#58BE6A"
                "running" -> "#58BE6A"
                "error" -> "#D97A7A"
                else -> "#777777"
            }
            addView(TextView(activity).apply {
                text = "${project.memberCount} 人"
                setTextColor(Color.parseColor(statusColor))
                textSize = 12f
                setPadding(dp(8), 0, 0, 0)
            })
        })

        if (!project.description.isNullOrBlank()) {
            row.addView(TextView(activity).apply {
                text = project.description
                setTextColor(Color.parseColor("#A8A8A8"))
                textSize = 12f
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
                setPadding(0, dp(3), 0, 0)
            })
        }

        row.addView(TextView(activity).apply {
            text = "模板：${project.template}  ·  ${projectJoinModeSummary(project.joinMode)}"
            setTextColor(Color.parseColor("#777777"))
            textSize = 11f
            setPadding(0, dp(2), 0, 0)
        })
        return row
    }

    // ─── 项目详情 + 加入 ───────────────────────────────────────────────────

    private fun showProjectDetail(project: StoreProject, parentDialog: AlertDialog) {
        val msg = buildString {
            append("创建者：${project.ownerAccount}\n")
            append("模板：${project.template}\n")
            append("成员：${project.memberCount} 人\n")
            append("加入方式：${projectJoinModeDetail(project.joinMode)}\n")
            if (!project.description.isNullOrBlank()) {
                append("\n${project.description}")
            }
        }

        val joinLabel = projectJoinActionLabel(project.joinMode)
        val builder = AlertDialog.Builder(activity)
            .setTitle(project.name)
            .setMessage(msg)
            .setNegativeButton("返回", null)

        if (project.joinMode != "invite") {
            builder.setPositiveButton(joinLabel) { _, _ -> doJoinProject(project, parentDialog) }
        }
        builder.show()
    }

    // ─── 执行加入 ─────────────────────────────────────────────────────────

    private fun doJoinProject(project: StoreProject, parentDialog: AlertDialog) {
        val token = tokenProvider()
        if (!isLoggedIn() || token == null) {
            Toast.makeText(activity, "请先登录再加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        Thread {
            try {
                joinStoreProject(http, serverUrl, project.id, token)
                activity.runOnUiThread {
                    addJoinedProject(project)
                    Toast.makeText(activity, projectJoinSuccessToast(project.joinMode), Toast.LENGTH_SHORT).show()
                    parentDialog.dismiss()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "加入失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    // ─── UI 工具 ──────────────────────────────────────────────────────────

    private fun createRoundedBg(radiusDp: Int, colorHex: String): android.graphics.drawable.GradientDrawable {
        return android.graphics.drawable.GradientDrawable().apply {
            shape = android.graphics.drawable.GradientDrawable.RECTANGLE
            cornerRadius = dp(radiusDp).toFloat()
            setColor(Color.parseColor(colorHex))
        }
    }
}
