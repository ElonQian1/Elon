package com.elon.app.chatgptweb

import com.elon.app.PendingAttachment
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatAttachmentSelectionKind
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class ChatGptWebAttachmentPickerPreparationTest {
    @get:Rule val temporary = TemporaryFolder()

    private class Fixture {
        var state = WebBridgeDocumentSession.Snapshot(1, 1, "doc_fixture_1")
        var url: String? = "https://chatgpt.com/"
        var available = true
        var time = 0L
        val scripts = mutableListOf<String>()
        val timers = mutableMapOf<Runnable, Long>()
        val owner = ChatGptWebAttachmentPickerPreparation(
            { state }, { url }, { available }, { scripts.add(it) },
            { task, delay -> timers[task] = delay }, { timers.remove(it) }, { time },
        )
        fun begin() = requireNotNull(owner.begin(WebChatAttachmentSelectionKind.DOCUMENT))
    }

    private fun attachment(): PendingAttachment {
        val file = temporary.newFile().apply { writeText("synthetic fixture") }
        return PendingAttachment("file", displayName = "fixture.txt", fileName = "fixture.txt",
            mimeType = "text/plain", file = file)
    }

    @Test fun exactPreparedFileTakesOnceWithoutExposingItsPathToThePage() {
        val f = Fixture()
        val selected = attachment()
        f.begin().selected(listOf(selected))
        assertTrue(requireNotNull(f.owner.take(selected)).startsWith("selection_"))
        assertNull(f.owner.take(selected))
        assertTrue(f.timers.isEmpty())
        assertEquals(1, f.scripts.size)
        assertFalse(f.scripts.single().contains(selected.file.path))
    }

    @Test fun samePathReplacementAndEditedFileCannotConsumeTheEarlierSelection() {
        for (replaceObject in listOf(true, false)) {
            val f = Fixture()
            val selected = attachment()
            f.begin().selected(listOf(selected))
            val uploaded = if (replaceObject) selected.copy() else selected.also { it.file.appendText("changed") }
            assertNull(f.owner.take(uploaded))
            assertTrue(f.scripts.last().contains("cancelSelection"))
            assertTrue(f.timers.isEmpty())
        }
    }

    @Test fun cancellationEmptyMultiSelectionAndExpiryReleaseTheSlot() {
        for (mode in listOf("cancel", "empty", "multiple", "expired", "timer")) {
            val f = Fixture()
            val selected = attachment()
            val ticket = f.begin()
            when (mode) {
                "cancel" -> ticket.cancel()
                "empty" -> ticket.selected(emptyList())
                "multiple" -> ticket.selected(listOf(selected, attachment()))
                "expired" -> { ticket.selected(listOf(selected)); f.time = 120_000L }
                "timer" -> f.timers.keys.single().run()
            }
            assertNull(f.owner.take(selected))
            assertTrue(f.timers.isEmpty())
        }
    }

    @Test fun lateResultOrCancelFromOldPickerCannotOverwriteReplacement() {
        val f = Fixture()
        val old = f.begin()
        val current = f.begin()
        val selected = attachment()
        old.selected(listOf(attachment()))
        old.cancel()
        current.selected(listOf(selected))
        assertNotNull(f.owner.take(selected))
    }

    @Test fun documentRouteAndAvailabilityChangesInvalidateTheSelection() {
        for (mode in listOf("document", "route", "unavailable")) {
            val f = Fixture()
            val selected = attachment()
            f.begin().selected(listOf(selected))
            when (mode) {
                "document" -> f.state = WebBridgeDocumentSession.Snapshot(2, 2, "doc_fixture_2")
                "route" -> f.url = "https://chatgpt.com/c/other"
                "unavailable" -> f.available = false
            }
            assertNull(f.owner.take(selected))
            assertTrue(f.timers.isEmpty())
        }
    }

    @Test fun backgroundAdapterPauseDoesNotInvalidateTheSameDocumentSelection() {
        val f = Fixture()
        val selected = attachment()
        val ticket = f.begin()
        f.state = f.state.copy(adapterGeneration = 0)
        ticket.selected(listOf(selected))
        assertNotNull(f.owner.take(selected))
        assertNull(f.owner.begin(WebChatAttachmentSelectionKind.DOCUMENT))
    }
}
