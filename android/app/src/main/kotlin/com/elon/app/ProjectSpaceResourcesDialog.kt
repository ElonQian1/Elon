package com.elon.app

import android.content.Intent
import android.net.Uri
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

/** Manifest links use the same allowed schemes as the project-space API. */
internal fun showProjectSpaceResources(activity: AppCompatActivity, space: ProjectSpace?, serverUrl: String) {
    val links = space?.introduction?.resources.orEmpty()
    if (links.isEmpty()) {
        AlertDialog.Builder(activity).setTitle("资料与版本")
            .setMessage("项目尚未配置资料或版本链接，可从项目文档和构建与交付频道查看已有内容。")
            .setPositiveButton("知道了", null).show()
        return
    }
    AlertDialog.Builder(activity).setTitle("资料与版本")
        .setItems(links.map { it.label }.toTypedArray()) { _, index ->
            val value = links[index].url
            val url = if (value.startsWith("/")) serverUrl.trimEnd('/') + value else value
            runCatching { activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }.onFailure {
                android.widget.Toast.makeText(activity, "无法打开链接，请检查浏览器是否可用。", android.widget.Toast.LENGTH_SHORT).show()
            }
        }.setNegativeButton("关闭", null).show()
}
