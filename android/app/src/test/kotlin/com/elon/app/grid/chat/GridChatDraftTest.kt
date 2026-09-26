package com.elon.app.grid.chat

import org.junit.Assert.*
import org.junit.Test

class GridChatDraftTest {
    private var clock = 100L
    private val source = BinanceGridReadStore.Context("owner", "account", "sub", "doc")
    private val fields = (BinanceGridReadStore.baseFields + BinanceGridReadStore.metricFields)
        .associateWith { when (it) { "id" -> "123"; "symbol" -> "龙虾USDT"; "investment" -> "4029.33780960"; else -> null } }
    @Test fun plainChatDoesNotNeedAnExchangeSession() {
        assertNull(GridChatDraft { clock }.validate("普通问题", null, null))
    }
    @Test fun exactPreviewIsTheVisibleSendPayloadAndNoIdentityLeaves() {
        val draft = GridChatDraft { clock }; val block = draft.stage("chat1", source, 1790000000000, 1000, fields)
        assertNull(draft.validate("请分析\n$block", "chat1", source))
        assertTrue(block.contains("4029.33780960")); assertFalse(block.contains("account")); assertFalse(block.contains("owner"))
        assertEquals("question_required", draft.validate(block, "chat1", source))
    }
    @Test fun expiryAccountDocumentChatAndLostStateRejectExistingSnapshot() {
        val draft = GridChatDraft { clock }; val block = draft.stage("chat1", source, 99, 1000, fields)
        assertEquals("context_changed", draft.validate("问题$block", "chat2", source))
        assertEquals("context_changed", draft.validate("问题$block", "chat1", source.copy(account = "other")))
        assertEquals("context_changed", draft.validate("问题$block", "chat1", source.copy(document = "next")))
        assertEquals("snapshot_missing", GridChatDraft { clock }.validate("问题$block", "chat1", source))
        clock += 1000; assertEquals("snapshot_expired", draft.validate("问题$block", "chat1", source))
    }
    @Test fun userEditsAndDuplicateBlocksCannotPassAsVerifiedFacts() {
        val draft = GridChatDraft { clock }; val block = draft.stage("chat1", source, 99, 1000, fields)
        assertEquals("snapshot_modified", draft.validate("问题" + block.replace("4029.33780960", "0"), "chat1", source))
        assertEquals("snapshot_modified", draft.validate("问题$block$block", "chat1", source))
        assertEquals("问题", draft.clear("问题\n\n$block")); assertFalse(draft.present)
        assertNull(draft.validate("问题", "chat2", null))
    }
    @Test fun failedSendRetryUsesSameCompleteDraftWithoutAppendingAgain() {
        val draft = GridChatDraft { clock }; val block = draft.stage("chat1", source, 99, 1000, fields)
        val text = "问题\n$block"
        repeat(2) { assertNull(draft.validate(text, "chat1", source)) }
        assertEquals(1, Regex(Regex.escape(GridChatDraft.BEGIN)).findAll(text).count())
    }
}
