package com.elon.app

internal object MainSocialAiChatNavigationPolicy {
    fun refreshIndex(
        active: Boolean,
        providerId: WebChatProviderId,
        projectId: String?,
        conversationPath: String?,
        probeChatGptProject: (String, String) -> Boolean,
        refreshDirectory: (String?) -> Boolean,
    ): Boolean {
        if (!active) return false
        val requestedPath = conversationPath?.trim()?.takeIf(String::isNotEmpty)
        val requestedProjectId = projectId?.trim()?.takeIf(String::isNotEmpty)
        if (requestedPath == null) return refreshDirectory(requestedProjectId)
        if (requestedProjectId == null || providerId != WebChatProviderId.CHATGPT_WEB) return false
        return probeChatGptProject(requestedPath, requestedProjectId)
    }
}

internal class WebChatProjectMoveRecoveryGate {
    private var checked = false

    fun observe(providerId: WebChatProviderId, state: String, recover: () -> Unit) {
        if (checked || providerId != WebChatProviderId.CHATGPT_WEB || state != "ready") return
        checked = true
        recover()
    }
}
