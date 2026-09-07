package com.elon.app.chatgptweb

import android.content.ContentUris
import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.MediaStore
import java.io.File
import java.nio.file.Files
import java.nio.file.LinkOption.NOFOLLOW_LINKS
import java.util.concurrent.atomic.AtomicBoolean

internal object ChatGptWebFileByteRecovery {
    private const val DIRECTORY = "chatgpt-pending-downloads-v1"
    private val started = AtomicBoolean()

    fun start(context: Context) {
        if (!started.compareAndSet(false, true)) return
        val app = context.applicationContext
        Thread({ runCatching { recover(app) } }, "chatgpt-file-recovery").apply { isDaemon = true }.start()
    }

    fun journal(context: Context) = ChatGptWebDownloadJournal(File(context.noBackupFilesDir, DIRECTORY))

    fun pendingName(id: String): String {
        require(ChatGptWebDownloadJournal.validId(id))
        return "elon-chatgpt-pending-$id"
    }

    fun pendingFile(context: Context, id: String): File {
        require(ChatGptWebDownloadJournal.validId(id))
        val downloads = checkNotNull(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS))
        val directory = File(downloads, ".$DIRECTORY")
        check(!Files.isSymbolicLink(directory.toPath()))
        return File(directory, "$id.part")
    }

    fun recover(context: Context): ChatGptWebDownloadJournal.Recovery {
        if (!File(context.noBackupFilesDir, DIRECTORY).isDirectory) {
            return ChatGptWebDownloadJournal.Recovery(0, 0, 0)
        }
        return journal(context).recover { id ->
            val mediaClean = Build.VERSION.SDK_INT < Build.VERSION_CODES.Q || discardMedia(context, id)
            // Also handles pending app-external files left before an Android OS upgrade.
            val pending = pendingFile(context, id).toPath()
            val fileClean = when {
                !Files.exists(pending, NOFOLLOW_LINKS) -> true
                !Files.isRegularFile(pending, NOFOLLOW_LINKS) -> false
                else -> Files.deleteIfExists(pending)
            }
            mediaClean && fileClean
        }
    }

    @android.annotation.TargetApi(29)
    @Suppress("DEPRECATION")
    private fun discardMedia(context: Context, id: String): Boolean {
        val resolver = context.contentResolver
        val collection = MediaStore.Downloads.EXTERNAL_CONTENT_URI
        val name = pendingName(id)
        val selection = "${MediaStore.MediaColumns.DISPLAY_NAME} LIKE ? AND ${MediaStore.MediaColumns.IS_PENDING} = ?"
        val arguments = arrayOf("$name%", "1")
        val projection = arrayOf(MediaStore.MediaColumns._ID, MediaStore.MediaColumns.OWNER_PACKAGE_NAME)
        val cursor = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            resolver.query(collection, projection, Bundle().apply {
                putString(android.content.ContentResolver.QUERY_ARG_SQL_SELECTION, selection)
                putStringArray(android.content.ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, arguments)
                putInt(MediaStore.QUERY_ARG_MATCH_PENDING, MediaStore.MATCH_INCLUDE)
            }, null)
        } else resolver.query(MediaStore.setIncludePending(collection), projection, selection, arguments, null)
        val rows = mutableListOf<Uri>()
        checkNotNull(cursor).use {
            while (it.moveToNext()) {
                if (rows.size >= 16 || it.getString(1) != context.packageName) return false
                rows += ContentUris.withAppendedId(collection, it.getLong(0))
            }
        }
        // Never touch published files, even after a crash between publication and journal completion.
        return rows.all { resolver.delete(it, selection, arguments) == 1 }
    }
}
