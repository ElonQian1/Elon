package com.elon.app

import android.view.inputmethod.InputMethodManager
import android.content.Context
import android.text.InputType
import android.view.View
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal object ProjectCreateDialog {
    fun show(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        defaultTitle: String,
        onCreate: (title: String, nodeId: String) -> Unit
    ) {
        val nameInput = EditText(activity).apply {
            setText(defaultTitle)
            selectAll()
            hint = "项目名称"
            maxLines = 1
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        }
        val nodeStatus = TextView(activity).apply {
            text = "正在加载可用 PC 节点..."
            textSize = 13f
        }
        val nodeSpinner = Spinner(activity).apply {
            visibility = View.GONE
        }
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            val padding = dp(activity, 20)
            setPadding(padding, dp(activity, 8), padding, 0)
            addView(nameInput)
            addView(nodeStatus, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(activity, 14) })
            addView(nodeSpinner, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(activity, 8) })
        }

        var onlineNodes = emptyList<ProjectCreateNodeOption>()
        val dialog = AlertDialog.Builder(activity)
            .setTitle("新建项目")
            .setView(content)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            val positive = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
            positive.isEnabled = false
            positive.setOnClickListener {
                val title = nameInput.text.toString().trim()
                if (title.isBlank()) {
                    nameInput.error = "请输入项目名称"
                    return@setOnClickListener
                }
                val selected = onlineNodes.getOrNull(nodeSpinner.selectedItemPosition)
                if (selected == null) {
                    nodeStatus.text = "请先启动并选择一个在线 PC 节点"
                    return@setOnClickListener
                }
                dialog.dismiss()
                onCreate(title, selected.nodeId)
            }
            loadNodes(activity, http, serverUrl, nodeStatus, nodeSpinner) { nodes ->
                onlineNodes = nodes
                positive.isEnabled = onlineNodes.isNotEmpty()
            }
            nameInput.post {
                nameInput.requestFocus()
                val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
                imm?.showSoftInput(nameInput, InputMethodManager.SHOW_IMPLICIT)
            }
        }
        dialog.show()
    }

    private fun loadNodes(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        nodeStatus: TextView,
        nodeSpinner: Spinner,
        onLoaded: (List<ProjectCreateNodeOption>) -> Unit
    ) {
        thread(name = "project-create-nodes") {
            val result = runCatching {
                fetchProjectCreateNodes(http, serverUrl, activity)
                    .filter { it.online && it.cliProjectReady }
                    .sortedBy { it.displayName }
            }
            activity.runOnUiThread {
                result
                    .onSuccess { nodes ->
                        onLoaded(nodes)
                        if (nodes.isEmpty()) {
                            nodeSpinner.visibility = View.GONE
                            nodeStatus.text = "没有可用于项目的在线 PC 节点。请先启动已配置 Codex/Copilot 的 PC 节点。"
                            return@onSuccess
                        }
                        nodeSpinner.visibility = View.VISIBLE
                        nodeStatus.text = if (nodes.size == 1) {
                            "将创建到这个在线 PC 节点"
                        } else {
                            "选择项目代码要创建到哪个 PC 节点"
                        }
                        nodeSpinner.adapter = ArrayAdapter(
                            activity,
                            android.R.layout.simple_spinner_dropdown_item,
                            nodes.map { node ->
                                val cli = node.allowedClis.takeIf { it.isNotEmpty() }?.joinToString("/") ?: "CLI"
                                val projectSuffix = node.projectCount.takeIf { it > 0 }?.let { " · ${it}个项目" }.orEmpty()
                                "${node.displayName} · ${node.shortId}$projectSuffix · $cli"
                            }
                        )
                    }
                    .onFailure { error ->
                        onLoaded(emptyList())
                        nodeSpinner.visibility = View.GONE
                        nodeStatus.text = "加载 PC 节点失败：${error.message ?: "未知错误"}"
                    }
            }
        }
    }

    private fun dp(context: Context, value: Int): Int {
        return (value * context.resources.displayMetrics.density).toInt()
    }
}
