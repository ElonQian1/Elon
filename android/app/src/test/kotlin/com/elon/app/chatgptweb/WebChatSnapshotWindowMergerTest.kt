package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test
import com.elon.app.WebChatTextBlock

class WebChatSnapshotWindowMergerTest {
    @Test
    fun conversationOwnershipIsDerivedFromValidatedOfficialPaths() {
        val previous = snapshot(listOf(message("m0", "old"), message("m1", "tail")), observed = 2)
        val incoming = snapshot(listOf(message("m1", "tail")), observed = 2)
        assertEquals(previous.messages,
            WebChatSnapshotWindowMerger.mergeConversation(previous, incoming, false).messages)
        for (url in listOf("https://other.example/c/example", "https://chatgpt.com/",
            "https://chatgpt.com/c/another")) {
            val different = incoming.copy(url = url)
            assertEquals(different,
                WebChatSnapshotWindowMerger.mergeConversation(previous, different, true))
        }
        assertEquals(incoming, WebChatSnapshotWindowMerger.mergeConversation(null, incoming, true))
    }

    @Test
    fun sparseKnownDomMessagesDoNotEraseMiddleHistory() {
        val cached = snapshot((0 until 18).map { message("m$it", "body$it") }, observed = 18)
        val live = snapshot(listOf(message("m0", "body0"), message("m8", "updated8"),
            message("m17", "body17")), observed = 18)
        val merged = WebChatSnapshotWindowMerger.merge(cached, live, true)
        assertEquals((0 until 18).map { "m$it" }, merged.messages.map { it.id })
        assertEquals("updated8", merged.messages[8].content)
        assertEquals(0, merged.messageWindowStart)
        assertEquals(18, merged.observedMessageCount)
    }

    @Test
    fun completePrivateHistoryReplacesOldBranchAndInflatedWindow() {
        val previous = snapshot(listOf(message("u1", "question"), message("old", "old branch")),
            start = 5, observed = 9)
        val incoming = snapshot(listOf(message("u1", "question"), message("new", "new branch")), observed = 2)
        val merged = WebChatSnapshotWindowMerger.merge(previous, incoming, true, authoritativeHistory = true)
        assertEquals(incoming, merged)
    }

    @Test
    fun fullReadThenSparseDomRemainsComplete() {
        val all = (0 until 18).map { message("m$it", "body$it") }
        val initial = snapshot(listOf(all.first(), all.last()), start = 8, observed = 20)
        val history = WebChatSnapshotWindowMerger.merge(initial, snapshot(all, observed = 18), true, true)
        val sparse = snapshot(listOf(all[0], all[8], all[17]), observed = 18)
        var current = history
        repeat(3) { current = WebChatSnapshotWindowMerger.merge(current, sparse, true) }
        assertEquals(history, current)
    }

    @Test
    fun privateAuthorityCannotReplaceActiveStreamOrPartialWindow() {
        val previous = snapshot(listOf(message("u1", "question"), message("a1", "answer")), observed = 2)
        val partial = snapshot(listOf(message("a1", "answer")), start = 1, observed = 2)
        assertEquals(previous.messages,
            WebChatSnapshotWindowMerger.merge(previous, partial, true, true).messages)
        val active = previous.copy(streaming = true)
        val candidate = snapshot(listOf(message("u1", "question")), observed = 1)
        assertEquals(previous.messages,
            WebChatSnapshotWindowMerger.merge(active, candidate, true, true).messages)
    }

    @Test
    fun privateHistoryRespectsBoundAndConversationIsolation() {
        val previous = snapshot(listOf(message("old", "old")), observed = 1)
        val history = snapshot((0 until 90).map { message("m$it", "body$it") }, observed = 90)
        val bounded = WebChatSnapshotWindowMerger.merge(previous, history, true, true)
        assertEquals(80, bounded.messages.size)
        assertEquals(10, bounded.messageWindowStart)
        assertEquals(90, bounded.observedMessageCount)
        val another = snapshot(listOf(message("new", "new")), observed = 1)
        assertEquals(another, WebChatSnapshotWindowMerger.merge(previous, another, false, true))
    }

    @Test
    fun partialOfficialWindowUpdatesCacheWithoutErasingOlderMessages() {
        val cached = snapshot(
            messages = listOf(message("u1", "旧问题"), message("a1", "旧回答"),
                message("u2", "新问题"), message("a2", "旧的部分回答")),
            observed = 4,
        )
        val live = snapshot(
            messages = listOf(message("u2", "新问题"), message("a2", "完整回答")),
            start = 2,
            observed = 4,
        )

        val merged = WebChatSnapshotWindowMerger.merge(cached, live, sameConversation = true)

        assertEquals(listOf("旧问题", "旧回答", "新问题", "完整回答"),
            merged.messages.map { it.content })
        assertEquals(0, merged.messageWindowStart)
        assertEquals(4, merged.observedMessageCount)
    }

    @Test
    fun emptyTransientSnapshotKeepsCachedConversationVisible() {
        val cached = snapshot(listOf(message("u1", "问题"), message("a1", "回答")), observed = 2)

        val merged = WebChatSnapshotWindowMerger.merge(
            cached,
            snapshot(emptyList(), observed = 0),
            sameConversation = true,
        )

        assertEquals(listOf("问题", "回答"), merged.messages.map { it.content })
    }

    @Test
    fun differentConversationNeverInheritsCachedMessages() {
        val merged = WebChatSnapshotWindowMerger.merge(
            snapshot(listOf(message("old", "私人旧内容")), observed = 1),
            snapshot(listOf(message("new", "新会话")), observed = 1),
            sameConversation = false,
        )

        assertEquals(listOf("新会话"), merged.messages.map { it.content })
    }

    @Test
    fun mergedWindowRemainsBoundedToTheAdapterLimit() {
        val cached = snapshot((0 until 80).map { message("m$it", "内容$it") }, observed = 80)
        val live = snapshot((70 until 90).map { message("m$it", "更新$it") }, start = 70, observed = 90)

        val merged = WebChatSnapshotWindowMerger.merge(cached, live, sameConversation = true)

        assertEquals(80, merged.messages.size)
        assertEquals(10, merged.messageWindowStart)
        assertEquals("内容10", merged.messages.first().content)
        assertEquals("更新89", merged.messages.last().content)
    }

    @Test
    fun exactDomBlockKeepsStructuredIdentityInBothWindowPaths() {
        val source = ChatGptWebMessagePart("writing_block", "Draft", textBlock =
            WebChatTextBlock("writing-a", "writing", "Draft", "", "body", true, "a1"))
        val dom = ChatGptWebMessagePart("code", "Code", textBlock =
            WebChatTextBlock("code-0", "code", "", "", "body", true))
        val old = message("a1", "body").copy(parts = listOf(source))
        val fresh = message("a1", "body").copy(parts = listOf(dom))
        for (reorder in listOf(false, true)) {
            val cached = snapshot(listOf(old, message("a2", "other")), observed = 2)
            val incoming = if (reorder) listOf(message("a2", "other"), fresh) else listOf(fresh)
            val merged = WebChatSnapshotWindowMerger.merge(cached, snapshot(incoming, observed = 2), true)
            // A reordered window cannot establish the same positional owner.
            assertEquals(if (reorder) dom else source, merged.messages.single { it.id == "a1" }.parts.single())
        }
    }

    @Test
    fun officialMessageReplacesItsExactStreamAliasWithoutKeepingADuplicateTail() {
        val id = "11111111-1111-4111-8111-111111111111"
        val user = message("u1", "fixture question")
        val stream = message("private-stream:$id", "FIXTURE_ANSWER")
        val official = message(id, "FIXTURE\\_ANSWER")
        for (old in listOf(listOf(user, stream), listOf(user, official, stream))) {
            val merged = WebChatSnapshotWindowMerger.merge(
                snapshot(old, observed = old.size), snapshot(listOf(user, official), observed = 2), true,
            )
            assertEquals(listOf("u1", id), merged.messages.map { it.id })
            assertEquals(official.content, merged.messages.last().content)
        }
    }

    @Test
    fun subsequentStreamSnapshotKeepsTheCanonicalIdentity() {
        val id = "11111111-1111-4111-8111-111111111111"
        val user = message("u1", "fixture question")
        val merged = WebChatSnapshotWindowMerger.merge(
            snapshot(listOf(user, message(id, "partial")), observed = 2),
            snapshot(listOf(user, message("private-stream:$id", "complete")), observed = 2), true,
        )
        assertEquals(listOf("u1", id), merged.messages.map { it.id })
        assertEquals("complete", merged.messages.last().content)
    }

    @Test
    fun matchingTextDoesNotMergeDifferentProviderMessagesOrPlaceholderSentinels() {
        val id = "11111111-1111-4111-8111-111111111111"
        val other = "22222222-2222-4222-8222-222222222222"
        for (alias in listOf("private-stream:$other", "private-stream:assistant")) {
            val cached = listOf(message("u1", "fixture"), message(alias, "same answer"))
            val incoming = listOf(message("u1", "fixture"), message(id, "same answer"))
            val merged = WebChatSnapshotWindowMerger.merge(
                snapshot(cached, observed = 2), snapshot(incoming, observed = 2), true,
            )
            assertEquals(setOf("u1", id, alias), merged.messages.map { it.id }.toSet())
        }
    }

    @Test
    fun exactStreamIdentityPreservesWritingBlockStructureWhenCanonicalDomArrives() {
        val id = "11111111-1111-4111-8111-111111111111"
        val block = ChatGptWebMessagePart("writing_block", "Draft", textBlock =
            WebChatTextBlock("writing-a", "writing", "Draft", "", "body", true, id))
        val dom = ChatGptWebMessagePart("code", "Code", textBlock =
            WebChatTextBlock("code-0", "code", "", "", "body", true))
        val merged = WebChatSnapshotWindowMerger.merge(
            snapshot(listOf(message("private-stream:$id", "body").copy(parts = listOf(block))), observed = 1),
            snapshot(listOf(message(id, "body").copy(parts = listOf(dom))), observed = 1), true,
        )
        assertEquals(id, merged.messages.single().id)
        assertEquals(block, merged.messages.single().parts.single())
    }

    private fun snapshot(
        messages: List<ChatGptWebMessage>,
        start: Int = 0,
        observed: Int,
    ) = ChatGptWebSnapshot(
        title = "会话",
        url = "https://chatgpt.com/c/example",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "自动",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf("send_prompt")),
        pageKind = "conversation",
        messageWindowStart = start,
        observedMessageCount = observed,
    )

    private fun message(id: String, content: String) = ChatGptWebMessage(
        id = id,
        role = if (id.startsWith("u")) "user" else "assistant",
        content = content,
        state = "completed",
        parts = emptyList(),
    )
}
