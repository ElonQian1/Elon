package com.elon.app

import android.app.Application
import android.content.Context
import com.elon.app.mcp.*

class ElonApplication : Application() {

    /** 全局 WS 管理器，应用进程内保持连接以接收聊天和更新提醒。 */
    val globalWs: GlobalWsManager by lazy { GlobalWsManager(SERVER_URL) }

    private val chatNotificationListener = object : GlobalWsManager.Listener {
        override fun onGlobalWsEvent(event: GlobalWsEvent) {
            when (event) {
                is GlobalWsEvent.FriendMessage ->
                    ChatMessageNotifications.showFriendMessage(
                        context = this@ElonApplication,
                        fromUserId = event.fromUserId,
                        messageId = event.messageId,
                        content = event.content,
                        senderName = event.senderName,
                        createdAt = event.createdAt
                    )
                is GlobalWsEvent.GroupMessage ->
                    ChatMessageNotifications.showGroupMessage(
                        context = this@ElonApplication,
                        groupId = event.groupId,
                        fromUserId = event.fromUserId,
                        messageId = event.messageId,
                        content = event.content,
                        senderName = event.senderName,
                        groupName = event.groupName,
                        createdAt = event.createdAt
                    )
                else -> Unit
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        ChatMessageNotifications.createChannel(this)
        globalWs.addListener(chatNotificationListener)
        globalWs.start(this)
        DebugTraceStore.init(this)
        DebugTraceStore.record(
            "app_start",
            mapOf(
                "version_name" to BuildConfig.VERSION_NAME,
                "version_code" to BuildConfig.VERSION_CODE
            )
        )
        McpDebugServer.start(this)
    }

    companion object {
        /** 主服务器 URL（与 BuildConfig.SERVER_URL 保持一致，供静态引用）。动态 URL 请用 [activeServerUrl]。 */
        val SERVER_URL: String = BuildConfig.SERVER_URL

        /** 返回当前活跃服务器 URL（主服务器或备用服务器）。 */
        @JvmStatic
        fun activeServerUrl(context: Context): String = ServerUrlManager.getActive(context)
    }
}
