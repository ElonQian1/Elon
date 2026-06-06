// ui/ProjectPlazaActivity.kt
// module: ui | layer: presentation | role: 项目广场
// summary: 浏览公开项目、申请加入（open/approval 模式）、查看申请状态

package com.elon.app.agent.ui

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
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

private const val BG = "#101010"
private const val CARD = "#181B20"
private const val TEXT_PRIMARY = "#F2F5FA"
private const val TEXT_SECONDARY = "#A6AFBD"
private const val TEXT_TERTIARY = "#6F7785"
private const val PRIMARY_BG = "#58BE6A"
private const val PRIMARY_TEXT = "#07120A"
private const val SECONDARY_BG = "#283140"
private const val SECONDARY_TEXT = "#DDE8FC"
private const val BORDER = "#1E2126"
private const val PENDING_COLOR = "#F0A030"
private const val DANGER = "#D97A7A"

/**
 * 项目广场 — 浏览/搜索公开项目，申请加入。
 *
 * 两个 Tab：
 *   - 发现：浏览公开项目列表，可搜索
 *   - 我的申请：查看自己提交的加入申请状态
 */
class ProjectPlazaActivity : Activity() {

    private lateinit var authService: AuthService
    private val scope = CoroutineScope(Dispatchers.Main)
    private val mainHandler = Handler(Looper.getMainLooper())

    private lateinit var contentArea: LinearLayout
    private lateinit var tabDiscover: Button
    private lateinit var tabMyRequests: Button
    private lateinit var tabOwnerReview: Button
    private var currentTab = "discover"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authService = AuthService(this)
        setContentView(buildLayout())
        showDiscover()
    }

    override fun onResume() {
        super.onResume()
        // 从 JoinRequestsActivity 返回时刷新（特别是 owner_review tab 的 badge）
        when (currentTab) {
            "owner_review" -> showOwnerReview()
            "my_requests" -> showMyRequests()
            else -> {}
        }
    }

    // ── 顶层布局 ─────────────────────────────────────────────────────────────

    private fun buildLayout(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(BG))

            addView(buildHeader())
            addView(buildTabs())

            contentArea = LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f
                )
            }
            addView(ScrollView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f
                )
                addView(contentArea)
            })
        }
    }

    private fun buildHeader(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor(CARD))
            gravity = Gravity.CENTER_VERTICAL
            setPadding(24, 48, 24, 24)

            addView(Button(context).apply {
                text = "← 返回"
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor("#6091CF"))
                setOnClickListener { finish() }
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
            })

            addView(TextView(context).apply {
                text = "🏪 项目广场"
                textSize = 18f
                setTypeface(null, Typeface.BOLD)
                setTextColor(Color.parseColor(TEXT_PRIMARY))
                gravity = Gravity.CENTER
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })

            addView(View(context).apply {
                layoutParams = LinearLayout.LayoutParams(120, 1)
            })
        }
    }

    private fun buildTabs(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(16, 0, 16, 0)

            tabDiscover = Button(context).apply {
                text = "发现"
                textSize = 14f
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor(PRIMARY_BG))
                setOnClickListener { switchTab("discover") }
            }
            addView(tabDiscover, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))

            tabMyRequests = Button(context).apply {
                text = "我的申请"
                textSize = 14f
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor(TEXT_SECONDARY))
                setOnClickListener { switchTab("my_requests") }
            }
            addView(tabMyRequests, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))

            tabOwnerReview = Button(context).apply {
                text = "我管理的"
                textSize = 14f
                setBackgroundColor(Color.TRANSPARENT)
                setTextColor(Color.parseColor(TEXT_SECONDARY))
                setOnClickListener { switchTab("owner_review") }
            }
            addView(tabOwnerReview, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        }
    }

    private fun switchTab(tab: String) {
        currentTab = tab
        val activeColor = Color.parseColor(PRIMARY_BG)
        val inactiveColor = Color.parseColor(TEXT_SECONDARY)
        tabDiscover.setTextColor(if (tab == "discover") activeColor else inactiveColor)
        tabMyRequests.setTextColor(if (tab == "my_requests") activeColor else inactiveColor)
        tabOwnerReview.setTextColor(if (tab == "owner_review") activeColor else inactiveColor)
        when (tab) {
            "discover" -> showDiscover()
            "my_requests" -> showMyRequests()
            "owner_review" -> showOwnerReview()
        }
    }

    // ── Tab1: 发现项目 ───────────────────────────────────────────────────────

    private fun showDiscover() {
        contentArea.removeAllViews()
        contentArea.addView(buildSearchBar())
        val listView = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, 80)
        }
        contentArea.addView(listView)
        loadProjects(listView, "")
    }

    private fun buildSearchBar(): LinearLayout {
        val searchInput = EditText(this).apply {
            hint = "搜索项目名称或描述"
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
            setBackgroundColor(Color.parseColor(SECONDARY_BG))
            setPadding(24, 20, 24, 20)
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(16, 16, 16, 16)
            gravity = Gravity.CENTER_VERTICAL

            addView(searchInput)
            addView(Button(context).apply {
                text = "搜索"
                setBackgroundColor(Color.parseColor(PRIMARY_BG))
                setTextColor(Color.parseColor(PRIMARY_TEXT))
                setPadding(24, 0, 24, 0)
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT, 120
                ).apply { marginStart = 8 }
                setOnClickListener {
                    val query = searchInput.text.toString().trim()
                    val listView = (contentArea.getChildAt(1) as? LinearLayout) ?: return@setOnClickListener
                    listView.removeAllViews()
                    loadProjects(listView, query)
                }
            })
        }
    }

    private fun loadProjects(container: LinearLayout, query: String) {
        container.addView(buildLoadingView())
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                fetchProjects(query)
            }
            container.removeAllViews()
            if (result == null) {
                container.addView(buildErrorView("加载失败，请检查网络"))
            } else if (result.length() == 0) {
                container.addView(buildEmptyView(if (query.isEmpty()) "暂无公开项目" else "未找到「$query」相关项目"))
            } else {
                for (i in 0 until result.length()) {
                    container.addView(buildProjectCard(result.getJSONObject(i)))
                }
            }
        }
    }

    private fun fetchProjects(query: String): JSONArray? {
        return try {
            val url = buildString {
                append(authService.getServerUrl())
                append("/api/store/projects?limit=30")
                if (query.isNotEmpty()) append("&q=${java.net.URLEncoder.encode(query, "UTF-8")}")
            }
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "GET"
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            val token = authService.getToken()
            if (token != null) conn.setRequestProperty("Authorization", "Bearer $token")
            val resp = BufferedReader(InputStreamReader(conn.inputStream)).readText()
            JSONObject(resp).optJSONArray("projects")
        } catch (e: Exception) {
            null
        }
    }

    private fun buildProjectCard(project: JSONObject): LinearLayout {
        val projectId = project.optString("id")
        val name = project.optString("name", "未知项目")
        val description = project.optString("description", "")
        val ownerAccount = project.optString("owner_account", "")
        val memberCount = project.optInt("member_count", 0)
        val joinMode = project.optString("join_mode", "open") // open | approval | invite | readonly
        val template = project.optString("template", "android")

        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(24, 24, 24, 20)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = 12
                marginStart = 16
                marginEnd = 16
            }

            // 项目名 + 模板标签
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(context).apply {
                    text = name
                    textSize = 16f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })

                // 模板小标签
                addView(TextView(context).apply {
                    text = template
                    textSize = 11f
                    setTextColor(Color.parseColor(SECONDARY_TEXT))
                    setBackgroundColor(Color.parseColor(SECONDARY_BG))
                    setPadding(10, 4, 10, 4)
                })
            })

            // 描述
            if (description.isNotEmpty()) {
                addView(TextView(context).apply {
                    text = description
                    textSize = 13f
                    setTextColor(Color.parseColor(TEXT_SECONDARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 8 }
                    maxLines = 2
                    ellipsize = android.text.TextUtils.TruncateAt.END
                })
            }

            // owner + 成员数
            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = 12 }

                addView(TextView(context).apply {
                    text = "👤 $ownerAccount"
                    textSize = 12f
                    setTextColor(Color.parseColor(TEXT_TERTIARY))
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })

                addView(TextView(context).apply {
                    text = "👥 $memberCount 人"
                    textSize = 12f
                    setTextColor(Color.parseColor(TEXT_TERTIARY))
                })
            })

            // 操作按钮（根据 join_mode）
            if (joinMode != "invite") {
                addView(buildJoinButton(projectId, name, joinMode))
            }
        }
    }

    private fun buildJoinButton(projectId: String, projectName: String, joinMode: String): View {
        val (btnText, btnColor) = when (joinMode) {
            "open" -> Pair("直接加入", PRIMARY_BG)
            "approval" -> Pair("申请加入", "#5B7FBA")
            "readonly" -> Pair("只读访问", SECONDARY_BG)
            else -> Pair("加入", PRIMARY_BG)
        }

        return Button(this).apply {
            text = btnText
            textSize = 14f
            setBackgroundColor(Color.parseColor(btnColor))
            setTextColor(if (joinMode == "readonly") Color.parseColor(SECONDARY_TEXT) else Color.parseColor(PRIMARY_TEXT))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 110
            ).apply { topMargin = 16 }

            setOnClickListener {
                if (!authService.isLoggedIn()) {
                    Toast.makeText(context, "请先登录", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                when (joinMode) {
                    "approval" -> showApplyDialog(projectId, projectName)
                    else -> doJoin(projectId, projectName)
                }
            }
        }
    }

    private fun showApplyDialog(projectId: String, projectName: String) {
        val msgInput = EditText(this).apply {
            hint = "申请留言（可选）"
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
            maxLines = 3
        }
        AlertDialog.Builder(this)
            .setTitle("申请加入「$projectName」")
            .setMessage("owner 审批通过后你将成为项目成员")
            .setView(msgInput)
            .setPositiveButton("提交申请") { _, _ ->
                val msg = msgInput.text.toString().trim()
                doRequestJoin(projectId, projectName, msg.ifEmpty { null })
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doJoin(projectId: String, projectName: String) {
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                postJson("${authService.getServerUrl()}/api/projects/$projectId/join", JSONObject())
            }
            if (result != null && result.optBoolean("ok")) {
                Toast.makeText(this@ProjectPlazaActivity, "已成功加入「$projectName」", Toast.LENGTH_SHORT).show()
            } else {
                // 如果返回 approval_required，自动弹申请对话框
                val code = result?.optString("code")
                if (code == "approval_required") {
                    showApplyDialog(projectId, projectName)
                } else {
                    val msg = extractApiError(result, "加入失败")
                    Toast.makeText(this@ProjectPlazaActivity, msg, Toast.LENGTH_LONG).show()
                }
            }
        }
    }

    private fun doRequestJoin(projectId: String, projectName: String, message: String?) {
        scope.launch {
            val body = JSONObject().apply {
                if (message != null) put("message", message)
            }
            val result = withContext(Dispatchers.IO) {
                postJson("${authService.getServerUrl()}/api/projects/$projectId/request-join", body)
            }
            if (result != null && result.optBoolean("ok")) {
                Toast.makeText(
                    this@ProjectPlazaActivity,
                    "申请已提交，等待「$projectName」owner 审核",
                    Toast.LENGTH_LONG
                ).show()
                // 切换到"我的申请"tab 查看状态
                switchTab("my_requests")
            } else {
                val msg = extractApiError(result, "申请失败")
                Toast.makeText(this@ProjectPlazaActivity, msg, Toast.LENGTH_LONG).show()
            }
        }
    }

    // ── Tab2: 我的申请 ───────────────────────────────────────────────────────

    private fun showMyRequests() {
        contentArea.removeAllViews()
        if (!authService.isLoggedIn()) {
            contentArea.addView(buildEmptyView("请先登录查看申请状态"))
            return
        }
        contentArea.addView(buildLoadingView())
        scope.launch {
            val result = withContext(Dispatchers.IO) { fetchMyRequests() }
            contentArea.removeAllViews()
            if (result == null) {
                contentArea.addView(buildErrorView("加载失败，请检查网络"))
            } else if (result.length() == 0) {
                contentArea.addView(buildEmptyView("暂无加入申请记录"))
            } else {
                val list = LinearLayout(this@ProjectPlazaActivity).apply {
                    orientation = LinearLayout.VERTICAL
                    setPadding(0, 8, 0, 80)
                }
                for (i in 0 until result.length()) {
                    list.addView(buildRequestCard(result.getJSONObject(i)))
                }
                contentArea.addView(list)
            }
        }
    }

    private fun fetchMyRequests(): JSONArray? {
        return try {
            val url = "${authService.getServerUrl()}/api/me/join-requests"
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

    // ── Tab3: owner 审批中心 ──────────────────────────────────────────────────

    private fun showOwnerReview() {
        contentArea.removeAllViews()
        if (!authService.isLoggedIn()) {
            contentArea.addView(buildEmptyView("请先登录管理你创建的项目"))
            return
        }
        contentArea.addView(buildLoadingView())
        scope.launch {
            val result = withContext(Dispatchers.IO) { fetchOwnedProjects() }
            contentArea.removeAllViews()
            if (result == null) {
                contentArea.addView(buildErrorView("加载失败，请检查网络"))
                return@launch
            }
            val projects = result.optJSONArray("projects")
            val totalPending = result.optInt("total_pending", 0)
            // 顶部摘要
            contentArea.addView(TextView(this@ProjectPlazaActivity).apply {
                text = if (totalPending > 0) "共有 $totalPending 个待审批申请" else "暂无待审批申请"
                textSize = 13f
                setTextColor(Color.parseColor(if (totalPending > 0) PENDING_COLOR else TEXT_SECONDARY))
                setPadding(24, 20, 24, 8)
            })
            // 注册本地项目按钮（用于把外部本地路径项目注册到云端）
            contentArea.addView(Button(this@ProjectPlazaActivity).apply {
                text = "+ 注册本地项目（外部路径）"
                textSize = 13f
                setTextColor(Color.WHITE)
                setBackgroundColor(Color.parseColor(PRIMARY_BG))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ).apply {
                    marginStart = 16
                    marginEnd = 16
                    topMargin = 8
                    bottomMargin = 8
                }
                setOnClickListener { showRegisterExternalDialog() }
            })
            if (projects == null || projects.length() == 0) {
                contentArea.addView(buildEmptyView("你还没有创建任何项目"))
                return@launch
            }
            val list = LinearLayout(this@ProjectPlazaActivity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(0, 0, 0, 80)
            }
            for (i in 0 until projects.length()) {
                list.addView(buildOwnedProjectCard(projects.getJSONObject(i)))
            }
            contentArea.addView(list)
        }
    }

    private fun fetchOwnedProjects(): JSONObject? {
        return try {
            val url = "${authService.getServerUrl()}/api/me/owned-projects/pending-counts"
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "GET"
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            conn.setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            val resp = BufferedReader(InputStreamReader(conn.inputStream)).readText()
            JSONObject(resp)
        } catch (e: Exception) {
            null
        }
    }

    private fun buildOwnedProjectCard(project: JSONObject): LinearLayout {
        val projectId = project.optString("project_id")
        val projectName = project.optString("project_name", "未知项目")
        val pendingCount = project.optInt("pending_count", 0)

        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(24, 24, 24, 24)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = 12
                marginStart = 16
                marginEnd = 16
            }

            addView(LinearLayout(context).apply {
                orientation = LinearLayout.VERTICAL
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                addView(TextView(context).apply {
                    text = projectName
                    textSize = 15f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                })
                addView(TextView(context).apply {
                    text = if (pendingCount > 0) "$pendingCount 条待审批" else "已全部处理"
                    textSize = 12f
                    setTextColor(Color.parseColor(if (pendingCount > 0) PENDING_COLOR else TEXT_TERTIARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 4 }
                })
            })

            // 红点 badge
            if (pendingCount > 0) {
                addView(TextView(context).apply {
                    text = if (pendingCount > 99) "99+" else pendingCount.toString()
                    textSize = 12f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.WHITE)
                    setBackgroundColor(Color.parseColor(DANGER))
                    setPadding(16, 6, 16, 6)
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { marginEnd = 12 }
                })
            }

            addView(TextView(context).apply {
                text = "›"
                textSize = 22f
                setTextColor(Color.parseColor(TEXT_TERTIARY))
            })

            setOnClickListener {
                JoinRequestsActivity.start(this@ProjectPlazaActivity, projectId, projectName)
            }
        }
    }

    private fun buildRequestCard(req: JSONObject): LinearLayout {
        val projectName = req.optString("project_name", "未知项目")
        val status = req.optString("status", "pending")
        val createdAt = req.optString("created_at", "").take(10)
        val message = req.optString("message", "")

        val (statusText, statusColor) = when (status) {
            "pending" -> Pair("⏳ 待审核", PENDING_COLOR)
            "approved" -> Pair("✅ 已通过", PRIMARY_BG)
            "rejected" -> Pair("❌ 已拒绝", DANGER)
            else -> Pair(status, TEXT_SECONDARY)
        }

        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(CARD))
            setPadding(24, 20, 24, 20)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = 12
                marginStart = 16
                marginEnd = 16
            }

            addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(context).apply {
                    text = projectName
                    textSize = 15f
                    setTypeface(null, Typeface.BOLD)
                    setTextColor(Color.parseColor(TEXT_PRIMARY))
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                })

                addView(TextView(context).apply {
                    text = statusText
                    textSize = 13f
                    setTextColor(Color.parseColor(statusColor))
                })
            })

            if (message.isNotEmpty()) {
                addView(TextView(context).apply {
                    text = "留言：$message"
                    textSize = 13f
                    setTextColor(Color.parseColor(TEXT_SECONDARY))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply { topMargin = 8 }
                })
            }

            addView(TextView(context).apply {
                text = "申请时间：$createdAt"
                textSize = 12f
                setTextColor(Color.parseColor(TEXT_TERTIARY))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = 8 }
            })

            // pending 时显示"撤销申请"按钮
            if (status == "pending") {
                val reqId = req.optString("id")
                addView(Button(context).apply {
                    text = "撤销申请"
                    textSize = 13f
                    setBackgroundColor(Color.parseColor(SECONDARY_BG))
                    setTextColor(Color.parseColor(SECONDARY_TEXT))
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply {
                        topMargin = 12
                        gravity = Gravity.END
                    }
                    setOnClickListener { confirmCancelRequest(reqId) }
                })
            }
        }
    }

    private fun confirmCancelRequest(reqId: String) {
        AlertDialog.Builder(this)
            .setTitle("撤销申请")
            .setMessage("确认撤销此次加入申请？撤销后可重新提交。")
            .setNegativeButton("取消", null)
            .setPositiveButton("确认撤销") { _, _ ->
                scope.launch {
                    val ok = withContext(Dispatchers.IO) { cancelMyRequest(reqId) }
                    if (ok) {
                        Toast.makeText(this@ProjectPlazaActivity, "已撤销", Toast.LENGTH_SHORT).show()
                        showMyRequests()
                    } else {
                        Toast.makeText(this@ProjectPlazaActivity, "撤销失败，请稍后重试", Toast.LENGTH_LONG).show()
                    }
                }
            }
            .show()
    }

    private fun cancelMyRequest(reqId: String): Boolean {
        return try {
            val url = "${authService.getServerUrl()}/api/me/join-requests/$reqId"
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "DELETE"
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            conn.setRequestProperty("Authorization", "Bearer ${authService.getToken()}")
            conn.responseCode in 200..299
        } catch (e: Exception) {
            false
        }
    }

    // ── 注册外部本地路径项目 ────────────────────────────────────────────────

    private fun showRegisterExternalDialog() {
        val container = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(48, 24, 48, 0)
        }
        val nameInput = EditText(this).apply {
            hint = "项目名称（如：bb64a）"
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
        }
        val pathInput = EditText(this).apply {
            hint = "本机绝对路径（如：D:\\rust\\active-projects\\bb64a）"
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
        }
        val descInput = EditText(this).apply {
            hint = "描述（可选）"
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_PRIMARY))
            setHintTextColor(Color.parseColor(TEXT_TERTIARY))
            maxLines = 3
        }
        val publishCheck = CheckBox(this).apply {
            text = "发布到项目广场"
            textSize = 14f
            isChecked = true
            setTextColor(Color.parseColor(TEXT_PRIMARY))
        }
        val joinModeSpinner = Spinner(this).apply {
            val labels = arrayOf("只读体验", "直接加入", "申请加入")
            adapter = ArrayAdapter(
                this@ProjectPlazaActivity,
                android.R.layout.simple_spinner_dropdown_item,
                labels,
            )
            setSelection(0)
        }
        container.addView(nameInput)
        container.addView(pathInput)
        container.addView(descInput)
        container.addView(publishCheck)
        container.addView(joinModeSpinner)
        AlertDialog.Builder(this)
            .setTitle("注册本地项目")
            .setMessage("把已存在的本机目录登记为云端项目。路径必须存在于服务器或在线 PC 节点上。")
            .setView(container)
            .setPositiveButton("注册") { _, _ ->
                val n = nameInput.text.toString().trim()
                val p = pathInput.text.toString().trim()
                val d = descInput.text.toString().trim()
                if (n.isEmpty() || p.isEmpty()) {
                    Toast.makeText(this, "名称和路径都不能为空", Toast.LENGTH_SHORT).show()
                    return@setPositiveButton
                }
                val joinMode = when (joinModeSpinner.selectedItemPosition) {
                    1 -> "open"
                    2 -> "approval"
                    else -> "readonly"
                }
                doRegisterExternal(n, p, d.ifEmpty { null }, publishCheck.isChecked, joinMode)
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doRegisterExternal(
        name: String,
        path: String,
        description: String?,
        isPublic: Boolean,
        joinMode: String,
    ) {
        scope.launch {
            val body = JSONObject().apply {
                put("name", name)
                put("workspace_path", path)
                if (description != null) put("description", description)
                put("is_public", isPublic)
                put("join_mode", joinMode)
            }
            val result = withContext(Dispatchers.IO) {
                postJson("${authService.getServerUrl()}/api/projects/external", body)
            }
            val project = result?.optJSONObject("project")
            if (project != null && project.optString("id").isNotEmpty()) {
                val reused = result.optBoolean("reused_existing", false)
                Toast.makeText(
                    this@ProjectPlazaActivity,
                    if (reused) "已存在同名项目，复用：${project.optString("name")}" else "注册成功：${project.optString("name")}",
                    Toast.LENGTH_LONG,
                ).show()
                showOwnerReview()
            } else {
                Toast.makeText(this@ProjectPlazaActivity, extractApiError(result, "注册失败"), Toast.LENGTH_LONG).show()
            }
        }
    }

    // ── 公用工具 ─────────────────────────────────────────────────────────────

    private fun postJson(url: String, body: JSONObject): JSONObject? {
        return try {
            val conn = URL(url).openConnection() as HttpURLConnection
            conn.requestMethod = "POST"
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
                bottomMargin = 80
            }
        }
    }

    private fun buildErrorView(msg: String): TextView {
        return TextView(this).apply {
            text = msg
            textSize = 14f
            setTextColor(Color.parseColor(DANGER))
            gravity = Gravity.CENTER
            setPadding(32, 80, 32, 80)
        }
    }

    private fun buildEmptyView(msg: String): TextView {
        return TextView(this).apply {
            text = msg
            textSize = 14f
            setTextColor(Color.parseColor(TEXT_TERTIARY))
            gravity = Gravity.CENTER
            setPadding(32, 80, 32, 80)
        }
    }

    private fun extractApiError(result: JSONObject?, fallback: String): String {
        val message = result?.optString("message")?.trim().orEmpty()
        if (message.isNotEmpty()) return message
        val error = result?.optString("error")?.trim().orEmpty()
        if (error.isNotEmpty()) return error
        return fallback
    }
}
