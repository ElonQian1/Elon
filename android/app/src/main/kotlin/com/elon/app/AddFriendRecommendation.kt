package com.elon.app

import org.json.JSONObject

internal data class AddFriendRecommendation(
    val id: String,
    val name: String,
    val account: String,
    val phone: String?,
    val avatarDataUrl: String?,
    val mutualFriendCount: Int,
    val alreadyFriend: Boolean,
    val isSelf: Boolean = false
)

internal fun parseRecommendation(json: JSONObject): AddFriendRecommendation {
    val id = json.optString("id", "").trim()
    val account = json.optString("account_hint", "").trim()
        .ifBlank { json.optString("account", "").trim().ifBlank { id } }
    val phone = json.optString("phone", "").trim().takeIf { it.isNotEmpty() }
    val nickname = json.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
    return AddFriendRecommendation(
        id = id,
        name = nickname ?: account.ifBlank { "朋友名称" },
        account = account,
        phone = phone,
        avatarDataUrl = json.optString("avatar_data_url", "").trim().takeIf { it.isNotEmpty() },
        mutualFriendCount = json.optInt("mutual_friend_count", 0).coerceAtLeast(0),
        alreadyFriend = json.optBoolean("already_friend", false),
        isSelf = json.optBoolean("is_self", false)
    )
}
