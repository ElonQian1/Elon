package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptWebFileDownloadPolicyTest {
    @Test fun onlyDedicatedHttpsFileOriginsAreAcceptedWithoutCredentialHeaders() {
        assertNotNull(ChatGptWebFileDownloadPolicy.signedUrl("https://files.oaiusercontent.com/a?sig=synthetic"))
        for (value in listOf("http://files.oaiusercontent.com/a", "https://oaiusercontent.com.evil.test/a",
            "https://user:secret@files.oaiusercontent.com/a", "https://files.oaiusercontent.com:8443/a",
            "https://files.oaiusercontent.com/a#fragment", "https://chatgpt.com/backend-api/private",
            "https://127.0.0.1/a", "file:///private", "https://files.oaiusercontent.com/\na")) {
            assertNull(value, ChatGptWebFileDownloadPolicy.signedUrl(value))
        }
        assertNull(ChatGptWebFileDownloadPolicy.signedUrl("https://files.oaiusercontent.com/" + "a".repeat(16384)))
    }

    @Test fun aDownloadLeaseIsOneUseAndCannotBeReplacedInTheSameContextWhilePending() {
        val leases = ChatGptWebFileDownloadLease()
        val first = requireNotNull(leases.begin("doc_a", 1, "https://chatgpt.com/c/a", "a.txt", "text/plain", 1_000))
        assertNull(leases.begin(first.token, first.generation, first.href, "b.txt", "text/plain", 2_000))
        assertNull(leases.consume(first.id, first.token, first.generation, "https://chatgpt.com/c/b", 2_000))
        assertNotNull(leases.consume(first.id, first.token, first.generation, first.href, 2_000))
        assertNull(leases.consume(first.id, first.token, first.generation, first.href, 2_000))
        assertNotNull(leases.begin(first.token, first.generation, first.href, "retry.txt", "text/plain", 2_001))
    }

    @Test fun replacingTheDocumentOrRouteDoesNotBlockANewUserDownloadForTheOldLeaseTtl() {
        val leases = ChatGptWebFileDownloadLease()
        val first = requireNotNull(leases.begin("doc_a", 1, "https://chatgpt.com/c/a", "a.txt", "text/plain", 1_000))
        val next = requireNotNull(leases.begin("doc_b", 2, "https://chatgpt.com/c/b", "b.txt", "text/plain", 1_001))
        assertNull(leases.consume(first.id, first.token, first.generation, first.href, 1_002))
        assertNotNull(leases.consume(next.id, next.token, next.generation, next.href, 1_002))
    }

    @Test fun documentReplacementExpiryAndCancellationRejectLateDownloads() {
        val leases = ChatGptWebFileDownloadLease()
        val first = requireNotNull(leases.begin("doc_a", 1, "https://chatgpt.com/", "a.txt", "text/plain", 0))
        assertNull(leases.consume(first.id, "doc_b", 1, first.href, 1))
        assertNull(leases.consume(first.id, first.token, 2, first.href, 1))
        assertNull(leases.consume(first.id, first.token, 1, first.href, 25_000))
        val second = requireNotNull(leases.begin("doc_a", 1, first.href, "b.txt", "text/plain", 25_000))
        leases.cancel()
        assertNull(leases.consume(second.id, second.token, 1, second.href, 25_001))
    }

    @Test fun filenamesCannotEscapeTheDestinationAndOpaqueHandlesAreValidated() {
        val name = ChatGptWebFileDownloadPolicy.safeName("../../a\\b:\u0000.txt")
        assertFalse(name.contains('/'))
        assertFalse(name.contains('\\'))
        assertFalse(name.contains(':'))
        assertFalse(name.contains('\u0000'))
        assertEquals("download.bin", ChatGptWebFileDownloadPolicy.safeName(". ."))
        assertEquals(150, ChatGptWebFileDownloadPolicy.safeName("a".repeat(200)).length)
        assertTrue(ChatGptWebFileDownloadPolicy.HANDLE.matches("download_" + "a".repeat(32)))
        assertFalse(ChatGptWebFileDownloadPolicy.HANDLE.matches("https://files.oaiusercontent.com/a?sig=private"))
    }
}
