package com.elon.app.update

import android.content.Context
import android.util.Log
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString
import java.io.File
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * 同WiFi APK 种子管理器
 *
 * 当 APP 启动后，在后台注册本设备为 APK 种子节点。
 * 若同WiFi下有其他用户需要下载更新包，服务器会通过此 WS 连接中继 APK 数据。
 *
 * 工作流：
 *   1. 连接 WS ws://<server>/app/peer/ws?version_code=<当前安装版本>
 *   2. 收到服务器指令 "SEND_APK" 后，把本机安装的 APK 以 64KB 块流式发送
 *   3. 发送完毕后发送 Text "DONE" 通知服务器传输结束
 *   4. WS 断开后自动重连（退避 5 秒）
 *
 * 使用方式（MainActivity.onCreate 中调用一次）：
 *   PeerSeederManager.start(this)
 */
object PeerSeederManager {

    private const val TAG = "PeerSeeder"
    private const val CHUNK_SIZE = 64 * 1024 // 64 KB per WS binary frame
    private const val RECONNECT_DELAY_MS = 5_000L
    private const val SERVER_URL = "ws://43.139.149.158:8080/app/peer/ws"

    private val started = AtomicBoolean(false)

    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)  // 长连接，禁止读超时
        .connectTimeout(15, TimeUnit.SECONDS)
        .build()

    /**
     * 启动后台种子线程（幂等，重复调用无副作用）
     */
    fun start(context: Context) {
        if (!started.compareAndSet(false, true)) return
        val appCtx = context.applicationContext
        Thread({
            seedLoop(appCtx)
        }, "PeerSeeder").apply {
            isDaemon = true
            start()
        }
        Log.i(TAG, "种子服务已启动")
    }

    // ─── 主循环（自动重连） ────────────────────────────────────────────────────

    private fun seedLoop(context: Context) {
        while (true) {
            try {
                connectAndSeed(context)
            } catch (e: Exception) {
                Log.w(TAG, "连接异常，5秒后重连: ${e.message}")
            }
            try {
                Thread.sleep(RECONNECT_DELAY_MS)
            } catch (_: InterruptedException) {
                break
            }
        }
    }

    private fun connectAndSeed(context: Context) {
        val versionCode = getInstalledVersionCode(context)
        val url = "$SERVER_URL?version_code=$versionCode"
        Log.d(TAG, "连接服务器: $url")

        val request = Request.Builder().url(url).build()
        // latch 用于阻塞 connectAndSeed 直到 WS 断开（让重连逻辑在外层处理）
        val closeLatch = java.util.concurrent.CountDownLatch(1)

        val listener = object : WebSocketListener() {
            override fun onOpen(ws: WebSocket, response: Response) {
                Log.i(TAG, "✅ 已注册为种子节点 (versionCode=$versionCode)")
            }

            override fun onMessage(ws: WebSocket, text: String) {
                when (text.trim()) {
                    "SEND_APK" -> {
                        Log.i(TAG, "📤 收到传输指令，开始发送 APK…")
                        sendApk(ws, context)
                    }
                    else -> Log.d(TAG, "服务器消息: $text")
                }
            }

            override fun onMessage(ws: WebSocket, bytes: ByteString) {
                // 服务器不应发来二进制消息，忽略
            }

            override fun onClosing(ws: WebSocket, code: Int, reason: String) {
                ws.close(1000, null)
                Log.i(TAG, "WS 正在关闭: $reason")
            }

            override fun onClosed(ws: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "WS 已关闭: $reason")
                closeLatch.countDown()
            }

            override fun onFailure(ws: WebSocket, t: Throwable, response: Response?) {
                Log.w(TAG, "WS 错误: ${t.message}")
                closeLatch.countDown()
            }
        }

        client.newWebSocket(request, listener)
        closeLatch.await() // 阻塞至连接断开
    }

    // ─── APK 发送 ─────────────────────────────────────────────────────────────

    private fun sendApk(ws: WebSocket, context: Context) {
        val apkFile = getInstalledApkFile(context)
        if (apkFile == null || !apkFile.exists()) {
            Log.e(TAG, "找不到已安装的 APK 文件，无法传输")
            ws.send("DONE") // 避免服务器等待超时
            return
        }

        Log.d(TAG, "APK 路径: ${apkFile.absolutePath}  大小: ${apkFile.length() / 1024} KB")

        try {
            apkFile.inputStream().buffered().use { stream ->
                val buf = ByteArray(CHUNK_SIZE)
                var sent = 0L
                var read: Int
                while (stream.read(buf).also { read = it } != -1) {
                    val chunk = if (read == buf.size) buf else buf.copyOf(read)
                    ws.send(chunk.toByteString())
                    sent += read
                }
                Log.i(TAG, "✅ APK 发送完毕，共 ${sent / 1024} KB，发送 DONE")
            }
            ws.send("DONE")
        } catch (e: Exception) {
            Log.e(TAG, "APK 发送异常: ${e.message}")
            ws.send("DONE") // 让服务器知道传输结束（失败情况下服务器会返回错误）
        }
    }

    // ─── 工具方法 ──────────────────────────────────────────────────────────────

    /** 获取当前已安装应用的 APK 文件路径 */
    private fun getInstalledApkFile(context: Context): File? {
        return try {
            val appInfo = context.packageManager.getApplicationInfo(context.packageName, 0)
            File(appInfo.sourceDir)
        } catch (e: Exception) {
            Log.e(TAG, "获取 APK 路径失败: ${e.message}")
            null
        }
    }

    /** 获取当前已安装应用的 versionCode */
    private fun getInstalledVersionCode(context: Context): Long {
        return try {
            val pi = context.packageManager.getPackageInfo(context.packageName, 0)
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) {
                pi.longVersionCode
            } else {
                @Suppress("DEPRECATION")
                pi.versionCode.toLong()
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取 versionCode 失败: ${e.message}")
            0L
        }
    }
}
