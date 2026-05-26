package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.view.Gravity
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
                        joinBtn.setTextColor(Color.parseColor("#888888"))
                        joinBtn.backgroundTintList = ColorStateList.valueOf(Color.parseColor("#2A2A2A"))
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
            setPadding(0, dp(40), 0, dp(40))
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
            setPadding(dp(20), dp(40), dp(20), dp(40))
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
                text = "暂无公开项目\n在[项目管理]中可将项目设为公开，欢迎其他用户加入。"
                textSize = 14f
                setTextColor(Color.parseColor("#888888"))
                gravity = Gravity.CENTER
                setPadding(dp(24), dp(40), dp(24), dp(40))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
            })
            return
        }

        container.addView(TextView(activity).apply {
            text = "发现公开项目 · 加入即可参与开发"
            textSize = 12f
            setTextColor(Color.parseColor("#666666"))
            setPadding(dp(16), dp(14), dp(16), dp(6))
        })

        for (project in projects) {
            container.addView(buildProjectCard(project))
        }
    }

    // ─── 卡片构建 ─────────────────────────────────────────────────────────────

    private fun buildProjectCard(project: StoreProject): LinearLayout {
        val alreadyJoined = joinedIds.contains(project.id)

        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor("#1C1C1C"))
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.setMargins(dp(12), dp(6), dp(12), 0)
            layoutParams = lp
            setPadding(dp(14), dp(14), dp(14), dp(14))
        }

        val textColumn = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }

        textColumn.addView(TextView(activity).apply {
            text = project.name
            textSize = 16f
            setTextColor(Color.parseColor("#E0E0E0"))
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        })

        val desc = project.description?.takeIf { it.isNotBlank() }
        if (desc != null) {
            textColumn.addView(TextView(activity).apply {
                text = desc
                textSize = 13f
                setTextColor(Color.parseColor("#A0A0A0"))
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
                val lp = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
                lp.topMargin = dp(3)
                layoutParams = lp
            })
        }

        textColumn.addView(TextView(activity).apply {
            val owner = project.ownerAccount.takeIf { it != "?" }?.let { " · $it" } ?: ""
            text = "${project.memberCount} 人参与$owner"
            textSize = 11f
            setTextColor(Color.parseColor("#666666"))
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.topMargin = dp(5)
            layoutParams = lp
        })

        card.addView(textColumn)

        val joinLabel = if (alreadyJoined) "已加入" else if (project.joinMode == "open") "加入" else "申请"
        val joinBtn = TextView(activity).apply {
            text = joinLabel
            textSize = 13f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(if (alreadyJoined) "#888888" else "#E0E0E0"))
            backgroundTintList = ColorStateList.valueOf(
                Color.parseColor(if (alreadyJoined) "#2A2A2A" else "#3A6BDE")
            )
            setBackgroundResource(android.R.drawable.btn_default)
            isEnabled = !alreadyJoined
            val lp = LinearLayout.LayoutParams(dp(60), dp(34))
            lp.gravity = Gravity.CENTER_VERTICAL
            lp.marginStart = dp(10)
            layoutParams = lp
            setPadding(dp(4), 0, dp(4), 0)
        }

        if (!alreadyJoined) {
            joinBtn.setOnClickListener { tryJoinProject(project.id, joinBtn) }
        }

        card.addView(joinBtn)
        return card
    }
}
