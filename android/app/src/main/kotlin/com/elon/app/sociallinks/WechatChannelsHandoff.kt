package com.elon.app.sociallinks

import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.Lifecycle
import org.json.JSONObject
import java.util.UUID

/** No exported JS bridge, arbitrary Intent parsing, login transfer, or fake WeChat user agent. */
internal class WechatChannelsHandoff(
    private val activity: AppCompatActivity,
    private val status: (String) -> Unit,
) {
    private val handler = Handler(Looper.getMainLooper())
    private var generation = 0
    private var busy = false
    private var dialog: AlertDialog? = null
    private val adapter by lazy { activity.assets.open("wechat_channels_handoff.js").bufferedReader().use { it.readText() } }

    private fun foreground() = !activity.isFinishing && !activity.isDestroyed && activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)

    fun confirmPageLink(web: WebView, target: String) {
        val source = web.url.orEmpty()
        if (!foreground() || dialog != null || !WechatChannelsPolicy.allows(source, target)) return
        dialog = AlertDialog.Builder(activity).setMessage("在微信中打开此视频号内容？")
            .setNegativeButton("取消", null)
            .setPositiveButton("打开微信") { _, _ ->
                if (foreground() && web.url == source) launch(source, target)
            }.create().also {
                it.setOnDismissListener { dialog = null }; it.show()
            }
    }

    fun openCurrent(web: WebView) {
        if (busy || !foreground()) return
        val source = web.url.orEmpty()
        if (!WechatChannelsPolicy.isPreview(source)) { status("请等待视频号预览加载完成后重试。"); return }
        val nonce = UUID.randomUUID().toString()
        val mine = ++generation
        busy = true
        status("正在获取微信跳转链接…")
        val deadline = android.os.SystemClock.elapsedRealtime() + 8500
        fun alive() = mine == generation && foreground() && source == web.url
        fun finish(message: String) { if (mine == generation) { busy = false; status(message) } }
        fun poll(attempt: Int) {
            if (!alive()) { if (mine == generation) busy = false; return }
            if (android.os.SystemClock.elapsedRealtime() > deadline || attempt > 35) { finish("获取微信跳转超时，请重试。"); return }
            web.evaluateJavascript("window.ElonWechatChannelsHandoff.read(${JSONObject.quote(nonce)})") { encoded ->
                if (!alive()) { if (mine == generation) busy = false; return@evaluateJavascript }
                val value = runCatching { JSONObject(encoded) }.getOrNull()
                when (value?.optString("status")) {
                    "pending" -> handler.postDelayed({ poll(attempt + 1) }, 200)
                    "ready" -> {
                        busy = false
                        if (value.optString("nonce") == nonce && value.optString("source") == source && value.optLong("expiresAt") > System.currentTimeMillis() + 1000) {
                            launch(source, value.optString("url"))
                        } else finish("跳转链接已失效，请重试。")
                    }
                    else -> finish("暂未取得微信跳转链接，请刷新页面后重试。")
                }
            }
        }
        handler.postDelayed({ if (mine == generation && busy) { ++generation; busy = false; status("获取微信跳转超时，请重试。") } }, 9000)
        web.evaluateJavascript("$adapter.start(${JSONObject.quote(source)},${JSONObject.quote(nonce)})") { if (alive()) poll(0) else if (mine == generation) busy = false }
    }

    private fun launch(source: String, target: String) {
        if (!WechatChannelsPolicy.allows(source, target)) { status("不支持的视频号跳转格式。"); return }
        runCatching { activity.startActivity(intent(target)) }
            .onSuccess { status("已请求微信打开，返回后可继续阅读。") }
            .onFailure { status("未能调用微信，请安装或更新微信后重试，也可在微信中打开原链接。") }
    }

    fun cancel() { ++generation; busy = false; handler.removeCallbacksAndMessages(null); dialog?.dismiss(); dialog = null }

    companion object {
        internal fun intent(target: String) = Intent(Intent.ACTION_VIEW, Uri.parse(target))
            .addCategory(Intent.CATEGORY_BROWSABLE).setPackage("com.tencent.mm")
    }
}
