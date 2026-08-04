package com.elon.app

internal enum class ProjectPlazaPrimaryActionKind {
    OPEN,
    JOIN,
    REQUEST_JOIN
}

internal data class ProjectPlazaPrimaryAction(
    val kind: ProjectPlazaPrimaryActionKind,
    val label: String,
    val enabled: Boolean = true
)

internal enum class ProjectPlazaTone {
    SUCCESS,
    DANGER,
    NEUTRAL
}

internal data class ProjectPlazaStatus(
    val label: String,
    val tone: ProjectPlazaTone
)

internal fun projectPlazaPrimaryAction(
    project: StoreProject,
    joined: Boolean,
    requestPending: Boolean = false,
    busy: Boolean = false
): ProjectPlazaPrimaryAction {
    val action = when {
        joined -> ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "进入空间")
        requestPending -> ProjectPlazaPrimaryAction(
            ProjectPlazaPrimaryActionKind.REQUEST_JOIN,
            "申请已提交",
            enabled = false
        )
        normalizeProjectJoinMode(project.joinMode) == PROJECT_JOIN_MODE_APPROVAL -> {
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.REQUEST_JOIN, "申请加入")
        }
        normalizeProjectJoinMode(project.joinMode) == PROJECT_JOIN_MODE_OPEN -> {
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.JOIN, "加入项目")
        }
        normalizeProjectJoinMode(project.joinMode) == PROJECT_JOIN_MODE_READONLY -> {
            ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "进入体验")
        }
        else -> ProjectPlazaPrimaryAction(ProjectPlazaPrimaryActionKind.OPEN, "查看项目")
    }
    return if (busy) action.copy(label = "处理中…", enabled = false) else action
}

internal fun projectPlazaAccessStatus(project: StoreProject, joined: Boolean): ProjectPlazaStatus {
    if (joined) return ProjectPlazaStatus("已加入", ProjectPlazaTone.SUCCESS)
    return when (normalizeProjectJoinMode(project.joinMode)) {
        PROJECT_JOIN_MODE_APPROVAL -> ProjectPlazaStatus("需审批", ProjectPlazaTone.DANGER)
        PROJECT_JOIN_MODE_INVITE -> ProjectPlazaStatus("仅限邀请", ProjectPlazaTone.NEUTRAL)
        PROJECT_JOIN_MODE_READONLY -> ProjectPlazaStatus("只读体验", ProjectPlazaTone.NEUTRAL)
        else -> if (!project.latestApkUrl.isNullOrBlank()) {
            ProjectPlazaStatus("可安装", ProjectPlazaTone.SUCCESS)
        } else {
            ProjectPlazaStatus("无需审批", ProjectPlazaTone.SUCCESS)
        }
    }
}

internal fun projectPlazaBuildStatus(rawStatus: String?): ProjectPlazaStatus {
    val normalized = rawStatus?.trim()?.lowercase()?.replace('-', '_').orEmpty()
    return when (normalized) {
        "success", "succeeded", "completed", "complete", "passed", "ready", "done" -> {
            ProjectPlazaStatus("构建成功", ProjectPlazaTone.SUCCESS)
        }
        "failed", "failure", "error", "cancelled", "canceled", "blocked" -> {
            ProjectPlazaStatus("构建异常", ProjectPlazaTone.DANGER)
        }
        "running", "building", "pending", "queued", "in_progress", "processing", "working" -> {
            ProjectPlazaStatus("构建中", ProjectPlazaTone.NEUTRAL)
        }
        else -> ProjectPlazaStatus("暂无构建", ProjectPlazaTone.NEUTRAL)
    }
}
