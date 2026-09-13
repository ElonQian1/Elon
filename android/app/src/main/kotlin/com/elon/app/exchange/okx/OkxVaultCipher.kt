package com.elon.app.exchange.okx

import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Cipher format is independent of Android storage, so tamper/owner isolation can be tested directly. */
internal object OkxVaultCipher {
    fun seal(key: SecretKey, aad: ByteArray, plain: ByteArray): ByteArray {
        require(plain.size <= 4000)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key); updateAAD(aad) }
        val encrypted = cipher.doFinal(plain)
        check(cipher.iv.size == 12)
        return byteArrayOf(1) + cipher.iv + encrypted
    }
    fun open(key: SecretKey, aad: ByteArray, data: ByteArray): ByteArray {
        require(data.size in 30..4096 && data[0] == 1.toByte())
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, data.copyOfRange(1, 13))); updateAAD(aad)
        }
        return cipher.doFinal(data.copyOfRange(13, data.size))
    }
}
