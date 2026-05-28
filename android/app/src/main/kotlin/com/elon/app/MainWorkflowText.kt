package com.elon.app

internal fun progressStepLabel(content: String): String {
    return when {
        content.startsWith("AI 回复片段") -> "开发说明"
        content.startsWith("CLI 工作区") -> "准备项目"
        content.startsWith("项目环境已准备好") -> "准备项目"
        content.startsWith("环境提醒") -> "环境检查"
        content.startsWith("正在启动本地 CLI") -> "启动助手"
        content.startsWith("开发助手已启动") -> "启动助手"
        content.startsWith("开发助手仍在运行") -> "处理中"
        content.startsWith("CLI 输出") -> "后台处理"
        content.startsWith("后台正在处理") -> "后台处理"
        content.startsWith("CLI 已结束") -> "检查结果"
        content.startsWith("开发处理已结束") -> "检查结果"
        content.startsWith("未找到 APK") -> "产物缺失"
        content.contains("下载链接") -> "生成下载"
        content.contains("APK") || content.contains("编译") -> "编译打包"
        else -> "进度更新"
    }
}

internal fun initialWorkflowMessage(isDevelopment: Boolean): String {
    return if (isDevelopment) "正在按这个计划推进。" else "正在整理回复。"
}

internal fun workflowProgressMessage(content: String): String {
    val progress = userFacingProgress(content.ifBlank { "正在推进当前任务。" })
    if (progress == "正在思考") return progress
    return "${progressStepLabel(progress)}：$progress"
}

internal fun shouldShowProgressBubble(content: String): Boolean {
    val progress = userFacingProgress(content)
    return !isRoutineWorkflowMessage(workflowProgressMessage(content)) &&
        !isRoutineWorkflowMessage(progress) &&
        (content.startsWith("环境提醒") ||
            content.contains("已识别为开发任务") ||
            content.contains("正在确认这是否需要进入开发流程") ||
            content.startsWith("正在准备项目工作区") ||
            content.contains("AI 助手正在处理") ||
            content.contains("通用项目工作流") ||
            content.contains("已轮到本会话任务") ||
            content.contains("已获得本会话执行权") ||
            content.contains("已获得项目执行权") ||
            content.contains("进入队列") ||
            content.contains("排队") ||
            content.startsWith("未找到 APK") ||
            content.contains("失败") ||
            content.contains("错误") ||
            content.contains("不可用"))
}

internal fun userFacingProgress(content: String): String {
    extractUserVisibleCliMessage(content)?.let { return it }
    return when {
        content.startsWith("AI 回复片段") ->
            "开发助手正在给出阶段说明。"
        content.contains("已识别为开发任务") ->
            "已确认这是开发任务，开始进入项目流程。"
        content.contains("正在确认这是否需要进入开发流程") ->
            "我正在确认这条消息是否需要改代码。"
        content.startsWith("正在准备项目工作区") ->
            "正在准备项目环境。"
        content.contains("AI 助手正在处理") ->
            "开发助手正在处理你的需求。"
        content.contains("通用项目工作流") ->
            "开发流程已启用，我会按需求确认、代码修改、验证和交付来推进。"
        content.contains("当前会话已有任务") ->
            "当前会话已有任务在处理，这条需求已排队。"
        content.contains("已轮到本会话任务") ||
            content.contains("已获得本会话执行权") ||
            content.contains("已获得项目执行权") ->
            "轮到本轮任务了，正在让开发助手处理项目。"
        content.startsWith("CLI 工作区") ->
            "项目环境已准备好，正在进入开发流程。"
        content.startsWith("正在启动本地 CLI") ->
            "开发助手已启动，正在处理你的需求。"
        content.startsWith("CLI 仍在运行") ->
            "正在思考"
        content.startsWith("CLI 已结束") ->
            "开发处理已结束，正在检查 APK 文件。"
        content.startsWith("未找到 APK") ->
            "暂时没有找到 APK 文件，正在判断是否需要继续处理。"
        content.startsWith("环境提醒") && content.contains("Codex CLI") ->
            "服务器开发助手配置需要检查，可能会影响本次开发。"
        content.startsWith("环境提醒") && content.contains("Android SDK") ->
            "服务器 Android 构建环境需要检查，可能会影响打包 APK。"
        content.startsWith("环境提醒") && content.contains("java", ignoreCase = true) ->
            "服务器 Java 环境需要检查，可能会影响打包 APK。"
        content.startsWith("CLI 输出") ->
            "后台正在处理项目，技术日志已收起。"
        else -> content
    }
}

internal fun finalReplyMessage(content: String, apkUrl: String?, imageUrl: String?, wasDevelopment: Boolean): String {
    val cleanAsDevelopment = shouldCleanFinalAsDevelopment(content, wasDevelopment, apkUrl)
    val main = cleanFinalReplyForUser(content, cleanAsDevelopment, apkUrl).ifBlank {
        if (cleanAsDevelopment) "本轮开发任务已完成。" else "回复已完成。"
    }
    return buildString {
        append(main)
        imageUrl?.takeIf { !main.contains(it) }?.let { append("\n\n图片链接：$it") }
    }
}

internal fun mainWorkflowStoppedMessage(reason: String, wasDevelopment: Boolean): String {
    val stage = if (wasDevelopment) "需要处理" else "回复中断"
    return "工作停止：$stage。原因：$reason"
}

internal fun nextWorkflowHint(stage: String): String {
    return when (stage) {
        "需求分析" -> "定位相关文件。"
        "开发实现" -> "继续修改并检查结果。"
        "编译打包" -> "等待编译结果。"
        "交付完成" -> "整理最终结果。"
        "需要处理" -> "根据错误判断是否可修复。"
        else -> "等待下一步结果。"
    }
}

internal fun stageLine(index: Int, active: Int, label: String): String {
    val state = when {
        active == -1 -> if (index == 1) "需处理" else "等待"
        active > index -> "已完成"
        active == index -> "进行中"
        else -> "等待"
    }
    return "$index. $label：$state"
}

internal fun toolLabel(tool: String): String = when (tool) {
    "init_project" -> "初始化项目"
    "read_file" -> "读取文件"
    "write_file" -> "写入代码"
    "list_dir" -> "查看目录"
    "run_shell" -> "执行命令"
    "shell" -> "执行命令"
    "file_change" -> "修改文件"
    "git_commit" -> "保存版本"
    "build_project" -> "编译项目"
    else -> tool
}
