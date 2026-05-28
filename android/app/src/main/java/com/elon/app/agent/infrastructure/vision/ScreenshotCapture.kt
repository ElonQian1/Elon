// infrastructure/vision/ScreenshotCapture.kt
// module: infrastructure/vision | layer: infrastructure | role: screenshot-capture
// summary: 屏幕截图捕获服务，支持多种截图方式

package com.elon.app.agent.infrastructure.vision

import android.accessibilityservice.AccessibilityService
import android.content.Context
import android.graphics.Bitmap
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.Image
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Base64
import android.util.DisplayMetrics
import android.util.Log
import android.view.WindowManager
import androidx.annotation.RequiresApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * 屏幕截图捕获器
 * 
 * 支持两种模式：
 * 1. AccessibilityService.takeScreenshot (Android 11+, 推荐)
 * 2. MediaProjection (需要用户授权，兼容旧版)
 */
class ScreenshotCapture(
    private val context: Context,
    private val accessibilityService: AccessibilityService? = null
) {
    companion object {
        private const val TAG = "ScreenshotCapture"
        private const val JPEG_QUALITY = 80
        private const val MAX_WIDTH = 1080  // 限制宽度，节省传输
    }
    
    private var mediaProjection: MediaProjection? = null
    private var virtualDisplay: VirtualDisplay? = null
    private var imageReader: ImageReader? = null
    
    /**
     * 截取屏幕并返回 Base64 编码的图片
     */
    suspend fun captureScreenBase64(): ScreenshotResult = withContext(Dispatchers.IO) {
        try {
            val bitmap = captureScreen()
            val resized = resizeBitmap(bitmap)
            val base64 = bitmapToBase64(resized)
            
            if (bitmap != resized) {
                bitmap.recycle()
            }
            resized.recycle()
            
            ScreenshotResult.Success(
                base64 = base64,
                width = resized.width,
                height = resized.height
            )
        } catch (e: Exception) {
            Log.e(TAG, "截图失败", e)
            ScreenshotResult.Failure(e.message ?: "未知错误")
        }
    }
    
    /**
     * 截取屏幕并返回 Bitmap
     */
    suspend fun captureScreen(): Bitmap {
        // Android 11+ 优先使用 AccessibilityService.takeScreenshot
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && accessibilityService != null) {
            return captureViaAccessibility()
        }
        
        // 回退到 MediaProjection
        return captureViaMediaProjection()
    }
    
    /**
     * 通过 AccessibilityService 截图 (Android 11+)
     * 优点：无需用户授权，简单可靠
     */
    @RequiresApi(Build.VERSION_CODES.R)
    private suspend fun captureViaAccessibility(): Bitmap = suspendCancellableCoroutine { cont ->
        accessibilityService?.takeScreenshot(
            0, // Display ID (0 = 默认屏幕)
            context.mainExecutor,
            object : AccessibilityService.TakeScreenshotCallback {
                override fun onSuccess(screenshot: AccessibilityService.ScreenshotResult) {
                    val bitmap = Bitmap.wrapHardwareBuffer(
                        screenshot.hardwareBuffer,
                        screenshot.colorSpace
                    )
                    if (bitmap != null) {
                        // 转换为可变 Bitmap（硬件 Bitmap 不能直接操作）
                        val softwareBitmap = bitmap.copy(Bitmap.Config.ARGB_8888, false)
                        bitmap.recycle()
                        screenshot.hardwareBuffer.close()
                        cont.resume(softwareBitmap)
                    } else {
                        cont.resumeWithException(Exception("Bitmap 创建失败"))
                    }
                }
                
                override fun onFailure(errorCode: Int) {
                    cont.resumeWithException(Exception("截图失败，错误码: $errorCode"))
                }
            }
        ) ?: cont.resumeWithException(Exception("AccessibilityService 不可用"))
    }
    
    /**
     * 通过 MediaProjection 截图
     * 需要先调用 setMediaProjection 设置投影
     */
    private suspend fun captureViaMediaProjection(): Bitmap = suspendCancellableCoroutine { cont ->
        val projection = mediaProjection
        if (projection == null) {
            cont.resumeWithException(Exception("MediaProjection 未设置，请先请求屏幕录制权限"))
            return@suspendCancellableCoroutine
        }
        
        val metrics = getDisplayMetrics()
        val width = metrics.widthPixels
        val height = metrics.heightPixels
        val density = metrics.densityDpi
        
        imageReader = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 2)
        
        virtualDisplay = projection.createVirtualDisplay(
            "ScreenCapture",
            width, height, density,
            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
            imageReader!!.surface,
            null, null
        )
        
        Handler(Looper.getMainLooper()).postDelayed({
            try {
                val image = imageReader?.acquireLatestImage()
                if (image != null) {
                    val bitmap = imageToBitmap(image)
                    image.close()
                    cont.resume(bitmap)
                } else {
                    cont.resumeWithException(Exception("获取图像失败"))
                }
            } catch (e: Exception) {
                cont.resumeWithException(e)
            } finally {
                cleanup()
            }
        }, 100) // 等待画面渲染
    }
    
    /**
     * 设置 MediaProjection（从 Activity 获取）
     */
    fun setMediaProjection(projection: MediaProjection) {
        this.mediaProjection = projection
    }
    
    private fun imageToBitmap(image: Image): Bitmap {
        val planes = image.planes
        val buffer = planes[0].buffer
        val pixelStride = planes[0].pixelStride
        val rowStride = planes[0].rowStride
        val rowPadding = rowStride - pixelStride * image.width
        
        val bitmap = Bitmap.createBitmap(
            image.width + rowPadding / pixelStride,
            image.height,
            Bitmap.Config.ARGB_8888
        )
        bitmap.copyPixelsFromBuffer(buffer)
        
        // 裁剪掉 padding
        return if (rowPadding > 0) {
            Bitmap.createBitmap(bitmap, 0, 0, image.width, image.height).also {
                bitmap.recycle()
            }
        } else {
            bitmap
        }
    }
    
    private fun resizeBitmap(bitmap: Bitmap): Bitmap {
        if (bitmap.width <= MAX_WIDTH) return bitmap
        
        val ratio = MAX_WIDTH.toFloat() / bitmap.width
        val newHeight = (bitmap.height * ratio).toInt()
        return Bitmap.createScaledBitmap(bitmap, MAX_WIDTH, newHeight, true)
    }
    
    private fun bitmapToBase64(bitmap: Bitmap): String {
        val stream = ByteArrayOutputStream()
        bitmap.compress(Bitmap.CompressFormat.JPEG, JPEG_QUALITY, stream)
        val bytes = stream.toByteArray()
        return Base64.encodeToString(bytes, Base64.NO_WRAP)
    }
    
    private fun getDisplayMetrics(): DisplayMetrics {
        val metrics = DisplayMetrics()
        val windowManager = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
        @Suppress("DEPRECATION")
        windowManager.defaultDisplay.getRealMetrics(metrics)
        return metrics
    }
    
    private fun cleanup() {
        virtualDisplay?.release()
        virtualDisplay = null
        imageReader?.close()
        imageReader = null
    }
    
    fun release() {
        cleanup()
        mediaProjection?.stop()
        mediaProjection = null
    }
}

/**
 * 截图结果
 */
sealed class ScreenshotResult {
    data class Success(
        val base64: String,
        val width: Int,
        val height: Int
    ) : ScreenshotResult()
    
    data class Failure(val error: String) : ScreenshotResult()
}
