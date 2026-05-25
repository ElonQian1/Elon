package com.elon.app

import android.graphics.Color
import android.text.InputType
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal class MainProjectGitDialogs(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val userId: String,
    private val projectProvider: () -> AppProject,
    private val projectTitleProvider: () -> String,
    private val addProjectEvent: (String) -> Unit,
    private val openUrl: (String) -> Unit,
    private val copyText: (String, String) -> Unit
) {
    fun showGitProjectDialog() {
        val actions = arrayOf("查看同步状态", "查看通用工作流", "配置 GitHub 仓库", "生成并复制 Deploy Key", "打开 GitHub Deploy Keys", "授权说明")
        AlertDialog.Builder(activity)
            .setTitle("${projectTitleProvider()} · Git 仓库")
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "查看同步状态" -> loadGitProjectStatus { status -> showGitStatusDialog(status) }
                    "查看通用工作流" -> loadGitProjectStatus { status -> showProjectWorkflowDialog(status) }
                    "配置 GitHub 仓库" -> showConfigureGitDialog()
                    "生成并复制 Deploy Key" -> generateDeployKey()
                    "打开 GitHub Deploy Keys" -> loadGitProjectStatus { status -> openUrl(status.deployKeysUrl) }
                    "授权说明" -> showGitAuthHelpDialog()
                }
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun showGitStatusDialog(status: GitProjectStatus) {
        val remoteLine = when (status.remoteOk) {
            true -> "远端权限：正常"
            false -> "远端权限：未通过\n${status.remoteMessage.orEmpty().ifBlank { "请检查 Deploy Key 是否已加到 GitHub，并勾选写权限。" }}"
            null -> "远端权限：尚未配置远端"
        }
        AlertDialog.Builder(activity)
            .setTitle("${projectTitleProvider()} · Git 状态")
            .setMessage(
                buildString {
                    append("Git 工作区：${if (status.hasGit) "已准备" else "未初始化"}\n")
                    append("远端：${status.origin ?: "未配置"}\n")
                    append("分支：${status.branch ?: "未设置"}\n")
                    append("Deploy Key：${if (status.deployKeyExists) "已生成" else "未生成"}\n")
                    append(remoteLine)
                }
            )
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun showProjectWorkflowDialog(status: GitProjectStatus) {
        AlertDialog.Builder(activity)
            .setTitle(status.workflowTitle.ifBlank { "通用项目工作流" })
            .setMessage(projectWorkflowDialogText(status))
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun showConfigureGitDialog() {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(6), dp(18), 0)
        }
        val repoInput = EditText(activity).apply {
            hint = "git@github.com:owner/repo.git"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            setSingleLine(true)
        }
        val branchInput = EditText(activity).apply {
            hint = "main"
            setText("main")
            inputType = InputType.TYPE_CLASS_TEXT
            setSingleLine(true)
        }
        root.addView(TextView(activity).apply {
            text = "仓库地址"
            setTextColor(Color.parseColor("#444444"))
            textSize = 13f
        })
        root.addView(repoInput)
        root.addView(TextView(activity).apply {
            text = "分支"
            setTextColor(Color.parseColor("#444444"))
            textSize = 13f
        })
        root.addView(branchInput)

        val dialog = AlertDialog.Builder(activity)
            .setTitle("配置 GitHub 仓库")
            .setView(root)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val repo = repoInput.text.toString().trim()
                val branch = branchInput.text.toString().trim().ifBlank { "main" }
                if (repo.isBlank()) {
                    repoInput.error = "请输入 GitHub 仓库地址"
                    return@setOnClickListener
                }
                saveGitConfig(repo, branch)
                dialog.dismiss()
            }
        }
        dialog.show()
    }

    private fun showGitAuthHelpDialog() {
        AlertDialog.Builder(activity)
            .setTitle("GitHub 授权说明")
            .setMessage(
                "当前版本使用每项目 Deploy Key：先生成公钥，在 GitHub 仓库 Settings → Deploy keys 添加，并勾选写权限。\n\n" +
                    "正式多用户版会接入 GitHub App，用户只需要在 GitHub 授权指定仓库，服务器再用短期 token 读写代码。"
            )
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun loadGitProjectStatus(onLoaded: (GitProjectStatus) -> Unit) {
        Thread {
            try {
                val status = fetchProjectGitStatus(http, serverUrl, userId, projectProvider())
                activity.runOnUiThread { onLoaded(status) }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "Git 状态读取失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun generateDeployKey() {
        Thread {
            try {
                val (publicKey, status) = generateProjectDeployKey(http, serverUrl, userId, projectProvider())
                activity.runOnUiThread {
                    copyText("GitHub Deploy Key", publicKey)
                    AlertDialog.Builder(activity)
                        .setTitle("Deploy Key 已复制")
                        .setMessage(
                            "已复制公钥。请到 GitHub 仓库 Settings → Deploy keys 添加它，并勾选写权限。\n\n$publicKey"
                        )
                        .setPositiveButton("打开 GitHub") { _, _ -> openUrl(status.deployKeysUrl) }
                        .setNegativeButton("知道了", null)
                        .show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "Deploy Key 生成失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun saveGitConfig(repoUrl: String, branch: String) {
        Thread {
            try {
                val project = projectProvider()
                val status = saveProjectGitConfig(http, serverUrl, userId, project, repoUrl, branch)
                activity.runOnUiThread {
                    project.subtitle = if (status.remoteOk == true) {
                        "GitHub 仓库已连接"
                    } else {
                        "GitHub 仓库待授权"
                    }
                    addProjectEvent("Git 仓库配置：${summarize(repoUrl, 30)}")
                    showGitStatusDialog(status)
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "Git 配置失败: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density).toInt()
    }
}
