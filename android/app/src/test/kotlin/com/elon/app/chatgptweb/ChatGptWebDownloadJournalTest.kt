package com.elon.app.chatgptweb

import java.io.File
import java.util.UUID
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class ChatGptWebDownloadJournalTest {
    @get:Rule val temporary = TemporaryFolder()
    private fun id() = UUID.randomUUID().toString()
    private fun journal() = ChatGptWebDownloadJournal(temporary.root)
    private fun markers() = temporary.root.listFiles()!!.filter { it.name.endsWith(".pending") }

    @Test fun interruptedOwnerIsRecoveredByANewJournalInstance() {
        val id = id()
        journal().begin(id).close()
        val visited = mutableListOf<String>()
        val result = journal().recover { visited += it; true }
        assertEquals(listOf(id), visited)
        assertEquals(ChatGptWebDownloadJournal.Recovery(1, 0, 0), result)
        assertTrue(markers().isEmpty())
    }

    @Test fun activeOwnerSurvivesRecoveryInAnotherInstance() {
        journal().begin(id()).use { owner ->
            assertEquals(ChatGptWebDownloadJournal.Recovery(0, 1, 0), journal().recover { error("active download") })
            assertEquals(1, markers().size)
            owner.complete()
        }
        assertEquals(ChatGptWebDownloadJournal.Recovery(0, 0, 0), journal().recover { error("completed") })
    }

    @Test fun failedCleanupKeepsMarkerAndCanBeRetried() {
        journal().begin(id()).close()
        assertEquals(1, journal().recover { false }.failed)
        assertEquals(1, journal().recover { throw java.io.IOException("storage offline") }.failed)
        assertEquals(1, markers().size)
        assertEquals(1, journal().recover { true }.cleaned)
    }

    @Test fun successfulPublicationBeforeJournalCompletionPreservesSavedFile() {
        val destination = temporary.newFile("completed-download.txt").apply { writeText("saved") }
        val owner = journal().begin(id())
        val pending = temporary.newFile("staging.part")
        assertTrue(pending.delete()) // Publication removed the pending resource, then the process exited.
        owner.close()
        assertEquals(1, journal().recover { !pending.exists() || pending.delete() }.cleaned)
        assertEquals("saved", destination.readText())
    }

    @Test fun markerIsPresentAndLockedBeforeAnyDestinationIsCreated() {
        val id = id()
        journal().begin(id).use {
            assertEquals("$id.pending", markers().single().name)
            assertEquals(0L, markers().single().length())
            assertEquals(1, journal().recover { error("not orphaned") }.active)
        }
    }

    @Test fun invalidIdsAndDuplicateOwnersCannotOverwriteTheMarker() {
        for (id in listOf("../escape", "", "token", "A".repeat(36))) {
            assertTrue(runCatching { journal().begin(id) }.isFailure)
        }
        val id = id()
        journal().begin(id).use {
            assertTrue(runCatching { journal().begin(id) }.isFailure)
            assertEquals(1, journal().recover { false }.active)
        }
        assertTrue(runCatching { journal().begin(id) }.isFailure)
    }

    @Test fun unknownFilesAndDirectoriesAreNeverRecoveredOrDeleted() {
        val unknown = temporary.newFile("notes.pending")
        val directory = temporary.newFolder("${id()}.pending")
        assertEquals(0, journal().recover { error("unknown artifact") }.cleaned)
        assertTrue(unknown.exists())
        assertTrue(directory.exists())
    }

    @Test fun cleanupIsBoundedAndSuccessfulMarkersLeaveTheNextBatch() {
        repeat(3) { journal().begin(id()).close() }
        assertEquals(2, journal().recover(limit = 2) { true }.cleaned)
        assertEquals(1, markers().size)
        assertEquals(1, journal().recover { true }.cleaned)
    }

    @Test fun abandonedHandleCannotDeleteAReplacementRecoveryOwner() {
        val owner = journal().begin(id())
        owner.close()
        journal().recover {
            owner.complete()
            assertEquals(1, markers().size)
            true
        }
        assertTrue(markers().isEmpty())
    }

    @Test fun storageCallbackDoesNotHoldTheGlobalJournalGuard() {
        journal().begin(id()).close()
        journal().recover {
            val failure = AtomicReference<Throwable?>()
            val next = Thread {
                runCatching { journal().begin(id()).use { owner -> owner.complete() } }.onFailure(failure::set)
            }
            next.start()
            next.join(2_000)
            assertFalse(next.isAlive)
            assertNull(failure.get())
            true
        }
    }

    @Test fun osReleasesTheDownloadLockAfterARealProcessIsKilled() {
        val child = ProcessBuilder(File(System.getProperty("java.home"), "bin/java").path,
            "-cp", System.getProperty("java.class.path"), javaClass.name, temporary.root.path, id()).start()
        try {
            val ready = File(temporary.root, "child.ready")
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10)
            while (!ready.exists() && child.isAlive && System.nanoTime() < deadline) Thread.sleep(20)
            assertTrue("child download owner started", ready.exists())
            assertEquals(1, journal().recover { error("other process is still downloading") }.active)
            child.destroyForcibly()
            assertTrue(child.waitFor(5, TimeUnit.SECONDS))
            // Windows may signal process exit before pending handle cleanup is observable.
            val cleanupDeadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
            var recovered = journal().recover { true }
            while (recovered.active > 0 && System.nanoTime() < cleanupDeadline) {
                Thread.sleep(20)
                recovered = journal().recover { true }
            }
            assertEquals(recovered.toString(), 1, recovered.cleaned)
            assertTrue(markers().isEmpty())
        } finally {
            child.destroyForcibly()
            child.waitFor(5, TimeUnit.SECONDS)
        }
    }

    companion object {
        @JvmStatic fun main(args: Array<String>) {
            val directory = File(args[0])
            ChatGptWebDownloadJournal(directory).begin(args[1]).use {
                File(directory, "child.ready").createNewFile()
                Thread.sleep(30_000)
            }
        }
    }
}
