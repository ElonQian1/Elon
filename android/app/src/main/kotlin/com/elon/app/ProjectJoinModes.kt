package com.elon.app

internal const val PROJECT_JOIN_MODE_OPEN = "open"
internal const val PROJECT_JOIN_MODE_APPROVAL = "approval"
internal const val PROJECT_JOIN_MODE_INVITE = "invite"
internal const val PROJECT_JOIN_MODE_READONLY = "readonly"

internal fun normalizeProjectJoinMode(mode: String?): String {
    val trimmed = mode?.trim()
    return when (trimmed) {
        PROJECT_JOIN_MODE_OPEN,
        PROJECT_JOIN_MODE_APPROVAL,
        PROJECT_JOIN_MODE_INVITE,
        PROJECT_JOIN_MODE_READONLY -> trimmed
        else -> PROJECT_JOIN_MODE_INVITE
    }
}

internal fun projectJoinModeSummary(mode: String): String = when (normalizeProjectJoinMode(mode)) {
    PROJECT_JOIN_MODE_OPEN -> "可直接加入"
    PROJECT_JOIN_MODE_APPROVAL -> "需审批"
    PROJECT_JOIN_MODE_READONLY -> "只读体验"
    PROJECT_JOIN_MODE_INVITE -> "仅邀请"
    else -> mode
}

internal fun projectJoinModeDetail(mode: String): String = when (normalizeProjectJoinMode(mode)) {
    PROJECT_JOIN_MODE_OPEN -> "公开（直接加入）"
    PROJECT_JOIN_MODE_APPROVAL -> "需管理员审批"
    PROJECT_JOIN_MODE_READONLY -> "公开只读（可进入、可问 AI、不能改代码）"
    PROJECT_JOIN_MODE_INVITE -> "仅邀请"
    else -> mode
}

internal fun projectJoinActionLabel(mode: String, alreadyJoined: Boolean = false): String {
    if (alreadyJoined) return "进入项目"
    return when (normalizeProjectJoinMode(mode)) {
        PROJECT_JOIN_MODE_OPEN -> "加入"
        PROJECT_JOIN_MODE_READONLY -> "进入体验"
        else -> "审批加入"
    }
}

internal fun projectJoinSuccessToast(mode: String): String {
    return when (normalizeProjectJoinMode(mode)) {
        PROJECT_JOIN_MODE_READONLY -> "已进入只读项目，可在频道里询问 AI"
        else -> "成功加入项目，点击按钮进入"
    }
}
