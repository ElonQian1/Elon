package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AuthManagerExpiryTest {
    @Test
    fun parsesServerRfc3339ExpiryIntoEpochMillis() {
        assertEquals(
            1_785_312_000_000L,
            parseServerExpiryEpochMillis("2026-07-29T08:00:00Z")
        )
    }

    @Test
    fun acceptsSecondAndMillisecondEpochExpiryValues() {
        assertEquals(1_785_312_000_000L, parseServerExpiryEpochMillis("1785312000"))
        assertEquals(1_785_312_000_000L, parseServerExpiryEpochMillis("1785312000000"))
    }

    @Test
    fun rejectsMissingOrMalformedExpiryValues() {
        assertNull(parseServerExpiryEpochMillis(""))
        assertNull(parseServerExpiryEpochMillis("not-a-time"))
    }
}
