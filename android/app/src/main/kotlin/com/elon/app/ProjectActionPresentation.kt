package com.elon.app

/** Presentation of project membership, independent of the action dispatcher. */
internal fun projectActionStatusText(project: AppProject, isJoint: Boolean): String {
    if (project.isSystemArchiveProject()) {
        return "${project.systemArchiveDisplayName()} · 专属会话归档"
    }
    if (!isJoint) return "个人项目"
    return when (normalizeProjectJoinMode(project.collaborationJoinMode)) {
        "open" -> "联合项目 · 商城公开"
        "readonly" -> "联合项目 · 广场只读"
        "approval" -> "联合项目 · 加入需审批"
        else -> "联合项目 · 邀请协作"
    }
}
