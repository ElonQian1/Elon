package com.elon.app

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.graphics.Color
import android.net.Uri
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import com.elon.uiruntime.compose.defaultComposeRuntimePreviewScenario
import com.elon.uiruntime.view.UiRuntimePreviewRegistry
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode
import java.time.LocalDate
import java.time.ZoneId

class UiRuntimeDebugPreviewProvider : ContentProvider() {
    override fun onCreate(): Boolean {
        UiRuntimePreviewRegistry.register(viewGalleryScenario())
        UiRuntimePreviewRegistry.register(socialSidebarScenario())
        UiRuntimePreviewRegistry.register(defaultComposeRuntimePreviewScenario())
        return true
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? = null

    override fun getType(uri: Uri): String? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0
    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = 0

    private fun viewGalleryScenario() = object : UiRuntimePreviewScenario {
        override val screenId = "elon.view.gallery"
        override val supportedScenarios = SCENARIOS

        override fun createView(context: Context, request: UiRuntimePreviewRequest): View =
            LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER
                setPadding(dp(context, 24), dp(context, 24), dp(context, 24), dp(context, 24))
                setBackgroundColor(if (request.theme == "dark") Color.rgb(18, 18, 18) else Color.WHITE)
                addView(TextView(context).apply {
                    text = "View Runtime · ${request.scenario}"
                    textSize = 22f
                    setTextColor(if (request.theme == "dark") Color.WHITE else Color.BLACK)
                }.uiNode("preview.view.title"))
                when (request.scenario) {
                    "loading" -> addView(ProgressBar(context).uiNode("preview.view.loading"))
                    "empty" -> addView(TextView(context).apply { text = "暂无内容" }.uiNode("preview.view.empty"))
                    "error" -> addView(TextView(context).apply {
                        text = "加载失败，请重试"
                        setTextColor(Color.rgb(180, 35, 35))
                    }.uiNode("preview.view.error"))
                    else -> addView(Button(context).apply { text = "主要操作" }.uiNode("preview.view.primary_action"))
                }
            }
    }

    private fun socialSidebarScenario() = object : UiRuntimePreviewScenario {
        override val screenId = "elon.social.sidebar"
        override val supportedScenarios = setOf("date", "favorites", "drag")

        override fun createView(context: Context, request: UiRuntimePreviewRequest): View {
            val previewMessages = socialSidebarPreviewMessages()
            return ChatSocialSideMenuView(
                context = context,
                timelineItems = {
                    previewMessages.mapIndexed { index, message ->
                        SocialSidebarTimelineItem(
                            key = SocialSidebarConversationKey(
                                if (index == 1) {
                                    SocialSidebarConversationType.GROUP
                                } else {
                                    SocialSidebarConversationType.FRIEND
                                },
                                "preview-$index"
                            ),
                            name = listOf("钱一龙", "产品体验群", "夜云")[index],
                            avatarDataUrl = null,
                            summary = previewTextForChatContent(message.content, message.attachments),
                            lastReceivedAt = message.createdAtMs,
                            unreadCount = listOf(6, 2, 12)[index],
                            message = message
                        )
                    }
                },
                favoriteItems = {
                    previewMessages.mapIndexed { index, message ->
                        SocialSidebarFavorite("favorite-$index", message.createdAtMs, message)
                    }
                },
                openConversation = { _ -> },
                loadTimelineMessage = { item, onDone ->
                    onDone(Result.success(item.message ?: previewMessages.first()))
                },
                openSettings = {},
                requestClose = { _ -> },
                dp = { value -> dp(context, value) },
                selectableForeground = { null },
                initialTab = if (request.scenario == "favorites") {
                    SocialSidebarTab.FAVORITES
                } else {
                    SocialSidebarTab.DATE
                }
            ).apply {
                render()
            }.uiNode("social.sidebar.root")
        }
    }

    private fun socialSidebarPreviewMessages(): List<ChatMessage> {
        val now = LocalDate.now()
            .atTime(13, 23)
            .atZone(ZoneId.systemDefault())
            .toInstant()
            .toEpochMilli()
        return listOf(
            ChatMessage(
                role = "friend",
                content = "今天看了有趣的灵魂发现很多事情，圣诞节佛爱上对方。",
                id = "preview-text",
                createdAtMs = now
            ),
            ChatMessage(
                role = "friend",
                content = "www.baidu.com",
                id = "preview-link",
                createdAtMs = now - 60_000L
            ),
            ChatMessage(
                role = "friend",
                content = "",
                attachments = listOf(
                    ChatAttachment(
                        kind = "video",
                        displayName = "侧栏拖拽演示.mp4",
                        mimeType = "video/mp4",
                        url = "https://example.invalid/sidebar-preview.mp4"
                    )
                ),
                id = "preview-video",
                createdAtMs = now - 120_000L
            )
        )
    }

    companion object {
        private val SCENARIOS = setOf("normal", "loading", "empty", "error")
        private fun dp(context: Context, value: Int): Int =
            (value * context.resources.displayMetrics.density).toInt()
    }
}
