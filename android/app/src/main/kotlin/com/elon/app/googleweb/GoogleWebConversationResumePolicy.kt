package com.elon.app.googleweb

internal object GoogleWebConversationResumePolicy {
    fun persistableUrl(rawUrl: String?): String? =
        GoogleWebNavigationPolicy.sanitizeConversationUrl(rawUrl)

    fun startupUrl(persistedUrl: String?): String =
        persistableUrl(persistedUrl) ?: GoogleWebNavigationPolicy.START_URL

    fun reloadUrl(currentUrl: String?, persistedUrl: String?): String {
        val currentNavigation = GoogleWebNavigationPolicy.sanitizeNavigableUrl(currentUrl)
        if (currentNavigation != null) {
            return persistableUrl(currentNavigation) ?: GoogleWebNavigationPolicy.START_URL
        }
        return persistableUrl(persistedUrl) ?: GoogleWebNavigationPolicy.START_URL
    }

    fun officialUrl(activeUrl: String?, snapshotUrl: String?): String =
        persistableUrl(activeUrl)
            ?: persistableUrl(snapshotUrl)
            ?: GoogleWebNavigationPolicy.START_URL
}
