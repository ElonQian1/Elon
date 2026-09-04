package com.elon.app.esk.platform

import android.content.SharedPreferences
import org.junit.Assert.*
import org.junit.Test
import java.lang.reflect.Proxy

internal fun eskPlatformSessionValues(): MutableMap<String, Any?> = mutableMapOf(
    "auth_user_id" to "fixture-user-a", "auth_token" to "fixture-token-a", "auth_expires_at" to 100_000L,
    "auth_account" to "fixture-account", "auth_nickname" to "Fixture name",
    "auth_session_revision" to "00000000-0000-4000-8000-000000000001",
)

class EskPlatformSessionStoreTest {
    private class Preferences {
        var values = eskPlatformSessionValues()
        var reads = 0
        var readFails = false
        var listener: SharedPreferences.OnSharedPreferenceChangeListener? = null
        val prefs = Proxy.newProxyInstance(SharedPreferences::class.java.classLoader,
            arrayOf(SharedPreferences::class.java)) { proxy, method, args ->
            when (method.name) {
                "getAll" -> { reads++; if (readFails) error("fixture read error"); values.toMap() }
                "registerOnSharedPreferenceChangeListener" -> { listener = args!![0] as SharedPreferences.OnSharedPreferenceChangeListener; null }
                "unregisterOnSharedPreferenceChangeListener" -> { assertSame(listener, args!![0]); listener = null; null }
                "toString" -> "FakePreferences"
                "hashCode" -> System.identityHashCode(proxy)
                "equals" -> proxy === args!![0]
                else -> error("Must use one all snapshot, not ${method.name}")
            }
        } as SharedPreferences
    }

    @Test fun captureReadsOneAtomicMapAndNeverIndividualFields() {
        val fake = Preferences()
        val store = EskPlatformSessionStore(fake.prefs) {}
        val captured = requireNotNull(store.capture(1_000L))
        assertEquals(1, fake.reads)
        assertEquals("fixture-user-a", captured.userId)
        assertEquals("Fixture name", captured.displayName)
        fake.values["auth_user_id"] = "fixture-user-b"
        assertEquals("fixture-user-a", captured.userId)
        assertFalse(captured.sameAs(store.capture(1_000L)))
        store.close()
    }

    @Test fun authChangesAndClearInvalidateButOtherSettingsDoNot() {
        val fake = Preferences()
        var count = 0
        val store = EskPlatformSessionStore(fake.prefs) { count++ }
        val listener = requireNotNull(fake.listener)
        listener.onSharedPreferenceChanged(fake.prefs, "theme")
        assertEquals(0, count)
        for (key in listOf("auth_token", "auth_user_id", "auth_nickname", "auth_expires_at", "auth_session_revision", null)) {
            listener.onSharedPreferenceChanged(fake.prefs, key)
        }
        assertEquals(6, count)
        store.close()
        assertNull(fake.listener)
        listener.onSharedPreferenceChanged(fake.prefs, "auth_token")
        assertEquals(6, count)
        assertNull(store.capture(1_000L))
        assertEquals(0, fake.reads)
    }

    @Test fun preferenceReadFailureOrInvalidDataFailsClosed() {
        val fake = Preferences()
        val store = EskPlatformSessionStore(fake.prefs) {}
        fake.readFails = true
        assertNull(store.capture(1_000L))
        fake.readFails = false
        fake.values.remove("auth_token")
        assertNull(store.capture(1_000L))
        store.close()
    }

    @Test fun legacySessionWithoutRevisionOrExpiryRemainsReadableUntilServerAuthentication() {
        val values = eskPlatformSessionValues().apply { remove("auth_session_revision"); remove("auth_expires_at") }
        val session = requireNotNull(EskPlatformSession.fromPreferences(values, 1_000L))
        assertNull(session.revision)
        assertEquals(0L, session.expiresAtMillis)
        assertTrue(session.validAt(Long.MAX_VALUE))
    }

    @Test fun malformedMissingOrExpiredFieldsAreRejectedWithoutTypeCoercion() {
        val invalid = listOf("auth_token" to "", "auth_token" to "x y", "auth_token" to "x\n",
            "auth_token" to "x".repeat(8193), "auth_token" to 1, "auth_user_id" to " ",
            "auth_user_id" to " user", "auth_user_id" to "a\nb", "auth_user_id" to "u".repeat(161),
            "auth_expires_at" to "100000", "auth_expires_at" to 100000, "auth_expires_at" to -1L,
            "auth_expires_at" to 1000L, "auth_expires_at" to 999L,
            "auth_session_revision" to "", "auth_session_revision" to true, "auth_nickname" to 1,
            "auth_account" to "x".repeat(1025), "auth_token" to null)
        for ((key, value) in invalid) {
            val values = eskPlatformSessionValues().apply { this[key] = value }
            assertNull("Invalid field $key", EskPlatformSession.fromPreferences(values, 1_000L))
        }
        assertNull(EskPlatformSession.fromPreferences(eskPlatformSessionValues(), -1L))
        assertNull(EskPlatformSession.fromPreferences(emptyMap<String, Any>(), 1_000L))
    }

    @Test fun labelIsBoundedAndRedactedStringNeverPrintsSessionData() {
        val values = eskPlatformSessionValues().apply { this["auth_nickname"] = "Long\n".repeat(100) }
        val session = requireNotNull(EskPlatformSession.fromPreferences(values, 1_000L))
        assertEquals(64, session.displayName.length)
        assertFalse(session.displayName.contains('\n'))
        assertEquals("EskPlatformSession(redacted)", session.toString())
        assertFalse(session.toString().contains(session.token))
        assertFalse(session.toString().contains(session.userId))
    }
}
