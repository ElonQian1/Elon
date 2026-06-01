package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.ShapeDrawable
import android.graphics.drawable.shapes.OvalShape
import android.view.Gravity
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import android.graphics.BitmapFactory
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

// StoreProject 和 fetchStoreProjects / joinStoreProject 均来自 MainStoreApi.kt

internal class MainMarketplaceActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val getListContainer: () -> LinearLayout,
    private val openJoinedProject: (StoreProject) -> Unit = {}
) {

    private val joinedIds = mutableSetOf<String>()
    private val avatarCache = HashMap<String, android.graphics.Bitmap>()

    // 根据字符串生成固定色相的深色渐变（用作卡片横幅背景）
    private val BANNER_PALETTES = arrayOf(
        intArrayOf(0xFF3B4F8A.toInt(), 0xFF2A3A73.toInt()),  // 深蓝紫
        intArrayOf(0xFF5A3070.toInt(), 0xFF3E1F5A.toInt()),  // 深紫
        intArrayOf(0xFF2D6A4A.toInt(), 0xFF1B4A33.toInt()),  // 深绿
        intArrayOf(0xFF7A3535.toInt(), 0xFF5A2020.toInt()),  // 深红
        intArrayOf(0xFF5A4A1A.toInt(), 0xFF3A3010.toInt()),  // 深金
        intArrayOf(0xFF1A5A6A.toInt(), 0xFF0F3A4A.toInt()),  // 深青
        intArrayOf(0xFF6A3A1A.toInt(), 0xFF4A260F.toInt()),  // 深橙
        intArrayOf(0xFF2A4A6A.toInt(), 0xFF1A3050.toInt()),  // 深天蓝
    )

    private fun paletteFor(key: String): IntArray {
        val hash = key.fold(0) { acc, c -> acc * 31 + c.code }
        return BANNER_PALETTES[Math.abs(hash) % BANNER_PALETTES.size]
    }

    // ─── 加载公开项目列表 ─────────────────────────────────────────────────────

    fun loadProjects(search: String? = null) {
        renderLoading()
        thread {
            val storeResult = runCatching {
                fetchStoreProjects(http, serverUrl, search?.trim()?.ifBlank { null })
            }
            val alreadyJoined: Set<String> = runCatching {
                if (!AuthManager.isLoggedIn(activity)) emptySet<String>()
                else fetchJoinedProjectIds(http, serverUrl, activity)
            }.getOrDefault(emptySet<String>())

            activity.runOnUiThread {
                joinedIds.clear()
                joinedIds.addAll(alreadyJoined)
                storeResult
                    .onSuccess { renderProjects(it) }
                    .onFailure { renderError(it.message ?: "加载失败") }
            }
        }
    }

    // ─── 加入项目 ─────────────────────────────────────────────────────────────

    private fun tryJoinProject(project: StoreProject, joinBtn: TextView) {
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity) ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        joinBtn.isEnabled = false
        joinBtn.text = "加入中..."
        thread {
            val result = runCatching {
                joinStoreProject(http, serverUrl, project.id, token)
            }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        joinedIds.add(project.id)
                        markProjectJoined(project, joinBtn)
                        Toast.makeText(activity, projectJoinSuccessToast(project.joinMode), Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        joinBtn.isEnabled = true
                        joinBtn.text = projectJoinActionLabel(project.joinMode)
                        Toast.makeText(activity, it.message ?: "加入失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun tryInstallProject(project: StoreProject, installBtn: TextView, joinBtn: TextView?) {
        if (!isAndroidApkInstallSupported()) {
            Toast.makeText(activity, "当前设备不是 Android，无法直接安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        val apkUrl = project.latestApkUrl?.trim().orEmpty()
        if (apkUrl.isBlank()) {
            Toast.makeText(activity, "这个项目还没有可安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后安装 APK", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (token.isBlank()) {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }

        val shouldJoin = !joinedIds.contains(project.id)
        installBtn.isEnabled = false
        installBtn.text = if (shouldJoin) "加入中..." else "准备安装..."
        thread {
            val result = runCatching {
                if (shouldJoin) joinStoreProject(http, serverUrl, project.id, token)
                apkUrl
            }
            activity.runOnUiThread {
                installBtn.isEnabled = true
                installBtn.text = "直接安装"
                result
                    .onSuccess { url ->
                        if (shouldJoin) {
                            joinedIds.add(project.id)
                            joinBtn?.let { markProjectJoined(project, it) }
                        }
                        openProjectApkInstall(activity, url, token)
                    }
                    .onFailure {
                        Toast.makeText(activity, it.message ?: "安装失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
    }

    private fun markProjectJoined(project: StoreProject, joinBtn: TextView) {
        joinBtn.text = "进入项目"
        joinBtn.isEnabled = true
        joinBtn.setTextColor(Color.parseColor("#FFFFFF"))
        (joinBtn.background as? GradientDrawable)?.setColor(Color.parseColor("#3BA55D"))
        joinBtn.setOnClickListener { openJoinedProject(project) }
    }

    // ─── 渲染 ─────────────────────────────────────────────────────────────────

    private fun renderLoading() {
        val container = getListContainer()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = "加载中..."
            textSize = 14f
            setTextColor(Color.parseColor("#888888"))
            gravity = Gravity.CENTER
            setPadding(0, dp(60), 0, dp(60))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
    }

    private fun renderError(msg: String) {
        val container = getListContainer()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = msg
            textSize = 14f
            setTextColor(Color.parseColor("#FF7A7A"))
            gravity = Gravity.CENTER
            setPadding(dp(20), dp(60), dp(20), dp(60))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
    }

    private fun renderProjects(projects: List<StoreProject>) {
        val container = getListContainer()
        container.removeAllViews()

        if (projects.isEmpty()) {
            container.addView(TextView(activity).apply {
                text = "暂无公开项目"
                textSize = 15f
                setTextColor(Color.parseColor("#888888"))
                gravity = Gravity.CENTER
                setPadding(dp(24), dp(60), dp(24), dp(60))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
            })
            return
        }

        // 顶部标题栏
        container.addView(TextView(activity).apply {
            text = "公开项目广场 · ${projects.size} 个项目"
            textSize = 12f
            setTextColor(Color.parseColor("#666666"))
            setPadding(dp(16), dp(16), dp(16), dp(8))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })

        for (project in projects) {
            container.addView(buildProjectCard(project))
        }
    }

    // ─── Discord 风格卡片 ─────────────────────────────────────────────────────

    private fun buildProjectCard(project: StoreProject): LinearLayout {
        val alreadyJoined = joinedIds.contains(project.id)
        val palette = paletteFor(project.id)

        // 外层卡片容器（圆角 + 深色背景）
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(12).toFloat()
                setColor(Color.parseColor("#1E1E1E"))
            }
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.setMargins(dp(12), dp(12), dp(12), dp(4))
            layoutParams = lp
            clipToOutline = true
        }

        // ── 顶部彩色横幅 ──────────────────────────────────────────────────────
        val bannerHeight = dp(80)
        val banner = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, bannerHeight
            )
            background = GradientDrawable(
                GradientDrawable.Orientation.TL_BR,
                palette
            ).apply { cornerRadii = floatArrayOf(dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), 0f, 0f, 0f, 0f) }
        }

        // 圆形头像容器：文字头像 + 真实头像图层叠加
        val avatarSize = dp(52)
        val avatarFrame = FrameLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize).apply {
                leftMargin = dp(16)
                topMargin = dp(16)
            }
        }
        // 底层：文字首字母头像
        val avatarText = TextView(activity).apply {
            text = project.name.firstOrNull()?.uppercaseChar()?.toString() ?: "P"
            textSize = 22f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            background = object : ShapeDrawable(OvalShape()) {
                init { paint.color = Color.parseColor("#00000066") }
            }
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
        }
        avatarFrame.addView(avatarText)
        // 上层：真实头像图（异步加载）
        val avatarImg = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
            scaleType = ImageView.ScaleType.CENTER_CROP
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                cornerRadius = avatarSize / 2f
            }
            clipToOutline = true
            visibility = android.view.View.GONE
        }
        avatarFrame.addView(avatarImg)
        if (project.ownerUserId.isNotBlank()) {
            loadAvatarAsync(project.ownerUserId, avatarImg)
        }
        banner.addView(avatarFrame)
        card.addView(banner)

        // ── 卡片内容区 ────────────────────────────────────────────────────────
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(12), dp(16), dp(14))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        // 项目名
        body.addView(TextView(activity).apply {
            text = project.name
            textSize = 17f
            setTextColor(Color.parseColor("#F0F0F0"))
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })

        // 在线人数 & 作者行
        val metaRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(5) }
        }

        // 绿点 + 成员数
        metaRow.addView(TextView(activity).apply {
            val dot = "\u25CF"  // ●
            text = "$dot  ${project.memberCount} 位成员"
            textSize = 12f
            setTextColor(Color.parseColor("#3BA55D"))  // Discord 绿
        })

        // 作者
        val owner = project.ownerAccount.takeIf { it != "?" && it.isNotBlank() }
        if (owner != null) {
            metaRow.addView(TextView(activity).apply {
                text = "  ·  创建者: $owner"
                textSize = 12f
                setTextColor(Color.parseColor("#888888"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })
        }
        body.addView(metaRow)

        // 描述
        val desc = project.description?.takeIf { it.isNotBlank() }
        if (desc != null) {
            body.addView(TextView(activity).apply {
                text = desc
                textSize = 13f
                setTextColor(Color.parseColor("#A0A0A0"))
                maxLines = 3
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(8) }
            })
        }

        // ── 加入按钮（全宽，Discord 绿色）─────────────────────────────────────
        val joinBtn = TextView(activity).apply {
            text = projectJoinActionLabel(project.joinMode, alreadyJoined)
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#FFFFFF"))
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor("#3BA55D"))
            }
            isEnabled = true
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(42)
            ).apply { topMargin = dp(12) }
        }

        if (alreadyJoined) {
            joinBtn.setOnClickListener { openJoinedProject(project) }
        } else {
            joinBtn.setOnClickListener { tryJoinProject(project, joinBtn) }
        }
        if (isAndroidApkInstallSupported() && !project.latestApkUrl.isNullOrBlank()) {
            val actionRow = LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(42)
                ).apply { topMargin = dp(12) }
            }
            joinBtn.layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            ).apply { rightMargin = dp(8) }

            val installBtn = TextView(activity).apply {
                text = "直接安装"
                textSize = 15f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#FFFFFF"))
                background = GradientDrawable().apply {
                    cornerRadius = dp(6).toFloat()
                    setColor(Color.parseColor("#5865F2"))
                }
                isEnabled = true
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { tryInstallProject(project, this, joinBtn) }
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            }

            actionRow.addView(joinBtn)
            actionRow.addView(installBtn)
            body.addView(actionRow)
        } else {
            body.addView(joinBtn)
        }

        card.addView(body)
        return card
    }

    // ─── 异步加载头像 ─────────────────────────────────────────────────────────

    private fun loadAvatarAsync(ownerUserId: String, imageView: ImageView) {
        val cached = avatarCache[ownerUserId]
        if (cached != null) {
            imageView.setImageBitmap(cached)
            imageView.visibility = android.view.View.VISIBLE
            return
        }
        thread(name = "avatar-$ownerUserId") {
            val result = runCatching {
                val req = okhttp3.Request.Builder()
                    .url("$serverUrl/api/users/$ownerUserId/avatar")
                    .get()
                    .build()
                val resp = http.newCall(req).execute()
                if (!resp.isSuccessful) return@runCatching null
                resp.body?.byteStream()?.let { BitmapFactory.decodeStream(it) }
            }
            val bitmap = result.getOrNull()
            if (bitmap != null) {
                avatarCache[ownerUserId] = bitmap
                activity.runOnUiThread {
                    imageView.setImageBitmap(bitmap)
                    imageView.visibility = android.view.View.VISIBLE
                }
            }
        }
    }
}
