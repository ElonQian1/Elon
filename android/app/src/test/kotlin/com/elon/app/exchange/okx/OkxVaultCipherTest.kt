package com.elon.app.exchange.okx

import org.junit.Assert.*
import org.junit.Test
import javax.crypto.KeyGenerator

class OkxVaultCipherTest {
    private fun key() = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
    @Test fun encryptedValuesRoundTripWithDifferentNonceOnEveryWrite() {
        val key = key(); val aad = "test-owner:global:live".toByteArray(); val plain = "test-only-credential".toByteArray()
        val a = OkxVaultCipher.seal(key, aad, plain); val b = OkxVaultCipher.seal(key, aad, plain)
        assertFalse(a.contentEquals(b)); assertFalse(a.toString(Charsets.UTF_8).contains("test-only-credential"))
        assertArrayEquals(plain, OkxVaultCipher.open(key, aad, a))
    }
    @Test fun wrongOwnerDomainKeyAndTamperingCannotDecrypt() {
        val key = key(); val aad = "alice:global:live".toByteArray(); val encrypted = OkxVaultCipher.seal(key, aad, "test-only".toByteArray())
        for (wrong in listOf("bob:global:live", "alice:global:demo"))
            assertThrows(java.security.GeneralSecurityException::class.java) { OkxVaultCipher.open(key, wrong.toByteArray(), encrypted) }
        assertThrows(java.security.GeneralSecurityException::class.java) { OkxVaultCipher.open(key(), aad, encrypted) }
        val changed = encrypted.copyOf().apply { this[lastIndex] = (this[lastIndex].toInt() xor 1).toByte() }
        assertThrows(java.security.GeneralSecurityException::class.java) { OkxVaultCipher.open(key, aad, changed) }
        assertThrows(IllegalArgumentException::class.java) { OkxVaultCipher.open(key, aad, encrypted.copyOf(12)) }
    }
}
