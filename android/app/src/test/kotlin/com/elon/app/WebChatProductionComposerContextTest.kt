package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatProductionComposerContextTest {
    @Test
    fun resolvesProjectFromTheCurrentConversation() {
        val index = ChatGptWebConversationIndexState(
            conversations = listOf(
                conversation(
                    path = "/g/g-p-family/c/health-chat",
                    projectId = "g-p-family",
                    projectTitle = "家庭成员健康",
                ),
            ),
        )

        val title = WebChatProductionComposerContext.projectTitle(
            index,
            "/g/g-p-family/c/health-chat",
        )

        assertEquals("家庭成员健康", title)
        assertEquals(
            "家庭成员健康中的新聊天",
            WebChatProductionComposerContext.inputHint("输入内容", title),
        )
    }

    @Test
    fun resolvesAnActiveProjectBeforeItsNewConversationHasAPath() {
        val index = ChatGptWebConversationIndexState(
            projects = listOf(
                ChatGptWebProject("g-p-family", "家庭成员健康", "/g/g-p-family/project", active = true),
            ),
        )

        assertEquals("家庭成员健康", WebChatProductionComposerContext.projectTitle(index, null))
    }

    @Test
    fun preservesRecoveryHintsAndIgnoresMissingProjectMetadata() {
        assertNull(WebChatProductionComposerContext.projectTitle(ChatGptWebConversationIndexState(), null))
        assertEquals(
            "网页连接异常，输入内容将保留",
            WebChatProductionComposerContext.inputHint(
                "网页连接异常，输入内容将保留",
                "家庭成员健康",
            ),
        )
    }

    @Test
    fun doesNotLeakAnActiveSidebarProjectIntoAnUnprojectedConversation() {
        val index = ChatGptWebConversationIndexState(
            conversations = listOf(
                ChatGptWebConversation(
                    id = "plain-chat",
                    title = "普通聊天",
                    path = "/c/plain-chat",
                    active = true,
                ),
            ),
            projects = listOf(
                ChatGptWebProject("g-p-family", "家庭成员健康", "/g/g-p-family/project", active = true),
            ),
        )

        assertNull(WebChatProductionComposerContext.projectTitle(index, "/c/plain-chat"))
    }

    private fun conversation(
        path: String,
        projectId: String,
        projectTitle: String,
    ) = ChatGptWebConversation(
        id = path.substringAfterLast('/'),
        title = "健康咨询",
        path = path,
        active = true,
        projectId = projectId,
        projectTitle = projectTitle,
        projectPath = "/g/$projectId/project",
    )
}
