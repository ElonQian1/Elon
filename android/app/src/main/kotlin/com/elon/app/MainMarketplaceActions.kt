package com.elon.app

import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
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

    private data class ProjectCardIdentity(
        val title: String,
        val subtitle: String?
    )

    // 根据字符串生成固定色相的深色渐变（用作卡片顶部识别色带）
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

    private fun identityFor(project: StoreProject): ProjectCardIdentity {
        val name = project.name.trim().ifBlank { "未命名项目" }
        val description = project.description?.trim()?.takeIf { it.isNotBlank() }
        return if (description != null && looksLikeCodeName(name) && description.length <= 24) {
            ProjectCardIdentity(description, "项目代号：$name")
        } else {
            ProjectCardIdentity(name, description)
        }
    }

    private fun looksLikeCodeName(value: String): Boolean {
        if (value.length !in 3..24) return false
        return value.any { it.isLetter() } && value.all { it.isLetterOrDigit() || it == '_' || it == '-' || it == '.' }
    }

    private fun roundedRect(
        color: String,
        radiusDp: Int = 8,
        strokeColor: String? = null
    ): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(radiusDp).toFloat()
            setColor(Color.parseColor(color))
            if (strokeColor != null) setStroke(dp(1), Color.parseColor(strokeColor))
        }
    }

    private fun pill(text: String, textColor: String, bgColor: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 12f
            setTextColor(Color.parseColor(textColor))
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            gravity = Gravity.CENTER
            setPadding(dp(10), dp(5), dp(10), dp(5))
            background = roundedRect(bgColor, 999)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { rightMargin = dp(8) }
        }
    }

    private fun actionButton(text: String, bgColor: String, textColor: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 16f
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(textColor))
            background = roundedRect(bgColor, 8)
            isEnabled = true
            isClickable = true
            foreground = selectableForeground()
        }
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
        joinBtn.setTextColor(Color.parseColor("#07120A"))
        (joinBtn.background as? GradientDrawable)?.setColor(Color.parseColor("#58BE6A"))
        joinBtn.setOnClickListener { openJoinedProject(project) }
    }

    // ─── 渲染 ─────────────────────────────────────────────────────────────────

    private fun renderLoading() {
        val container = getListContainer()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = "加载中..."
            textSize = 14f
            setTextColor(Color.parseColor("#6F7785"))
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
                setTextColor(Color.parseColor("#6F7785"))
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
            setTextColor(Color.parseColor("#6F7785"))
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
        val identity = identityFor(project)

        // 外层卡片容器（圆角 + 深色背景）
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = roundedRect("#181B20", 8, "#1E2126")
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.setMargins(dp(12), dp(12), dp(12), dp(6))
            layoutParams = lp
            clipToOutline = true
        }

        // 顶部识别色带只负责区分卡片，不再抢占项目信息的主视觉。
        card.addView(FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(6)
            )
            background = GradientDrawable(
                GradientDrawable.Orientation.LEFT_RIGHT,
                palette
            )
        })

        // ── 卡片内容区 ────────────────────────────────────────────────────────
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(14), dp(16), dp(16))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        val headerRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        // 圆形头像容器：文字头像 + 真实头像图层叠加
        val avatarSize = dp(54)
        val avatarFrame = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(avatarSize, avatarSize).apply {
                rightMargin = dp(12)
            }
        }
        val avatarText = TextView(activity).apply {
            text = identity.title.firstOrNull()?.uppercaseChar()?.toString() ?: "P"
            textSize = 21f
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            setTextColor(Color.parseColor("#F2F5FA"))
            gravity = Gravity.CENTER
            background = GradientDrawable(
                GradientDrawable.Orientation.TL_BR,
                palette
            ).apply {
                shape = GradientDrawable.OVAL
                setStroke(dp(1), Color.parseColor("#283140"))
            }
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
        }
        avatarFrame.addView(avatarText)
        val avatarImg = ImageView(activity).apply {
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize)
            scaleType = ImageView.ScaleType.CENTER_CROP
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.TRANSPARENT)
            }
            clipToOutline = true
            visibility = android.view.View.GONE
        }
        avatarFrame.addView(avatarImg)
        if (project.ownerUserId.isNotBlank()) {
            loadAvatarAsync(project.ownerUserId, avatarImg)
        }
        headerRow.addView(avatarFrame)

        val titleColumn = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        titleColumn.addView(TextView(activity).apply {
            text = identity.title
            textSize = 21f
            setTextColor(Color.parseColor("#F2F5FA"))
            setTypeface(typeface, android.graphics.Typeface.BOLD)
            maxLines = 2
            setLineSpacing(dp(1).toFloat(), 1.0f)
            ellipsize = android.text.TextUtils.TruncateAt.END
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        })
        identity.subtitle?.let { subtitle ->
            titleColumn.addView(TextView(activity).apply {
                text = subtitle
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(4) }
            })
        }
        headerRow.addView(titleColumn)
        body.addView(headerRow)

        // 在线人数 & 作者行
        val pillRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(12) }
        }
        pillRow.addView(pill("\u25CF  ${project.memberCount} 位成员", "#58BE6A", "#13251A"))
        pillRow.addView(pill(projectJoinModeSummary(project.joinMode), "#81B3D9", "#152C3E"))
        if (!project.latestApkUrl.isNullOrBlank()) {
            pillRow.addView(pill("可安装 APK", "#DDE8FC", "#283140"))
        }
        body.addView(pillRow)

        // 作者
        val owner = project.ownerAccount.takeIf { it != "?" && it.isNotBlank() }
        if (owner != null) {
            body.addView(TextView(activity).apply {
                text = "创建者：$owner"
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(8) }
            })
        }

        // ── 加入按钮（全宽，Discord 绿色）─────────────────────────────────────
        val joinBtn = actionButton(
            projectJoinActionLabel(project.joinMode, alreadyJoined),
            "#58BE6A",
            "#07120A"
        ).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(46)
            ).apply { topMargin = dp(16) }
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
                    dp(46)
                ).apply { topMargin = dp(16) }
            }
            joinBtn.layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            ).apply { rightMargin = dp(10) }

            val installBtn = actionButton("直接安装", "#6091CF", "#F2F5FA").apply {
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
