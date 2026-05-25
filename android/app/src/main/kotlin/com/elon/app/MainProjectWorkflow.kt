package com.elon.app

import org.json.JSONObject

internal fun projectWorkflowDialogText(status: GitProjectStatus): String {
    val steps = status.workflowSteps.ifEmpty { defaultWorkflowSteps() }
    return buildString {
        append(status.workflowSummary.ifBlank { defaultWorkflowSummary() })
        append("\n\n")
        steps.forEachIndexed { index, step ->
            append(index + 1)
            append(". ")
            append(step)
            append('\n')
        }
        val memory = status.codexMemory.ifBlank { defaultCodexMemory() }
        append("\nCodex CLI 规则\n")
        append(memory)
    }.trim()
}

internal fun defaultWorkflowSummary(): String {
    return "所有项目都走同一套流程：先确认 Git/权限，再读取项目文档，然后修改、验证、提交和推送；合并、版本号和发布由服务器串行保护。"
}

internal fun defaultWorkflowSteps(): List<String> {
    return listOf(
        "项目准备：确认项目路径、Git 仓库、远端和写权限。",
        "读取文档：优先读取 AGENTS.md、CODEX.md、README.md、.github/instructions 和任务相关 docs。",
        "执行任务：按项目自己的技术栈修改代码，不把一龙自项目规则套到普通项目。",
        "验证保存：运行必要检查，commit；有可用远端时 push。",
        "共享动作：合并 main、版本号递增、APK 发布、服务器部署必须串行。"
    )
}

internal fun defaultCodexMemory(): String {
    return "Codex CLI 不依赖长期记忆；服务器每次任务都会在提示词中注入通用流程，同时要求它读取当前项目仓库内的说明文档。"
}

internal fun projectWorkflowCardText(currentStage: String): String {
    return buildString {
        append("通用项目工作流\n")
        defaultWorkflowSteps().forEachIndexed { index, step ->
            append(index + 1)
            append(". ")
            append(step.substringBefore('：'))
            append('\n')
        }
        append("\n当前阶段：")
        append(currentStage)
        append("\nCodex 每次任务都会重新读取项目文档，不靠记忆猜。")
    }.trim()
}

internal fun parseGitProjectStatus(json: JSONObject): GitProjectStatus {
    val git = json.optJSONObject("git") ?: JSONObject()
    val deployKey = json.optJSONObject("deploy_key") ?: JSONObject()
    val remoteCheck = git.optJSONObject("remote_check")
    val workflow = json.optJSONObject("workflow") ?: JSONObject()
    val workflowStepsJson = workflow.optJSONArray("steps")
    val workflowSteps = mutableListOf<String>()
    if (workflowStepsJson != null) {
        for (index in 0 until workflowStepsJson.length()) {
            workflowStepsJson.optString(index).takeIf { it.isNotBlank() }?.let {
                workflowSteps.add(it)
            }
        }
    }
    return GitProjectStatus(
        hasGit = git.optBoolean("has_git", false),
        origin = git.optString("origin", "").takeIf { it.isNotBlank() && it != "null" },
        branch = git.optString("branch", "").takeIf { it.isNotBlank() && it != "null" },
        remoteOk = remoteCheck?.optBoolean("ok"),
        remoteMessage = remoteCheck?.optString("message"),
        deployKeyExists = deployKey.optBoolean("exists", false),
        publicKey = deployKey.optString("public_key", "").takeIf { it.isNotBlank() && it != "null" },
        deployKeysUrl = deployKey.optString("github_deploy_keys_url", "https://github.com/settings/keys"),
        workflowTitle = workflow.optString("title", "通用项目工作流"),
        workflowSummary = workflow.optString("summary", defaultWorkflowSummary()),
        workflowSteps = workflowSteps,
        codexMemory = workflow.optString("codex_memory", defaultCodexMemory())
    )
}
