package com.elon.app

import org.json.JSONObject

/**
 * 全局 WS 事件类型。
 *
 * 服务器通过 /ws/app 推送的所有消息均有 `type` 字段，
 * GlobalWsManager 解析后以此类型分发给各监听者。
 *
 * 当前支持：
 *   - [AppUpdateAvailable]  有新版本 APK
 *
 * 预留扩展（服务端功能上线后 Android 端自动生效）：
 *   - [FriendMessage]       好友消息
 */
sealed class GlobalWsEvent {

    /** 有新版本 APK 可安装 */
    data class AppUpdateAvailable(
        val versionCode: Int,
        val versionName: String,
        val downloadUrl: String,
        val changelog: String,
        val forceUpdate: Boolean,
    ) : GlobalWsEvent()

    /** 好友消息（预留，服务端支持后启用） */
    data class FriendMessage(
        val fromUserId: String,
        val toUserId: String,
        val messageId: String,
        val content: String,
        val createdAt: String,
    ) : GlobalWsEvent()

    /** 群聊消息 */
    data class GroupMessage(
        val groupId: String,
        val fromUserId: String,
        val messageId: String,
        val content: String,
        val createdAt: String,
    ) : GlobalWsEvent()

    /** 好友在线状态变更：isOnline=true 表示上线，false 表示下线 */
    data class PresenceChange(
        val userId: String,
        val isOnline: Boolean,
    ) : GlobalWsEvent()

    /** 好友正在输入提示 */
    data class Typing(
        val fromUserId: String,
    ) : GlobalWsEvent()

    /** 无法识别的事件类型，原始 JSON 保留供调试 */
    data class Unknown(val raw: String) : GlobalWsEvent()

    companion object {
        fun parse(text: String): GlobalWsEvent = try {
            val json = JSONObject(text)
            when (json.optString("type")) {
                "app_update_available" -> AppUpdateAvailable(
                    versionCode = json.optInt("versionCode", 0),
                    versionName = json.optString("versionName", ""),
                    downloadUrl = json.optString("downloadUrl", ""),
                    changelog = json.optString("changelog", ""),
                    forceUpdate = json.optBoolean("forceUpdate", false),
                )
                "friend_message" -> FriendMessage(
                    fromUserId = json.optString("fromUserId", ""),
                    toUserId = json.optString("toUserId", ""),
                    messageId = json.optString("messageId", ""),
                    content = json.optString("content", ""),
                    createdAt = json.optString("createdAt", ""),
                )
                "group_message" -> GroupMessage(
                    groupId = json.optString("groupId", ""),
                    fromUserId = json.optString("fromUserId", ""),
                    messageId = json.optString("messageId", ""),
                    content = json.optString("content", ""),
                    createdAt = json.optString("createdAt", ""),
                )
                "presence" -> PresenceChange(
                    userId = json.optString("userId", ""),
                    isOnline = json.optBoolean("isOnline", false),
                )
                "typing" -> Typing(
                    fromUserId = json.optString("fromUserId", ""),
                )
                else -> Unknown(text)
            }
        } catch (_: Exception) {
            Unknown(text)
        }
    }
}
