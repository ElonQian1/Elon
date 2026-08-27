package com.elon.app

import java.util.Calendar

internal enum class WorkSummarySection { ATTENTION, PROGRESS, CONFIRM }

internal data class GeneratedWorkSummaryItem(
    val projectId: String,
    val project: String,
    val title: String,
    val reason: String,
    val suggestion: String,
    val primaryAction: String,
    val secondaryAction: String,
    val section: WorkSummarySection,
    val updatedAt: Long,
    val highPriority: Boolean = false,
    val highlightPrimary: Boolean = true,
)

internal data class GeneratedWorkSummary(
    val projectCount: Int,
    val attention: List<GeneratedWorkSummaryItem>,
    val progress: List<GeneratedWorkSummaryItem>,
    val confirm: List<GeneratedWorkSummaryItem>,
)

internal fun generateWorkSummary(projects: List<AppProject>, dayMillis: Long): GeneratedWorkSummary {
    val day = Calendar.getInstance().apply { timeInMillis = dayMillis }
    val start = (day.clone() as Calendar).apply {
        set(Calendar.HOUR_OF_DAY, 0); set(Calendar.MINUTE, 0); set(Calendar.SECOND, 0); set(Calendar.MILLISECOND, 0)
    }.timeInMillis
    val end = start + 24 * 60 * 60 * 1000L
    val realProjects = projects.filterNot { it.isSystemArchiveProject() }
    val items = realProjects.mapNotNull { project -> summarizeProject(project, start, end) }
        .sortedWith(compareByDescending<GeneratedWorkSummaryItem> { it.highPriority }.thenByDescending { it.updatedAt })
    return GeneratedWorkSummary(
        projectCount = realProjects.size,
        attention = items.filter { it.section == WorkSummarySection.ATTENTION },
        progress = items.filter { it.section == WorkSummarySection.PROGRESS },
        confirm = items.filter { it.section == WorkSummarySection.CONFIRM },
    )
}

private fun summarizeProject(project: AppProject, start: Long, end: Long): GeneratedWorkSummaryItem? {
    if (project.updatedAt !in start until end) return null
    val status = project.stage.trim().ifBlank { "待提交需求" }
    val normalized = status.lowercase()
    val health = project.workspaceHealthLabel?.trim().orEmpty()
    val unhealthy = project.workspaceHealthTone in setOf("bad", "warn") ||
        listOf("失败", "错误", "异常", "阻塞", "离线", "冲突").any { status.contains(it) || health.contains(it) }
    val needsConfirm = listOf("待确认", "等待确认", "待验收", "等待验收", "待测试", "等待测试", "待发布", "等待发布").any(status::contains)
    val completed = listOf("完成", "成功", "已发布", "已部署", "已合并", "verified", "released", "completed", "success").any(normalized::contains)
    val section = when {
        unhealthy -> WorkSummarySection.ATTENTION
        needsConfirm -> WorkSummarySection.CONFIRM
        completed -> WorkSummarySection.PROGRESS
        else -> WorkSummarySection.ATTENTION
    }
    val title = when (section) {
        WorkSummarySection.ATTENTION -> if (unhealthy) "项目状态需要处理" else "项目有待推进事项"
        WorkSummarySection.PROGRESS -> "项目取得新进展"
        WorkSummarySection.CONFIRM -> "项目等待你的确认"
    }
    val reason = listOf(status, health.takeIf { it.isNotBlank() && it != status }).filterNotNull().joinToString(" · ")
    val suggestion = when (section) {
        WorkSummarySection.ATTENTION -> project.workspaceHealthLabel?.takeIf { it.isNotBlank() }?.let { "检查工作区状态并处理：$it" }
            ?: "进入项目查看最新任务并继续处理"
        WorkSummarySection.PROGRESS -> "查看本次进展和交付结果"
        WorkSummarySection.CONFIRM -> "核对结果后确认下一步"
    }
    return GeneratedWorkSummaryItem(
        projectId = project.id,
        project = project.title,
        title = title,
        reason = reason,
        suggestion = suggestion,
        primaryAction = if (section == WorkSummarySection.PROGRESS) "查看项目" else "交给 AI 处理",
        secondaryAction = "进入项目",
        section = section,
        updatedAt = project.updatedAt,
        highPriority = unhealthy,
        highlightPrimary = section != WorkSummarySection.PROGRESS,
    )
}
