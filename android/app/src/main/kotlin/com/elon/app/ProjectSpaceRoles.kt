package com.elon.app

import java.util.Locale

internal fun projectRoleLabel(role: String?): String = when (role.normalizedProjectRole()) {
    "owner" -> "所有者"
    "admin" -> "管理员"
    "editor" -> "协作者"
    "member" -> "成员"
    "observer", "viewer" -> "只读成员"
    "visitor" -> "访客"
    else -> role?.takeIf { it.isNotBlank() } ?: "成员"
}

internal fun isProjectSpaceVisitor(role: String?): Boolean {
    return role.normalizedProjectRole() == "visitor"
}

internal fun canManageProjectMembers(role: String?): Boolean {
    return role.normalizedProjectRole() in setOf("owner", "admin")
}

internal fun canResolveProjectSuggestion(role: String?): Boolean {
    return role.normalizedProjectRole() in setOf("owner", "admin", "editor", "member")
}

internal fun canEditProjectAnnouncement(role: String?): Boolean {
    return role.normalizedProjectRole() in setOf("owner", "creator")
}

internal fun canEditProjectDescription(role: String?): Boolean {
    return role.normalizedProjectRole() in setOf("owner", "admin", "editor")
}

private fun String?.normalizedProjectRole(): String? {
    return this?.trim()?.lowercase(Locale.ROOT)?.takeIf { it.isNotBlank() }
}
