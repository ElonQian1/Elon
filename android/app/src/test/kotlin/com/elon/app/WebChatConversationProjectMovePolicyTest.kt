package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConversationProjectMovePolicyTest {
    @Test
    fun timingAllowsWebViewRecoveryWithoutHighFrequencyPolling() {
        assertTrue(WebChatConversationProjectMoveTiming.POLL_INTERVAL_MS >= 500L)
        assertEquals(30_000L, WebChatConversationProjectMoveTiming.NAVIGATION_TIMEOUT_MS)
        assertEquals(15_000L, WebChatConversationProjectMoveTiming.CONTROL_TIMEOUT_MS)
        assertEquals(15_000L, WebChatConversationProjectMoveTiming.COMMAND_TIMEOUT_MS)
        assertEquals(30_000L, WebChatConversationProjectMoveTiming.RECONCILIATION_TIMEOUT_MS)
    }

    @Test
    fun controlStateRefreshesPeriodicallyOnlyWhileTheUserActionIsWaiting() {
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshControls(0))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshControls(1))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshControls(4))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshControls(8))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshControls(29))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshControls(30))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshControls(-1))
    }

    @Test
    fun conversationOptionsRetryRunsOnceAfterAStableReadWindow() {
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryConversationOptions(0, 0))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryConversationOptions(7, 0))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRetryConversationOptions(8, 0))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryConversationOptions(8, 1))
    }

    @Test
    fun projectDirectoryRefreshesPeriodicallyOnlyDuringReconciliation() {
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(0))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(1))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(10))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(20))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(59))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(60))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshDirectory(-1))
    }

    @Test
    fun fullDirectoryRefreshRunsLessOftenThanScopedReconciliation() {
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(10))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(20))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(30))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(40))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(50))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRefreshFullDirectory(60))
    }

    @Test
    fun conversationNavigationRetriesOnlyTwiceBeforeAnyWrite() {
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryNavigation(0))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryNavigation(19))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRetryNavigation(20))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryNavigation(21))
        assertTrue(WebChatConversationProjectMoveTiming.shouldRetryNavigation(40))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryNavigation(41))
        assertFalse(WebChatConversationProjectMoveTiming.shouldRetryNavigation(60))
    }

    @Test
    fun staleSheetCallbacksCannotReleaseTheCurrentProgressSheet() {
        val lease = WebChatConversationProjectMoveSheetLease()
        val picker = lease.issue()
        val progress = lease.issue()

        assertFalse(lease.owns(picker))
        assertTrue(lease.owns(progress))

        lease.invalidate()
        assertFalse(lease.owns(progress))
    }

    @Test
    fun destinationsUseCachedProjectsAndExcludeTheCurrentProject() {
        val index = ChatGptWebConversationIndexState(
            projects = listOf(
                project("g-p-current", "当前项目"),
                project("g-p-target", "目标项目"),
                project("g-p-target", "重复项目"),
            ),
        )

        assertEquals(
            listOf("g-p-target"),
            WebChatConversationProjectMovePolicy.destinations(index, conversation()).map { it.id },
        )
    }

    @Test
    fun resolvesEachOfficialMenuStageWithoutGuessingAProjectChoice() {
        val current = conversation()
        val options = descriptor(
            id = "control_conversation_options",
            semantic = "conversation_options",
            label = "会话设置",
            region = "header",
            contextId = current.id,
        )
        val trigger = descriptor(
            id = "control_save_to_project",
            semantic = "save_to_project",
            label = "添加到项目",
            region = "overlay",
            contextId = current.id,
        )
        val choice = descriptor(
            id = "control_project_target",
            semantic = "project",
            label = "目标项目",
            region = "overlay",
            contextId = null,
            role = "menuitem",
        )
        val state = state(options, trigger, choice)

        assertEquals(options, WebChatConversationProjectMovePolicy.conversationOptions(state, current))
        assertEquals(trigger, WebChatConversationProjectMovePolicy.moveTrigger(state, current))
        assertEquals(
            choice,
            WebChatConversationProjectMovePolicy.projectChoice(
                state,
                project("g-p-target", "目标项目"),
            ),
        )
        assertEquals(
            listOf("g-p-target"),
            WebChatConversationProjectMovePolicy.officialDestinations(
                ChatGptWebConversationIndexState(
                    projects = listOf(
                        project("g-p-current", "当前项目"),
                        project("g-p-target", "目标项目"),
                        project("g-p-hidden", "未开放项目"),
                    ),
                ),
                current,
                state,
            ).map(ChatGptWebProject::id),
        )
    }

    @Test
    fun waitsForTheOfficialSidebarToCloseBeforeOpeningConversationOptions() {
        val current = conversation()
        val header = descriptor(
            id = "control_conversation_options",
            semantic = "conversation_options",
            label = "会话设置",
            region = "header",
            contextId = current.id,
        )
        val sidebarConversation = descriptor(
            id = "control_sidebar_conversation_options",
            semantic = "conversation_options",
            label = "其他会话设置",
            region = "overlay",
            contextId = "another-conversation",
        )

        assertNull(
            WebChatConversationProjectMovePolicy.conversationOptions(
                state(header, sidebarConversation),
                current,
            ),
        )
        assertEquals(
            header,
            WebChatConversationProjectMovePolicy.conversationOptions(
                state(
                    header,
                    descriptor(
                        id = "control_hidden_sidebar_conversation_options",
                        semantic = "conversation_options",
                        label = "其他会话设置",
                        region = "overlay",
                        contextId = "another-conversation",
                        inViewport = false,
                    ),
                ),
                current,
            ),
        )
    }

    @Test
    fun retriesTheHeaderMenuOnlyWhenNoOfficialOverlayIsVisible() {
        val current = conversation()
        val header = descriptor(
            id = "control_conversation_options",
            semantic = "conversation_options",
            label = "会话设置",
            region = "header",
            contextId = current.id,
        )
        val unrelatedOverlay = descriptor(
            id = "control_overlay",
            semantic = "archive",
            label = "归档",
            region = "overlay",
            contextId = current.id,
        )

        assertEquals(
            header,
            WebChatConversationProjectMovePolicy.retryableConversationOptions(
                state(header),
                current,
            ),
        )
        assertNull(
            WebChatConversationProjectMovePolicy.retryableConversationOptions(
                state(header, unrelatedOverlay),
                current,
            ),
        )
    }

    @Test
    fun supportsLegacyGenericTriggerButFailsClosedForAmbiguousProjectTitles() {
        val current = conversation()
        val legacyTrigger = descriptor(
            id = "control_project_menu",
            semantic = "project",
            label = "Add to project",
            region = "overlay",
            contextId = current.id,
        )
        assertEquals(
            legacyTrigger,
            WebChatConversationProjectMovePolicy.moveTrigger(state(legacyTrigger), current),
        )
        assertTrue(WebChatConversationProjectMovePolicy.isGenericMoveLabel("移动到项目"))
        assertFalse(WebChatConversationProjectMovePolicy.isGenericMoveLabel("家庭成员健康"))

        val first = descriptor(
            id = "control_project_first",
            semantic = "project",
            label = "同名项目",
            region = "overlay",
            contextId = null,
            role = "menuitem",
        )
        val second = descriptor(
            id = "control_project_second",
            semantic = "project",
            label = "同名项目",
            region = "overlay",
            contextId = null,
            role = "menuitem",
        )
        assertNull(
            WebChatConversationProjectMovePolicy.projectChoice(
                state(first, second),
                project("g-p-target", "同名项目"),
            ),
        )
    }

    @Test
    fun acceptsOneUnscopedOfficialMoveTriggerButRejectsAnotherConversationContext() {
        val current = conversation()
        val unscoped = descriptor(
            id = "control_unscoped_move",
            semantic = "save_to_project",
            label = "移至项目",
            region = "overlay",
            contextId = null,
        )
        assertEquals(
            unscoped,
            WebChatConversationProjectMovePolicy.moveTrigger(state(unscoped), current),
        )

        val foreign = descriptor(
            id = "control_foreign_move",
            semantic = "save_to_project",
            label = "移至项目",
            region = "overlay",
            contextId = "another-conversation",
        )
        assertNull(WebChatConversationProjectMovePolicy.moveTrigger(state(foreign), current))
        assertNull(WebChatConversationProjectMovePolicy.moveTrigger(state(unscoped, foreign), current))
    }

    @Test
    fun reconciliationRequiresTheSameConversationInTheSelectedProject() {
        val current = conversation()
        val destination = project("g-p-target", "目标项目")
        val moved = current.copy(
            path = "/g/${destination.id}/c/${current.id}",
            projectId = destination.id,
            projectTitle = destination.title,
            projectPath = destination.path,
        )

        assertTrue(
            WebChatConversationProjectMovePolicy.reconciled(
                ChatGptWebConversationIndexState(conversations = listOf(moved)),
                current,
                destination,
            ),
        )
        assertFalse(
            WebChatConversationProjectMovePolicy.reconciled(
                ChatGptWebConversationIndexState(conversations = listOf(current, moved)),
                current,
                destination,
            ),
        )
        assertFalse(
            WebChatConversationProjectMovePolicy.reconciled(
                ChatGptWebConversationIndexState(
                    conversations = listOf(moved.copy(path = current.path)),
                ),
                current,
                destination,
            ),
        )
        assertFalse(
            WebChatConversationProjectMovePolicy.reconciled(
                ChatGptWebConversationIndexState(conversations = listOf(current)),
                current,
                destination,
            ),
        )
    }

    @Test
    fun moveConfirmationIsUniqueAndBoundToTheCurrentConversation() {
        val current = conversation()
        val confirm = descriptor(
            id = "control_confirm_move",
            semantic = "confirm",
            label = "确认",
            region = "overlay",
            contextId = current.id,
        )
        assertEquals(
            confirm,
            WebChatConversationProjectMovePolicy.confirmation(state(confirm), current),
        )

        val stale = descriptor(
            id = "control_stale_confirm",
            semantic = "confirm",
            label = "确认",
            region = "overlay",
            contextId = "another-conversation",
        )
        assertNull(WebChatConversationProjectMovePolicy.confirmation(state(stale), current))
        val duplicate = descriptor(
            id = "duplicate",
            semantic = "confirm",
            label = "确认",
            region = "overlay",
            contextId = current.id,
        )
        assertNull(
            WebChatConversationProjectMovePolicy.confirmation(
                state(confirm, duplicate),
                current,
            ),
        )
    }

    @Test
    fun commandStatusIsBoundToTheExactRequestId() {
        val state = state().copy(
            commandRequests = listOf(
                WebChatConsumerCommandRequest("older", WebChatConsumerCommandStatus.FAILED),
                WebChatConsumerCommandRequest("target", WebChatConsumerCommandStatus.SUCCEEDED),
            ),
        )

        assertEquals(
            WebChatConsumerCommandStatus.SUCCEEDED,
            WebChatConversationProjectMovePolicy.commandStatus(state, "target"),
        )
        assertEquals(
            WebChatConsumerCommandStatus.UNKNOWN,
            WebChatConversationProjectMovePolicy.commandStatus(state, "missing"),
        )
    }

    @Test
    fun anAcceptedWriteWithoutAReceiptIsStillTreatedAsPossiblySubmitted() {
        assertTrue(
            WebChatConversationProjectMovePolicy.writeMayHaveBeenSubmitted(
                WebChatConsumerCommandResult(accepted = true, requestId = null),
            ),
        )
        assertFalse(
            WebChatConversationProjectMovePolicy.writeMayHaveBeenSubmitted(
                WebChatConsumerCommandResult(accepted = false, requestId = null),
            ),
        )
    }

    private fun conversation() = ChatGptWebConversation(
        id = "conversation-current",
        title = "当前会话",
        path = "/c/conversation-current",
        active = true,
        projectId = "g-p-current",
        projectTitle = "当前项目",
        projectPath = "/g/g-p-current/project",
    )

    private fun project(id: String, title: String) = ChatGptWebProject(
        id = id,
        title = title,
        path = "/g/$id/project",
    )

    private fun state(vararg controls: WebChatConsumerControlDescriptor) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = "conversation",
        pageUrl = "https://chatgpt.com/c/conversation-current",
        features = emptyList(),
        controls = controls.toList(),
        commandRequests = emptyList(),
    )

    private fun descriptor(
        id: String,
        semantic: String,
        label: String,
        region: String,
        contextId: String?,
        role: String = "button",
        inViewport: Boolean = true,
    ) = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = id,
            label = label,
            semantic = semantic,
            region = region,
            role = role,
            enabled = true,
            selected = false,
            contextId = contextId,
            inViewport = inViewport,
        ),
        requiresUserConfirmation = semantic == "save_to_project",
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = "selector:$id",
        pageActionPlacement = WebChatConsumerPageActionPlacement.CONVERSATION,
    )
}
