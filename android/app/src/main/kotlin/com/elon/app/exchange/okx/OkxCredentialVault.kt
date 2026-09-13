package com.elon.app.exchange.okx

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import com.elon.app.privateaccess.StrictJson
import java.io.File
import java.security.KeyStore
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey

internal class OkxSavedAccess(val credentials: OkxCredentials, val account: OkxAccount) {
    override fun toString() = "OkxSavedAccess(private)"
}

/** No plaintext fallback; Android backup never contains this directory or its encryption key. */
internal interface OkxAccessVault {
    fun save(owner: String, access: OkxSavedAccess)
    fun read(owner: String): OkxSavedAccess?
    fun revoke(owner: String)
}

internal class OkxCredentialVault(context: Context) : OkxAccessVault {
    private val directory = File(context.noBackupFilesDir, "okx-global-live-v1")
    private val alias = "yilong.okx.global.live.read.v1"
    private fun file(owner: String): AtomicFile = AtomicFile(File(directory, OkxReadProtocol.digest(owner) + ".bin"))
    private fun aad(owner: String) = (alias + ":" + OkxReadProtocol.digest(owner)).toByteArray(Charsets.UTF_8)
    private fun key(create: Boolean): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { return it }
        check(create)
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    @Synchronized override fun save(owner: String, access: OkxSavedAccess) {
        check(directory.isDirectory || directory.mkdirs())
        val plain = StrictJson.encode(mapOf("schema" to alias, "owner" to OkxReadProtocol.digest(owner),
            "key" to access.credentials.key, "secret" to access.credentials.secret, "passphrase" to access.credentials.passphrase,
            "account" to access.account.reference, "kind" to access.account.kind)).toByteArray(Charsets.UTF_8)
        try {
            val encrypted = OkxVaultCipher.seal(key(true), aad(owner), plain)
            val target = file(owner)
            val stream = target.startWrite()
            try { stream.write(encrypted); target.finishWrite(stream) }
            catch (error: Exception) { target.failWrite(stream); throw error }
        } finally { plain.fill(0) }
    }
    @Synchronized override fun read(owner: String): OkxSavedAccess? {
        val target = file(owner)
        if (!target.baseFile.exists() && !File(target.baseFile.path + ".bak").exists()) return null
        val data = target.openRead().use { it.readBytesBounded(4096) }
        val plain = OkxVaultCipher.open(key(false), aad(owner), data)
        try {
            val row = StrictJson.parse(plain.toString(Charsets.UTF_8), 4096)
            require(row.keys == setOf("schema", "owner", "key", "secret", "passphrase", "account", "kind"))
            require(row["schema"] == alias && row["owner"] == OkxReadProtocol.digest(owner))
            val account = row["account"] as? String ?: error("INVALID_VAULT")
            val kind = row["kind"] as? String ?: error("INVALID_VAULT")
            require(Regex("[a-f0-9]{64}").matches(account) && kind in setOf("primary", "sub"))
            return OkxSavedAccess(OkxCredentials(row["key"] as String, row["secret"] as String, row["passphrase"] as String), OkxAccount(account, kind))
        } finally { plain.fill(0) }
    }
    @Synchronized override fun revoke(owner: String) { file(owner).delete() }
}

internal fun java.io.InputStream.readBytesBounded(limit: Int): ByteArray {
    val out = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(8192)
    while (true) {
        val count = read(buffer)
        if (count < 0) break
        if (out.size() + count > limit) okxFail(OkxReadFailure.RESPONSE_LIMIT)
        out.write(buffer, 0, count)
    }
    return out.toByteArray()
}
