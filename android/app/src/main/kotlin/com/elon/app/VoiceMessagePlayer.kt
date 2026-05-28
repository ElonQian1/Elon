package com.elon.app

import android.media.MediaPlayer
import android.os.Handler
import android.os.Looper
import android.util.Log

/**
 * 全局语音消息播放器（单例）。
 *
 * 同一时刻只允许播放一条语音，点击另一条自动停止当前播放。
 * 通过 [onStateChanged] 回调通知外部更新 UI。
 */
object VoiceMessagePlayer {

    private const val TAG = "VoiceMessagePlayer"
    private val main = Handler(Looper.getMainLooper())

    private var player: MediaPlayer? = null
    private var currentSource: String? = null
    private var completionAction: (() -> Unit)? = null

    private var progressRunnable: Runnable? = null

    // 多监听器列表，避免多个语音气泡互相覆盖单个 onStateChanged
    private val listeners = mutableListOf<(String, Boolean, Int, Int) -> Unit>()

    fun addStateListener(listener: (source: String, isPlaying: Boolean, posMs: Int, durMs: Int) -> Unit) {
        listeners.add(listener)
    }

    fun removeStateListener(listener: (source: String, isPlaying: Boolean, posMs: Int, durMs: Int) -> Unit) {
        listeners.remove(listener)
    }

    private fun notifyListeners(source: String, isPlaying: Boolean, posMs: Int, durMs: Int) {
        listeners.forEach { it(source, isPlaying, posMs, durMs) }
    }

    /**
     * 兼容旧代码的单回调属性（设置后追加到 listeners 中）。
     * @deprecated 推荐使用 [addStateListener] / [removeStateListener]
     */
    var onStateChanged: ((source: String, isPlaying: Boolean, positionMs: Int, durationMs: Int) -> Unit)? = null

    val isPlaying: Boolean get() = player?.isPlaying == true

    fun currentSource(): String? = currentSource

    fun isCurrentlyPlaying(source: String): Boolean {
        return currentSource == source && player?.isPlaying == true
    }

    /**
     * 播放或暂停指定来源（URL 或本地文件路径）。
     * @param onComplete 播放自然结束后的回调（用于重置 UI）
     */
    fun playOrPause(source: String, onComplete: () -> Unit = {}) {
        if (currentSource == source) {
            val p = player ?: return
            if (p.isPlaying) {
                p.pause()
                stopProgressUpdates()
                dispatchState(source, false, p.currentPosition, p.duration)
            } else {
                p.start()
                dispatchState(source, true, p.currentPosition, p.duration)
                startProgressUpdates(source)
            }
            return
        }

        // 停止当前播放的那条，调用它的 completionAction 重置其 UI
        stopCurrent()

        // 记录新的 completion 回调
        completionAction = onComplete
        startNew(source)
    }

    /** 停止当前播放，并调用对应的 completionAction 重置 UI。 */
    fun stopCurrent() {
        stopProgressUpdates()
        val old = player
        val oldSource = currentSource
        val cb = completionAction
        player = null
        currentSource = null
        completionAction = null
        if (old != null) {
            runCatching { if (old.isPlaying) old.stop() }
            runCatching { old.release() }
            if (oldSource != null) {
                dispatchState(oldSource, false, 0, 0)
            }
        }
        cb?.invoke()
    }

    /** 释放所有资源（Activity onDestroy 时调用）。 */
    fun release() {
        stopProgressUpdates()
        player?.let {
            runCatching { it.stop() }
            runCatching { it.release() }
        }
        player = null
        currentSource = null
        completionAction = null
        listeners.clear()
    }

    // ─────────────────────────────────────────────

    private fun startNew(source: String) {
        val mp = MediaPlayer()
        try {
            mp.setDataSource(source)
            mp.setOnPreparedListener { p ->
                currentSource = source
                player = p
                p.start()
                dispatchState(source, true, 0, p.duration)
                startProgressUpdates(source)
            }
            mp.setOnCompletionListener { p ->
                stopProgressUpdates()
                val cb = completionAction
                currentSource = null
                player = null
                completionAction = null
                dispatchState(source, false, p.duration, p.duration)
                p.release()
                cb?.invoke()
            }
            mp.setOnErrorListener { p, what, extra ->
                Log.e(TAG, "播放错误 what=$what extra=$extra")
                stopProgressUpdates()
                val cb = completionAction
                currentSource = null
                player = null
                completionAction = null
                dispatchState(source, false, 0, 0)
                p.release()
                cb?.invoke()
                false
            }
            mp.prepareAsync()
        } catch (e: Exception) {
            Log.e(TAG, "播放初始化失败: ${e.message}")
            runCatching { mp.release() }
            completionAction?.invoke()
            completionAction = null
        }
    }

    private fun startProgressUpdates(source: String) {
        stopProgressUpdates()
        val r = object : Runnable {
            override fun run() {
                val p = player ?: return
                if (!p.isPlaying) return
                dispatchState(source, true, p.currentPosition, p.duration)
                main.postDelayed(this, 100L)
            }
        }
        progressRunnable = r
        main.postDelayed(r, 100L)
    }

    private fun stopProgressUpdates() {
        progressRunnable?.let { main.removeCallbacks(it) }
        progressRunnable = null
    }

    private fun dispatchState(source: String, isPlaying: Boolean, posMs: Int, durMs: Int) {
        notifyListeners(source, isPlaying, posMs, durMs)
        onStateChanged?.invoke(source, isPlaying, posMs, durMs)
    }
}
