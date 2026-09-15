package com.elon.app

import android.app.Application
import android.view.View
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.android.controller.ActivityController
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = Application::class)
class AiConversationShareSelectionActionsTest {
    private lateinit var lifecycle: ActivityController<AppCompatActivity>
    private lateinit var binding: ActivityMainBinding
    private lateinit var conversation: AppConversation
    private lateinit var adapter: ChatAdapter
    private lateinit var actions: MainChatSelectionActions
    private var saves = 0
    private var renders = 0

    @Before fun setup() {
        lifecycle = Robolectric.buildActivity(AppCompatActivity::class.java)
        val activity = lifecycle.get()
        activity.setTheme(R.style.Theme_ElonApp)
        lifecycle.setup()
        binding = ActivityMainBinding.inflate(activity.layoutInflater)
        activity.setContentView(binding.root)
        binding.chatPage.visibility = View.VISIBLE
        binding.inputLayout.visibility = View.VISIBLE
        binding.pageTabs.visibility = View.VISIBLE
        binding.inputEdit.setText("unsent draft")
        binding.inputEdit.setSelection(5)
        conversation = AppConversation(
            id = "existing-conversation", title = "Existing conversation", subtitle = "", updatedAt = 0L,
            messages = mutableListOf(
                ChatMessage("friend", "keep first", id = "keep-first"),
                ChatMessage("friend", "delete first", id = "delete-first"),
                ChatMessage("friend", "keep middle", id = "keep-middle"),
                ChatMessage("friend", "delete last", id = "delete-last"),
            ),
        )
        adapter = ChatAdapter(conversation.messages)
        binding.chatList.itemAnimator = null
        binding.chatList.adapter = adapter
        actions = MainChatSelectionActions(
            activity = activity,
            binding = binding,
            chatAdapter = { adapter },
            activeConversation = { conversation },
            saveConversations = { saves++ },
            renderConversationList = { renders++ },
            shareActions = { error("Selection deletion must not invoke sharing") },
            isProjectChannelActive = { false },
            summarizeInCurrentChannel = { error("Unexpected summary") },
            summarizeInPersonalChat = { error("Unexpected summary") },
            summarizeInNewPersonalChat = { error("Unexpected summary") },
            isAiChat = { false },
        )
        actions.setup()
    }

    @After fun cleanup() {
        lifecycle.pause().stop().destroy()
    }

    @Test fun nonAiDeleteRestoresInputAndTabsAndRemovesOnlySelectedRows() {
        assertTrue(adapter.ownsMessages(conversation.messages))
        actions.startSelection(conversation.messages[1])
        assertTrue(actions.isSelectionActive())
        assertEquals(View.GONE, binding.inputLayout.visibility)
        assertEquals(View.GONE, binding.pageTabs.visibility)
        assertEquals(View.VISIBLE, binding.chatSelectionBar.visibility)
        layoutRows()
        assertTrue(row(3).itemView.performClick())
        assertEquals(listOf("delete-first", "delete-last"), adapter.selectedMessagesInOrder().map { it.id })

        assertTrue(binding.selectionDeleteButton.isEnabled)
        assertTrue(binding.selectionDeleteButton.performClick())

        assertEquals(listOf("keep-first", "keep-middle"), conversation.messages.map { it.id })
        assertEquals(listOf("keep-first", "keep-middle"), adapter.currentMessagesForSharing().map { it.id })
        assertTrue(adapter.ownsMessages(conversation.messages))
        assertEquals(2, adapter.itemCount)
        assertEquals(1, saves)
        assertEquals(1, renders)
        assertSelectionClosed(View.VISIBLE, View.VISIBLE)
    }

    @Test fun cancelWithZeroSelectedPreservesOriginalVisibilityAndMessages() {
        val originalIds = conversation.messages.map { it.id }
        listOf(
            View.VISIBLE to View.VISIBLE,
            View.GONE to View.INVISIBLE,
            View.INVISIBLE to View.GONE,
        ).forEach { (inputVisibility, tabsVisibility) ->
            binding.inputLayout.visibility = inputVisibility
            binding.pageTabs.visibility = tabsVisibility
            actions.startSelection(conversation.messages[1])
            layoutRows()
            assertTrue(row(1).itemView.performClick())

            assertTrue(actions.isSelectionActive())
            assertTrue(adapter.selectedMessagesInOrder().isEmpty())
            assertEquals(View.VISIBLE, binding.chatSelectionBar.visibility)
            assertEquals(View.GONE, binding.inputLayout.visibility)
            assertEquals(View.GONE, binding.pageTabs.visibility)
            assertFalse(binding.selectionDeleteButton.isEnabled)
            assertTrue(binding.selectionCancelButton.performClick())

            assertSelectionClosed(inputVisibility, tabsVisibility)
            actions.cancelSelection()
            assertSelectionClosed(inputVisibility, tabsVisibility)
            assertEquals(originalIds, conversation.messages.map { it.id })
        }
        assertEquals(0, saves)
        assertEquals(0, renders)
    }

    private fun assertSelectionClosed(inputVisibility: Int, tabsVisibility: Int) {
        assertFalse(actions.isSelectionActive())
        assertTrue(adapter.selectedMessagesInOrder().isEmpty())
        assertEquals(View.GONE, binding.chatSelectionBar.visibility)
        assertEquals(inputVisibility, binding.inputLayout.visibility)
        assertEquals(tabsVisibility, binding.pageTabs.visibility)
        assertEquals("unsent draft", binding.inputEdit.text.toString())
        assertEquals(5, binding.inputEdit.selectionStart)
        assertFalse(binding.selectionDeleteButton.isEnabled)
    }

    private fun layoutRows() {
        binding.chatList.requestLayout()
        binding.chatList.measure(
            View.MeasureSpec.makeMeasureSpec(360, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(800, View.MeasureSpec.EXACTLY),
        )
        binding.chatList.layout(0, 0, 360, 800)
    }

    private fun row(index: Int): ChatAdapter.VH =
        requireNotNull(binding.chatList.findViewHolderForAdapterPosition(index) as? ChatAdapter.VH)
}
