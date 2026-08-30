package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionHeaderActionsTest {
    @Test
    fun exposesTheHeaderEntryWhileAChatGptChatPageIsRestoring() {
        val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val google = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)

        assertTrue(WebChatProductionHeaderActionPolicy.visible(chatGpt, "ready", "home"))
        assertTrue(WebChatProductionHeaderActionPolicy.visible(chatGpt, "loading", "home"))
        assertTrue(WebChatProductionHeaderActionPolicy.visible(chatGpt, "idle", "conversation"))
        assertTrue(WebChatProductionHeaderActionPolicy.visible(chatGpt, "ready", "conversation"))
        assertFalse(WebChatProductionHeaderActionPolicy.visible(chatGpt, "connecting", "home"))
        assertFalse(WebChatProductionHeaderActionPolicy.visible(chatGpt, "error", "home"))
        assertFalse(WebChatProductionHeaderActionPolicy.visible(chatGpt, "ready", "feature"))
        assertFalse(WebChatProductionHeaderActionPolicy.visible(google, "ready", "conversation"))
    }

    @Test
    fun resolvesObservedTemporaryStateWithoutDuplicatingConversationSettings() {
        val resolved = WebChatProductionHeaderActionPolicy.resolve(
            state(
                pageKind = "conversation",
                control = temporaryControl(selected = true),
            ),
            currentConversationPath = "/c/current",
        )

        assertNotNull(resolved.temporaryChat)
        assertTrue(resolved.temporaryChatSelected)
        assertTrue(resolved.conversationSettingsAvailable)
    }

    @Test
    fun keepsTemporaryStateUnknownUntilTheOfficialControlIsObserved() {
        val resolved = WebChatProductionHeaderActionPolicy.resolve(
            state(pageKind = "home", control = null),
            currentConversationPath = null,
        )

        assertNull(resolved.temporaryChat)
        assertFalse(resolved.temporaryChatSelected)
        assertFalse(resolved.conversationSettingsAvailable)
    }

    @Test
    fun rejectsDisabledOrNonSettableTemporaryControls() {
        val disabled = temporaryControl(selected = false, enabled = false)
        val nonSettable = temporaryControl(selected = false, stateSettable = false)

        assertNull(WebChatProductionHeaderActionPolicy.resolve(
            state("home", disabled),
            null,
        ).temporaryChat)
        assertNull(WebChatProductionHeaderActionPolicy.resolve(
            state("home", nonSettable),
            null,
        ).temporaryChat)
    }

    @Test
    fun usesADedicatedTemporaryChatIconAndStateLabel() {
        val inactive = WebChatProductionHeaderActionPolicy.buttonPresentation(selected = false)
        val active = WebChatProductionHeaderActionPolicy.buttonPresentation(selected = true)
        val syncing = WebChatProductionHeaderActionPolicy.buttonPresentation(selected = null)

        assertEquals(R.drawable.ic_temporary_chat, inactive.iconRes)
        assertFalse(inactive.selected)
        assertEquals("临时聊天未开启", inactive.statusLabel)
        assertEquals(R.drawable.ic_temporary_chat, active.iconRes)
        assertTrue(active.selected)
        assertEquals("临时聊天已开启", active.statusLabel)
        assertEquals(R.drawable.ic_temporary_chat, syncing.iconRes)
        assertFalse(syncing.selected)
        assertEquals("临时聊天状态同步中", syncing.statusLabel)
    }

    @Test
    fun keepsTheTemporaryChatPresetActionableBeforeTheOfficialControlIsObserved() {
        val item = WebChatProductionHeaderActionPolicy.temporaryChatItem(
            control = null,
            observation = WebChatProductionObservationState.SYNCING,
        )

        assertEquals(WebChatProductionHeaderActionPolicy.TEMPORARY_ITEM_ID, item.id)
        assertEquals("临时聊天", item.title)
        assertTrue(item.enabled)
        assertTrue(item.subtitle!!.contains("后台确认"))
        assertEquals("chatgpt-native:temporary-chat:临时聊天", item.contentDescription)
    }

    private fun state(
        pageKind: String,
        control: ChatGptWebUiControl?,
    ) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = pageKind,
        pageUrl = if (pageKind == "conversation") {
            "https://chatgpt.com/c/current"
        } else {
            "https://chatgpt.com/"
        },
        features = emptyList(),
        controls = control?.let {
            listOf(WebChatConsumerControlDescriptor(
                control = it,
                requiresUserConfirmation = false,
                presentation = WebChatConsumerControlPresentation.DIRECT,
                nativeSelector = "chatgpt-native:temporary-chat:临时聊天",
            ))
        }.orEmpty(),
        commandRequests = emptyList(),
    )

    private fun temporaryControl(
        selected: Boolean,
        enabled: Boolean = true,
        stateSettable: Boolean = true,
    ) = ChatGptWebUiControl(
        id = "control_temporary_chat",
        label = if (selected) "关闭临时聊天" else "临时聊天",
        semantic = "temporary_chat",
        region = "header",
        role = "button",
        enabled = enabled,
        selected = selected,
        stateSettable = stateSettable,
    )
}
