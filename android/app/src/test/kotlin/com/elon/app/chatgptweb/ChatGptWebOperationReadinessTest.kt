package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptWebOperationReadinessTest {
    private val page = ChatGptWebSnapshot(
        title = "Fixture", url = "https://chatgpt.com/", draft = "", messages = emptyList(),
        authenticated = true, composerReady = false, streaming = false, currentModel = "auto",
        attachments = emptyList(), dictationActive = false,
        capabilities = ChatGptWebCapabilities(emptySet()),
    )

    private fun rejection(action: String, snapshot: ChatGptWebSnapshot? = page,
        current: Boolean = true, ready: Boolean = false) =
        ChatGptWebOperationReadiness.rejection(action, snapshot, current, ready)

    @Test fun everyPublishedActionHasExactlyOneExplicitRequirement() {
        (ChatGptWebMcpActionCatalog.availableActions + "open_chatgpt_web").forEach {
            assertNotNull(it, ChatGptWebOperationReadiness.requirement(it))
        }
        assertEquals("unsupported_action", rejection("future_unreviewed_write", ready = true))
    }

    @Test fun cachedDirectoryAndNativeRecoveryNeverWaitForAWebPage() {
        listOf("chatgpt_get_conversations", "chatgpt_get_navigation", "state", "chatgpt_refresh",
            "chatgpt_cancel_file_download", "chatgpt_select_view").forEach {
            assertNull(it, rejection(it, null, current = false))
        }
    }

    @Test fun authenticatedReadsIgnoreComposerAndChatRateLimit() {
        listOf("chatgpt_list_conversations", "chatgpt_list_library_files", "chatgpt_list_conversation_files",
            "chatgpt_download_library_file", "chatgpt_download_conversation_file").forEach {
            assertNull(it, rejection(it))
            assertNull(it, rejection(it, page.copy(accessReason = "rate_limited")))
        }
    }

    @Test fun currentDocumentOperationsDoNotRequireAComposerOrAccount() {
        listOf("chatgpt_open_conversation", "chatgpt_list_composer_options", "chatgpt_refresh_controls",
            "chatgpt_list_features", "chatgpt_get_context", "chatgpt_find_controls",
            "chatgpt_stop_generation", "chatgpt_cancel_library_files").forEach {
            assertNull(it, rejection(it, page.copy(authenticated = false)))
        }
    }

    @Test fun accountMutationsDoNotWaitForComposerButRetainDocumentAndLoginAdmission() {
        listOf("chatgpt_set_conversation_pinned", "chatgpt_set_conversation_archived",
            "chatgpt_rename_conversation", "chatgpt_move_conversation_to_project",
            "chatgpt_delete_conversation", "chatgpt_share_conversation",
            "chatgpt_mutate_library_file").forEach { action ->
            assertEquals(ChatGptWebOperationReadiness.Requirement.ACCOUNT_MUTATION,
                ChatGptWebOperationReadiness.requirement(action))
            assertNull(action, rejection(action))
            assertNull(action, rejection(action, page.copy(authenticated = false)))
            assertEquals("adapter_generation_not_ready", rejection(action, current = false))
            assertEquals("login_required", rejection(action, page.copy(loginRequired = true)))
            assertEquals("unsupported_page", rejection(action, page.copy(url = "https://example.com/")))
        }
    }

    @Test fun privateReadersCanAcquireIdentityBeforeTheUiReportsAuthentication() {
        assertNull(rejection("chatgpt_list_library_files", page.copy(authenticated = false)))
        assertNull(rejection("chatgpt_list_conversations", page.copy(authenticated = false)))
        assertNull(rejection("chatgpt_list_conversations", page.copy(authenticated = false, composerReady = true)))
        assertNull(rejection("chatgpt_list_library_files",
            page.copy(authenticated = false, composerReady = true)))
    }

    @Test fun documentCommandsDoNotWaitForAnIndependentChatSnapshot() {
        listOf("chatgpt_find_controls", "chatgpt_open_conversation", "chatgpt_list_composer_options",
            "chatgpt_cancel_library_files").forEach { assertNull(it, rejection(it, null)) }
        assertNull(rejection("chatgpt_list_library_files", null))
    }

    @Test fun staleAdaptersCannotIssueNetworkOrPageCommandsEvenWithACachedLogin() {
        listOf("chatgpt_list_library_files", "chatgpt_list_conversations", "chatgpt_list_composer_options",
            "chatgpt_open_conversation", "chatgpt_get_context", "chatgpt_find_controls").forEach {
            assertEquals(it, "adapter_generation_not_ready", rejection(it, current = false))
        }
    }

    @Test fun explicitLoginAndUnsupportedOriginBlockButMissingUiSnapshotDoesNot() {
        val action = "chatgpt_list_conversation_files"
        assertNull(rejection(action, null))
        assertEquals("login_required", rejection(action, page.copy(loginRequired = true)))
        assertEquals("login_required", rejection(action, page.copy(url = "https://chatgpt.com/auth/login")))
        listOf("https://example.com/", "http://chatgpt.com/", "https://chatgpt.com:444/",
            "https://user@chatgpt.com/").forEach {
            assertEquals(it, "unsupported_page", rejection(action, page.copy(url = it)))
        }
    }

    @Test fun writesKeepExistingReadinessAndGenerationChecks() {
        ChatGptWebMcpActionCatalog.availableActions.filter {
            ChatGptWebOperationReadiness.requirement(it) == ChatGptWebOperationReadiness.Requirement.COMPOSER
        }.forEach {
            assertEquals(it, "bridge_not_ready", rejection(it))
            assertEquals(it, "adapter_generation_not_ready", rejection(it, current = false, ready = true))
            assertNull(it, rejection(it, page.copy(composerReady = true), ready = true))
        }
    }
}
