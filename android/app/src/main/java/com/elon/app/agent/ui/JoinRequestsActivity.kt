// ui/JoinRequestsActivity.kt
// module: ui | layer: presentation | role: owner 审批管理
// summary: 项目 owner 查看待审批申请并 approve/reject

package com.elon.app.agent.ui

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.widget.*
import com.elon.app.agent.infrastructure.auth.AuthService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

private const val JR_BG = "#101010"
private const val JR_CARD = "#181B20"
private const val JR_TEXT_PRIMARY = "#F2F5FA"
private const val JR_TEXT_SECONDARY = "#A6AFBD"
private const val JR_TEXT_TERTIARY = "#6F7785"
private const val JR_PRIMARY_BG = "#58BE6A"
private const val JR_PRIMARY_TEXT = "#07120A"
private const val JR_SECONDARY_BG = "#283140"
private const val JR_SECONDARY_TEXT = "#DDE8FC"
private const val JR_PENDING = "#F0A030"
private const val JR_DANGER = "#D97A7A"

/**
 * 加入申请审批界面 — 项目 owner 专用。
 *
 * 通过 Intent extra 传入：
 *   - "project_id"   String 必填
 *   - "project_name" String 可选（用于标题显示）
 */
class JoinRequestsActivity : Activity() {

    private lateinit var authService: AuthService
    private val scope = CoroutineScope(Dispatchers.Main)
    private lateinit var listContainer: LinearLayout
    private var projectId = ""
    private var projectName = ""

    companion object {
        fun start(context: Context, projectId: String, projectName: String = "") {
            context.startActivity(Intent(context, JoinRequestsActivity::class.java).apply {
                putExtra("project_id", projectId)
                putExtra("project_name", projectName)
            })
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authService = AuthService(this)
        projectId = intent.getStringExtra("project_id") ?: ""
        projectName = intent.getStringExtra("project_name") ?: "项目"
        if (projectId.isEmpty()) { finish(); return }
        setContentView(buildLayout())
        loadRequests()
    }

    private fun buildLayout(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(JR_BG))

            addView(buildHeader())

            listContainer = LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(0, 0, 0, 80)
            }
            addView(ScrollView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f
                )
                addView(listContainer)
            })
        }
    }

    private fun buildHeader(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor(JR_CARD))
            gravity = Gravity.CENTER_VERTICAL
            setPadding(24, 48, 24, 24)

            addView(Button(context).apply {
                text = "← 返回"
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor("#6091CF"))
                setOnClickListener { finish() }
            })

            addView(TextView(context).apply {
                text = "加入申请 · $projectName"
                textSize = 16f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(JR_TEXT_PRIMARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })

            addView(View(context).apply {
                layoutParams = LinearLayout.LayoutParams(120, 1)
            })
        }
    }

    private fun loadRequests() {
        listContainer.removeAllViews()
        listContainer.addView(buildLoadingView())
        scope.launch {
            val result = withContext(Dispatchers.IO) { fetchRequests() }
            listContainer.removeAllViews()
            when {
                result == null -> listContainer.addView(buildMsgView("加载失败，请检查网络", JR_DANGER))
                result.length() == 0 -> listContainer.addView(buildMsgView("暂无加入申请", JR_TEXT_TERTIARY))
                else -> {
                    for (i in 0 until result.length()) {
                        listContainer.addView(buildRequestCard(result.getJSONObject(i)))
                    }
                }
            }
        }
    }

    private fun fetchRequests(): JSONArray? {
        return try {
            val url = "${authService.getServerUrl()}/api/projects/$projectId/join-requests"
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "GET"
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            conn.setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            val resp = BufferedReader(InputStreamReader(conn.inputStream)).readText()
            JSONObject(resp).optJSONArray("requests")
        } catch (e: Exception) {
            null
        }
    }

    private fun buildRequestCard(req: JSONObject): LinearLayout {
        val reqId = req.optString("id")
        val userAccount = req.optString("user_account", "未知用户")
        val message = req.optString("message", "")
        val status = req.optString("status", "pending")
        val createdAt = req.optString("created_at", "").take(10)

        val (statusText, statusColor) = when (status) {
            "pending" -> Pair("⏳ 待审核", JR_PENDING)
            "approved" -> Pair("✅ 已通过", JR_PRIMARY_BG)
            "rejected" -> Pair("❌ 已拒绝", JR_DANGER)
            else -> Pair(status, JR_TEXT_SECONDARY)
        }

        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(JR_CARD))
            setPadding(24, 20, 24, 20)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = 12
                marginStart = 16
                marginEnd = 16
            }

            // 用户名 + 状态
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(context).apply {
                    text = userAccount
                    textSize = 15f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(JR_TEXT_PRIMARY))
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })

                addView(TextView(context).apply {
                    text = statusText
                    textSize = 13f
                    setTextColor(Color.parseColor(statusColor))
                })
            })

            // 申请留言
            if (message.isNotEmpty()) {
                addView(TextView(context).apply {
                    text = "留言：$message"
                    textSize = 13f
                    setTextColor(Color.parseColor(JR_TEXT_SECONDARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 8 }
                })
            }

            addView(TextView(context).apply {
                text = "申请时间：$createdAt"
                textSize = 12f
                setTextColor(Color.parseColor(JR_TEXT_TERTIARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = 6 }
            })

            // 待审核才显示操作按钮
            if (status == "pending") {
                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.HORIZONTAL
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 16 }

                    addView(Button(context).apply {
                        text = "✓ 通过"
                        textSize = 14f
                        setBackgroundColor(Color.parseColor(JR_PRIMARY_BG))
                        setTextColor(Color.parseColor(JR_PRIMARY_TEXT))
                        layoutParams = LinearLayout.LayoutParams(0, 110, 1f)
                        setOnClickListener { doReview(reqId, "approve") }
                    })

                    addView(Button(context).apply {
                        text = "✗ 拒绝"
                        textSize = 14f
                        setBackgroundColor(Color.parseColor(JR_DANGER))
                        setTextColor(Color.WHITE)
                        layoutParams = LinearLayout.LayoutParams(0, 110, 1f).apply {
                            marginStart = 12
                        }
                        setOnClickListener { doReview(reqId, "reject") }
                    })
                })
            }
        }
    }

    private fun doReview(reqId: String, action: String) {
        scope.launch {
            val body = JSONObject().put("action", action)
            val result = withContext(Dispatchers.IO) {
                patchJson(
                    "${authService.getServerUrl()}/api/projects/$projectId/join-requests/$reqId",
                    body
                )
            }
            if (result != null && result.optBoolean("ok")) {
                val msg = if (action == "approve") "已通过申请" else "已拒绝申请"
                Toast.makeText(this@JoinRequestsActivity, msg, Toast.LENGTH_SHORT).show()
                loadRequests() // 刷新列表
            } else {
                val msg = result?.optString("message") ?: "操作失败"
                Toast.makeText(this@JoinRequestsActivity, msg, Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun patchJson(url: String, body: JSONObject): JSONObject? {
        return try {
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "PATCH"
            conn.doOutput = true
            conn.setRequestProperty("Content-Type", "application/json")
            conn.setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            OutputStreamWriter(conn.outputStream).use { it.write(body.toString()) }
            val code = conn.responseCode
            val stream = if (code in 200..299) conn.inputStream else conn.errorStream
            val resp = BufferedReader(InputStreamReader(stream ?: conn.inputStream)).readText()
            JSONObject(resp)
        } catch (e: Exception) {
            null
        }
    }

    private fun buildLoadingView(): ProgressBar {
        return ProgressBar(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.CENTER_HORIZONTAL
                topMargin = 80
            }
        }
    }

    private fun buildMsgView(msg: String, color: String): TextView {
        return TextView(this).apply {
            text = msg
            textSize = 14f
            setTextColor(Color.parseColor(color))
            gravity = Gravity.CENTER
            setPadding(32, 80, 32, 80)
        }
    }
}
