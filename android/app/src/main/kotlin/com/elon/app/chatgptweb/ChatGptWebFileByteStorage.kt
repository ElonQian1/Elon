package com.elon.app.chatgptweb

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import androidx.core.content.FileProvider
import java.io.File

internal object ChatGptWebFileByteStorage {
    fun open(context: Context, lease: ChatGptWebFileDownloadLease.Value, onSaved: (Uri?) -> Unit): ChatGptWebFileByteDestination {
        val name = "elon-${lease.id}-${lease.name}"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val resolver = context.contentResolver
            val values = ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, name)
                put(MediaStore.Downloads.MIME_TYPE, lease.mediaType.ifBlank { "application/octet-stream" })
                put(MediaStore.Downloads.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
                put(MediaStore.Downloads.IS_PENDING, 1)
            }
            val uri = checkNotNull(resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values))
            val output = try { checkNotNull(resolver.openOutputStream(uri, "w")) } catch (failure: Exception) {
                runCatching { resolver.delete(uri, null, null) }
                throw failure
            }
            return object : ChatGptWebFileByteDestination {
                override fun write(bytes: ByteArray) { output.write(bytes) }
                override fun publish() {
                    output.close()
                    check(resolver.update(uri, ContentValues().apply { put(MediaStore.Downloads.IS_PENDING, 0) }, null, null) == 1)
                    onSaved(uri)
                }
                override fun discard() {
                    runCatching { output.close() }
                    resolver.delete(uri, null, null)
                }
            }
        }
        val directory = checkNotNull(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS))
        val pending = File(directory, "$name.part")
        val destination = File(directory, name)
        check(!destination.exists() && pending.createNewFile())
        val output = try { pending.outputStream() } catch (failure: Exception) {
            pending.delete()
            throw failure
        }
        return object : ChatGptWebFileByteDestination {
            override fun write(bytes: ByteArray) { output.write(bytes) }
            override fun publish() {
                output.close()
                check(!destination.exists() && pending.renameTo(destination))
                onSaved(runCatching { FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", destination) }.getOrNull())
            }
            override fun discard() {
                runCatching { output.close() }
                pending.delete()
            }
        }
    }
}
