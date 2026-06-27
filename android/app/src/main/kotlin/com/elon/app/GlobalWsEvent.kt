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
 *   - [ProjectTaskDone]     项目会话完成
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
        val senderName: String?,
    ) : GlobalWsEvent()

    /** 群聊消息 */
    data class GroupMessage(
        val groupId: String,
        val fromUserId: String,
        val messageId: String,
        val content: String,
        val createdAt: String,
        val senderName: String?,
        val groupName: String?,
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

    /** 消息已读回执：fromUserId 已读取至 lastReadAt 时间戳的消息 */
    data class ReadReceipt(
        val fromUserId: String,
        val lastReadAt: String,
    ) : GlobalWsEvent()

    /** 项目 AI 会话已完成，可用于跨 PC/手机提醒用户查看结果 */
    data class ProjectTaskDone(
        val projectId: String,
        val triggeredByUserId: String,
        val conversationId: String,
        val message: String,
        val apkUrl: String?,
    ) : GlobalWsEvent()

    /** 无法识别的事件类型，原始 JSON 保留供调试 */
    data class Unknown(val raw: String) : GlobalWsEvent()

    companion object {
        fun parse(text: String): GlobalWsEvent = try {
            val json = JSONObject(text)
            when (json.optString("type")) {
                "app_update_available" -> AppUpdateAvailable(
                    versionCode = json.intAny("versionCode", "version_code"),
                    versionName = json.stringAny("versionName", "version_name"),
                    downloadUrl = json.stringAny("downloadUrl", "download_url"),
                    changelog = json.optString("changelog", ""),
                    forceUpdate = json.booleanAny("forceUpdate", "force_update"),
                )
                "friend_message" -> FriendMessage(
                    fromUserId = json.stringAny("fromUserId", "from_user_id", "senderUserId", "sender_user_id"),
                    toUserId = json.stringAny("toUserId", "to_user_id", "receiverUserId", "receiver_user_id"),
                    messageId = json.stringAny("messageId", "message_id", "id"),
                    content = json.stringAny("content", "message", "text"),
                    createdAt = json.stringAny("createdAt", "created_at"),
                    senderName = json.stringAnyOrNull("senderName", "sender_name", "nickname"),
                )
                "group_message" -> GroupMessage(
                    groupId = json.stringAny("groupId", "group_id"),
                    fromUserId = json.stringAny("fromUserId", "from_user_id", "senderUserId", "sender_user_id"),
                    messageId = json.stringAny("messageId", "message_id", "id"),
                    content = json.stringAny("content", "message", "text"),
                    createdAt = json.stringAny("createdAt", "created_at"),
                    senderName = json.stringAnyOrNull("senderName", "sender_name", "nickname"),
                    groupName = json.stringAnyOrNull("groupName", "group_name", "name"),
                )
                "presence" -> PresenceChange(
                    userId = json.stringAny("userId", "user_id"),
                    isOnline = json.optBoolean("isOnline", false),
                )
                "typing" -> Typing(
                    fromUserId = json.stringAny("fromUserId", "from_user_id", "senderUserId", "sender_user_id"),
                )
                "read_receipt" -> ReadReceipt(
                    fromUserId = json.stringAny("fromUserId", "from_user_id"),
                    lastReadAt = json.stringAny("lastReadAt", "last_read_at"),
                )
                "project_task_done" -> ProjectTaskDone(
                    projectId = json.stringAny("projectId", "project_id"),
                    triggeredByUserId = json.stringAny("triggeredByUserId", "triggered_by_user_id", "userId", "user_id"),
                    conversationId = json.stringAny("conversationId", "conversation_id"),
                    message = json.optString("message", "项目任务已完成"),
                    apkUrl = json.stringAnyOrNull("apkUrl", "apk_url"),
                )
                else -> Unknown(text)
            }
        } catch (_: Exception) {
            Unknown(text)
        }

        private fun JSONObject.stringAny(vararg names: String): String {
            for (name in names) {
                val value = optString(name, "").trim()
                if (value.isNotEmpty()) return value
            }
            return ""
        }

        private fun JSONObject.stringAnyOrNull(vararg names: String): String? {
            return stringAny(*names).takeIf { it.isNotBlank() }
        }

        private fun JSONObject.intAny(vararg names: String): Int {
            for (name in names) {
                if (!has(name)) continue
                val value = opt(name)
                if (value is Number) return value.toInt()
                val parsed = optString(name, "").toIntOrNull()
                if (parsed != null) return parsed
            }
            return 0
        }

        private fun JSONObject.booleanAny(vararg names: String): Boolean {
            for (name in names) {
                if (!has(name)) continue
                val value = opt(name)
                if (value is Boolean) return value
                val text = optString(name, "").trim()
                if (text.equals("true", ignoreCase = true)) return true
                if (text.equals("false", ignoreCase = true)) return false
            }
            return false
        }
    }
}
