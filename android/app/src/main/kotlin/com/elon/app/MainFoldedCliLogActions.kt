package com.elon.app

internal class MainFoldedCliLogActions(
    private val currentStage: () -> String,
    private val updateStage: (String, String) -> Unit,
    private val maybeAppendVisibleCliSignal: (String, String) -> Boolean,
    private val recordEvidence: (String, String) -> Unit
) {
    private var foldedCliLogCount = 0
    private val foldedCliLogCategories = linkedMapOf<String, Int>()

    fun reset() {
        foldedCliLogCount = 0
        foldedCliLogCategories.clear()
    }

    fun handleFoldedCliOutput(content: String) {
        foldedCliLogCount += 1
        val line = cleanCliOutputLine(content)
        val category = cliOutputCategory(line)
        foldedCliLogCategories[category] = (foldedCliLogCategories[category] ?: 0) + 1

        val hint = when {
            category == "编译打包" -> "正在编译或检查 APK。"
            category == "执行命令" -> "正在检查项目文件。"
            category == "模型回复" -> "开发助手正在整理下一步。"
            category == "环境提示" -> "服务器环境有提示，技术细节已收起。"
            else -> "后台正在处理项目。"
        }
        val stage = when (category) {
            "编译打包" -> "编译打包"
            "环境提示" -> currentStage()
            else -> "开发实现"
        }
        updateStage(stage, hint)
        val surfaced = maybeAppendVisibleCliSignal(category, line)
        if (!surfaced && category != "模型回复") {
            recordEvidence(evidenceKindForCliCategory(category), line)
        }
    }

    fun summary(): String {
        val mainWork = foldedCliLogCategories.entries.maxByOrNull { it.value }?.key
        val friendly = when (mainWork) {
            "编译打包" -> "正在编译 APK"
            "执行命令" -> "正在检查项目文件"
            "环境提示" -> "环境提示已归类"
            "模型回复" -> "正在整理下一步"
            else -> "后台正在处理项目"
        }
        return "后台开发日志已收起（${foldedCliLogCount} 条） · $friendly"
    }
}
