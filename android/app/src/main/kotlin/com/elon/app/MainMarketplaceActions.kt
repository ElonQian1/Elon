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
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
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
    private val getListContainer: () -> LinearLayout
) {

    private val joinedIds = mutableSetOf<String>()

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

    private fun tryJoinProject(projectId: String, joinBtn: TextView) {
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
                joinStoreProject(http, serverUrl, projectId, token)
            }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        joinedIds.add(projectId)
                        joinBtn.text = "已加入"
                        joinBtn.isEnabled = false
                        joinBtn.setTextColor(Color.parseColor("#AAAAAA"))
                        (joinBtn.background as? GradientDrawable)?.setColor(Color.parseColor("#2A2A2A"))
                        Toast.makeText(activity, "成功加入项目", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure {
                        joinBtn.isEnabled = true
                        joinBtn.text = "加入"
                        Toast.makeText(activity, it.message ?: "加入失败", Toast.LENGTH_SHORT).show()
                    }
            }
        }
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
            lp.setMargins(dp(12), dp(8), dp(12), 0)
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

        // 圆形头像（项目名首字）
        val avatarSize = dp(52)
        val avatar = TextView(activity).apply {
            text = project.name.firstOrNull()?.uppercaseChar()?.toString() ?: "P"
            textSize = 22f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            background = object : ShapeDrawable(OvalShape()) {
                init { paint.color = Color.parseColor("#00000066") }
            }
            layoutParams = FrameLayout.LayoutParams(avatarSize, avatarSize).apply {
                leftMargin = dp(16)
                topMargin = dp(16)
            }
        }
        banner.addView(avatar)
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
        val joinLabel = when {
            alreadyJoined -> "已加入"
            project.joinMode == "open" -> "加入"
            else -> "申请加入"
        }
        val joinBtn = TextView(activity).apply {
            text = joinLabel
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(if (alreadyJoined) "#AAAAAA" else "#FFFFFF"))
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor(if (alreadyJoined) "#2A2A2A" else "#3BA55D"))
            }
            isEnabled = !alreadyJoined
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(42)
            ).apply { topMargin = dp(12) }
        }

        if (!alreadyJoined) {
            joinBtn.setOnClickListener { tryJoinProject(project.id, joinBtn) }
        }
        body.addView(joinBtn)

        card.addView(body)
        return card
    }
}
