package com.elon.app

import com.google.gson.Gson
import okhttp3.OkHttpClient
import java.text.SimpleDateFormat
import java.util.Locale
import java.util.concurrent.TimeUnit

/**
 * 聚合 MainActivity 的运行时可变状态与无需 Context 的工具实例。
 *
 * 独立成类的目的：
 * 1. 让 MainActivity 专注于组装 Action 类（组合根），而非同时充当状态容器。
 * 2. 便于将来下沉到 Jetpack ViewModel，实现生命周期安全的状态保留。
 * 3. 集中配置 OkHttpClient（含超时），防止网络请求永久挂起导致 ANR。
 */
class MainActivityState {

    // ── 请求与连接状态 ───────────────────────────────────────────────────────
    var waitingForReply = false
    var activeRequestIsDevelopment = false
    var serverResponseToken = 0
    var appInForeground = false
    var pendingRequestPayload: String? = null
    var pendingReconnectForActiveWork = false
    var reconnectAttempts = 0
    var backendConnected = false

    // ── 进行中的任务注册表 ────────────────────────────────────────────────────
    val runningConversationTasks = linkedMapOf<String, ConversationTaskState>()
    val runningTraceToConversation = linkedMapOf<String, String>()
    val taskResponseTokens = linkedMapOf<String, Int>()

    // ── 数据列表 ─────────────────────────────────────────────────────────────
    val projects = mutableListOf<AppProject>()
    internal val friends = mutableListOf<AppFriend>()
    internal val groups = mutableListOf<AppGroup>()
    var activeProjectIndex = 0

    // ── 工具实例 ─────────────────────────────────────────────────────────────
    val gson = Gson()
    val timeFormatter = SimpleDateFormat("HH:mm", Locale.CHINA)

    /** OkHttpClient 共享实例，统一配置超时防止网络请求永久挂起导致 ANR。 */
    val http: OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()
}
