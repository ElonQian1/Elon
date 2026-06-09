package com.elon.app

internal fun projectRoleLabel(role: String?): String = when (role) {
    "owner" -> "所有者"
    "admin" -> "管理员"
    "editor" -> "协作者"
    "member" -> "成员"
    "observer", "viewer" -> "只读成员"
    else -> role?.takeIf { it.isNotBlank() } ?: "成员"
}

internal fun canManageProjectMembers(role: String?): Boolean {
    return role == "owner" || role == "admin"
}

internal fun canResolveProjectSuggestion(role: String?): Boolean {
    return role == "owner" || role == "admin" || role == "editor" || role == "member"
}
