package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.util.Base64
import android.util.LruCache
import java.io.ByteArrayOutputStream

internal data class UserProfile(
    val displayName: String,
    val account: String?,
    val phone: String?,
    val wechatId: String,
    val signature: String,
    val avatarDataUrl: String?
)

internal object UserProfileStore {
    // 头像 Bitmap 内存缓存：key = dataUrl hashCode，最多 4MB（约 50 张 320x320 头像）
    private val avatarCache = object : LruCache<Int, Bitmap>(4 * 1024 * 1024) {
        override fun sizeOf(key: Int, value: Bitmap) = value.byteCount
    }

    private const val KEY_NAME = "profile_display_name"
    private const val KEY_SIGNATURE = "profile_signature"
    private const val KEY_AVATAR_DATA_URL = "profile_avatar_data_url"
    private const val MAX_AVATAR_SIZE = 320

    fun load(context: Context): UserProfile {
        val prefs = AuthManager.userDataPrefs(context)
        val account = AuthManager.account(context)
        val storedName = prefs.getString(KEY_NAME, null)?.trim().takeUnless { it.isNullOrBlank() }
        val fallbackName = AuthManager.nickname(context)
            ?: account
            ?: if (AuthManager.isLoggedIn(context)) "一龙用户" else "游客用户"
        val userId = AuthManager.effectiveUserId(context)
        val phone = account?.takeIf { it.all(Char::isDigit) }
        return UserProfile(
            displayName = storedName ?: fallbackName,
            account = account,
            phone = phone,
            wechatId = account ?: "elon_${userId.take(8)}",
            signature = prefs.getString(KEY_SIGNATURE, null)?.takeIf { it.isNotBlank() } ?: "用一龙把想法做成 App",
            avatarDataUrl = prefs.getString(KEY_AVATAR_DATA_URL, null)?.takeIf { it.isNotBlank() }
        )
    }

    fun save(context: Context, displayName: String, avatarDataUrl: String?, signature: String) {
        AuthManager.userDataPrefs(context).edit().apply {
            putString(KEY_NAME, displayName.trim())
            putString(KEY_SIGNATURE, signature.trim())
            if (avatarDataUrl.isNullOrBlank()) remove(KEY_AVATAR_DATA_URL)
            else putString(KEY_AVATAR_DATA_URL, avatarDataUrl)
        }.apply()
    }

    /** 仅更新头像，不覆盖昵称和签名。用于登录后从服务器恢复头像。 */
    fun saveAvatar(context: Context, avatarDataUrl: String) {
        if (avatarDataUrl.isBlank()) return
        AuthManager.userDataPrefs(context).edit()
            .putString(KEY_AVATAR_DATA_URL, avatarDataUrl)
            .apply()
    }

    fun avatarInitial(name: String): String =
        name.trim().firstOrNull()?.toString()?.ifBlank { "我" } ?: "我"

    fun decodeAvatar(dataUrl: String?): Bitmap? {
        val data = dataUrl?.substringAfter(",", "")?.takeIf { it.isNotBlank() } ?: return null
        val key = dataUrl.hashCode()
        avatarCache.get(key)?.let { return it }
        return runCatching {
            val bytes = Base64.decode(data, Base64.DEFAULT)
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
        }.getOrNull()?.also { avatarCache.put(key, it) }
    }

    fun avatarDataUrlFromUri(context: Context, uri: Uri): String {
        val bitmap = context.contentResolver.openInputStream(uri).use { stream ->
            BitmapFactory.decodeStream(stream)
        } ?: error("无法读取头像图片")
        val scaled = cropAndScaleAvatar(bitmap)
        val output = ByteArrayOutputStream()
        scaled.compress(Bitmap.CompressFormat.JPEG, 82, output)
        if (scaled !== bitmap) scaled.recycle()
        bitmap.recycle()
        val encoded = Base64.encodeToString(output.toByteArray(), Base64.NO_WRAP)
        return "data:image/jpeg;base64,$encoded"
    }

    private fun cropAndScaleAvatar(bitmap: Bitmap): Bitmap {
        val side = minOf(bitmap.width, bitmap.height)
        val left = ((bitmap.width - side) / 2).coerceAtLeast(0)
        val top = ((bitmap.height - side) / 2).coerceAtLeast(0)
        val cropped = if (left == 0 && top == 0 && bitmap.width == side && bitmap.height == side) {
            bitmap
        } else {
            Bitmap.createBitmap(bitmap, left, top, side, side)
        }
        if (side <= MAX_AVATAR_SIZE) return cropped
        val scaled = Bitmap.createScaledBitmap(cropped, MAX_AVATAR_SIZE, MAX_AVATAR_SIZE, true)
        if (cropped !== bitmap) cropped.recycle()
        return scaled
    }
}
