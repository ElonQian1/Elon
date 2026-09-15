package com.elon.app.sharing

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.PendingAttachment
import java.util.concurrent.Executors

internal object PendingSourceLinks {
    private val executor = Executors.newSingleThreadExecutor()
    fun inspect(activity: AppCompatActivity, files: List<PendingAttachment>) {
        files.filter { it.mimeType.startsWith("image/") && !it.sourceLinkChecked }.forEach { file ->
            executor.execute {
                val links = ImageQrLinks.decode(file.file)
                activity.runOnUiThread {
                    if (activity.isFinishing || activity.isDestroyed || !file.file.exists()) return@runOnUiThread
                    if (links.size <= 1) { file.sourceLink = links.singleOrNull(); file.sourceLinkChecked = true }
                    else AlertDialog.Builder(activity).setTitle("选择图片原文链接")
                        .setItems((links.map { it.url } + "不附加链接").toTypedArray()) { _, index -> file.sourceLink = links.getOrNull(index); file.sourceLinkChecked = true }
                        .setOnCancelListener { file.sourceLinkChecked = true }.show()
                }
            }
        }
    }
}
