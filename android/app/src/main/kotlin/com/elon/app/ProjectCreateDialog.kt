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
import java.util.Locale
import kotlin.concurrent.thread

internal object ProjectCreateDialog {
    fun show(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        defaultTitle: String,
        onCreate: (title: String, nodeId: String?) -> Unit
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
            // 按钮立即启用，不等节点加载——节点离线时也可先创建项目
            positive.isEnabled = true
            positive.setOnClickListener {
                val title = nameInput.text.toString().trim()
                if (title.isBlank()) {
                    nameInput.error = "请输入项目名称"
                    return@setOnClickListener
                }
                val selected = onlineNodes.getOrNull(nodeSpinner.selectedItemPosition)
                dialog.dismiss()
                onCreate(title, selected?.nodeId)
            }
            loadNodes(activity, http, serverUrl, nodeStatus, nodeSpinner) { nodes ->
                onlineNodes = nodes
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
                    .sortedWith(
                        compareBy<ProjectCreateNodeOption> { !it.canAcceptProject }
                            .thenBy { it.displayName }
                    )
            }
            activity.runOnUiThread {
                result
                    .onSuccess { nodes ->
                        val selectableNodes = nodes.filter { it.canAcceptProject }
                        onLoaded(selectableNodes)
                        if (selectableNodes.isEmpty()) {
                            nodeSpinner.visibility = View.GONE
                            nodeStatus.text = if (nodes.isEmpty()) {
                                "⚠️ 暂无在线 PC 节点，创建后工作区将在节点上线时自动初始化"
                            } else {
                                "⚠️ PC 节点暂不能接受新项目（${nodes.first().capacityHint()}），工作区将在节点就绪后初始化"
                            }
                            return@onSuccess
                        }
                        nodeSpinner.visibility = View.VISIBLE
                        nodeStatus.text = if (selectableNodes.size == 1) {
                            "将创建到这个在线 PC 节点"
                        } else {
                            "选择项目代码要创建到哪个 PC 节点"
                        }
                        nodeSpinner.adapter = ArrayAdapter(
                            activity,
                            android.R.layout.simple_spinner_dropdown_item,
                            selectableNodes.map { it.spinnerLabel() }
                        )
                    }
                    .onFailure { error ->
                        onLoaded(emptyList())
                        nodeSpinner.visibility = View.GONE
                        nodeStatus.text = "⚠️ 无法获取节点列表（${error.message ?: "网络错误"}），创建后工作区将在节点上线时自动初始化"
                    }
            }
        }
    }

    private fun dp(context: Context, value: Int): Int {
        return (value * context.resources.displayMetrics.density).toInt()
    }

    private fun ProjectCreateNodeOption.spinnerLabel(): String {
        val runtime = if (workspaceProvisionReady) "开发运行时" else "运行时未就绪"
        val aiCli = allowedClis.takeIf { it.isNotEmpty() }?.joinToString("/")?.let { "AI $it" }.orEmpty()
        return listOf(
            displayName,
            shortId,
            capacityLabel.ifBlank { "可创建项目" },
            projectSlotText(),
            diskText(),
            runtime,
            aiCli
        ).filter { it.isNotBlank() }.joinToString(" · ")
    }

    private fun ProjectCreateNodeOption.capacityHint(): String {
        return capacityWarnings.firstOrNull()
            ?: capacityLabel.takeIf { it.isNotBlank() }
            ?: when {
                !online -> "PC 节点离线"
                !workspaceProvisionReady -> "PC 开发运行时不可用"
                projectLimit > 0 && projectSlotsRemaining <= 0 -> "项目数已满"
                else -> "容量暂不可用"
            }
    }

    private fun ProjectCreateNodeOption.projectSlotText(): String {
        return if (projectLimit > 0) {
            "项目 ${projectCount}/${projectLimit}，剩余 ${projectSlotsRemaining.coerceAtLeast(0)}"
        } else {
            "项目 ${projectCount}"
        }
    }

    private fun ProjectCreateNodeOption.diskText(): String {
        return formatBytes(diskFreeBytes).takeIf { it.isNotBlank() }?.let { "磁盘 $it" }.orEmpty()
    }

    private fun formatBytes(value: Long?): String {
        val bytes = value ?: return ""
        if (bytes <= 0L) return ""
        val units = listOf("B", "KB", "MB", "GB", "TB")
        var amount = bytes.toDouble()
        var index = 0
        while (amount >= 1024.0 && index < units.lastIndex) {
            amount /= 1024.0
            index += 1
        }
        return if (index >= 3) {
            String.format(Locale.US, "%.1f %s", amount, units[index])
        } else {
            "${amount.toInt()} ${units[index]}"
        }
    }
}
