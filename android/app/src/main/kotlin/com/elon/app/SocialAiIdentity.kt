package com.elon.app

internal object SocialAiIdentity {
    const val USER_ID = "usr_elon_ai"
    const val DISPLAY_NAME = "一龙AI"
    const val ACCOUNT = "ai-agent"

    fun matches(userId: String?): Boolean = userId == USER_ID

    fun resolve(friends: List<AppFriend>): AppFriend =
        friends.firstOrNull { matches(it.id) } ?: builtInFriend()

    fun builtInFriend(): AppFriend = AppFriend(
        id = USER_ID,
        name = DISPLAY_NAME,
        account = ACCOUNT,
        phone = null,
        avatarDataUrl = null,
        friendSince = null,
        lastMessage = null,
        lastMessageAt = null,
        unreadCount = 0,
        isOnline = true,
    )
}

internal fun AppFriend?.isSocialAi(): Boolean = SocialAiIdentity.matches(this?.id)
