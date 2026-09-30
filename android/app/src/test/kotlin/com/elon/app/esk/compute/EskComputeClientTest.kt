package com.elon.app.esk.compute

import org.junit.Assert.*
import org.junit.Test

class EskComputeClientTest {
    @Test fun insecureSourcesNeverReadCredentialsOrDispatch() {
        for (base in listOf("http://example.test", "https://user:secret@example.test", "https://example.test/path", "https://example.test?x=1")) {
            var credentialRead = false
            val reader = EskComputeClient(okhttp3.Call.Factory { fail("Must not dispatch"); error("unreachable") })
            try { reader.fetch(base, 1) { credentialRead = true; "synthetic" }; fail("Expected rejection") } catch (_: java.io.IOException) { }
            assertFalse(credentialRead)
        }
    }
    @Test fun canceledReaderCannotDispatchOrReadCredentials() {
        var credentialRead = false
        val reader = EskComputeClient(okhttp3.Call.Factory { fail("Must not dispatch"); error("unreachable") })
        reader.cancel()
        try { reader.fetch("https://example.test", 1) { credentialRead = true; "synthetic" }; fail("Expected rejection") } catch (_: java.io.IOException) { }
        assertFalse(credentialRead)
    }
}
