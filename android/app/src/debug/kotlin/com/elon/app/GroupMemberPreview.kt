package com.elon.app

import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.os.Bundle
import android.view.View
import android.widget.Button
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.CopyOnWriteArrayList

/** Entirely offline data: no real account, group or HTTP request is used by this preview. */
internal class GroupMemberFixture : Interceptor {
    @Volatile var total = 172
    @Volatile var revision = 1
    @Volatile var denied = false
    @Volatile var role = "owner"
    val requests = CopyOnWriteArrayList<String>()
    override fun intercept(chain: Interceptor.Chain): Response {
        val request = chain.request(); val url = request.url
        requests.add(url.encodedPath + "?" + url.encodedQuery)
        val json = when {
            denied -> JSONObject().put("error", "你已不在这个群聊中")
            url.encodedPath.endsWith("/roster") -> {
                val query = url.queryParameter("q").orEmpty(); val filter = url.queryParameter("filter"); val offset = url.queryParameter("cursor")?.toIntOrNull() ?: 0
                val all = (0 until total).map { index -> JSONObject().put("id", "member-$index").put("display_name", "成员" + index.toString().padStart(4, '0'))
                    .put("role", if (index == 0) "owner" else if (index == 1) "admin" else "member").put("joined_at", "2026-10-01T10:00:00Z") }
                val matching = all.filter { it.getString("display_name").contains(query) && (filter != "admins" || it.getString("role") != "member") }
                JSONObject().put("group_id", "fixture-group").put("name", "杀蟑螂 · 离线预览").put("total_count", total).put("matched_count", matching.size).put("revision", revision)
                    .put("viewer_id", if (role == "owner") "member-0" else "member-2").put("viewer_role", role).put("invitation_policy", "members").put("pending_count", 1)
                    .put("permissions", JSONObject().put("invite", true).put("manage", role != "member").put("owner", role == "owner"))
                    .put("members", JSONArray(matching.drop(offset).take(50))).put("next_cursor", if (offset + 50 < matching.size) (offset + 50).toString() else JSONObject.NULL)
            }
            url.encodedPath.endsWith("/friends") -> JSONObject().put("friends", JSONArray().put(JSONObject().put("id", "friend-1").put("nickname", "待邀请好友")))
            url.encodedPath.endsWith("/invitations") -> JSONObject().put("requests", JSONArray()).put("total_count", 0).put("next_offset", JSONObject.NULL)
            else -> { revision++; JSONObject().put("ok", true).put("message", "离线预览操作已完成").put("exited", false) }
        }
        return Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(if (denied) 403 else 200).message("offline fixture").body(json.toString().toResponseBody("application/json".toMediaType())).build()
    }
}

class GroupMemberPreviewActivity : AppCompatActivity() {
    private var client: OkHttpClient? = null
    override fun onCreate(savedInstanceState: Bundle?) {
        applyOverrideConfiguration(Configuration(baseContext.resources.configuration).apply { fontScale = intent.getFloatExtra("fontScale", 1f) })
        delegate.localNightMode = if (intent.getStringExtra("theme") == "dark") AppCompatDelegate.MODE_NIGHT_YES else AppCompatDelegate.MODE_NIGHT_NO
        setTheme(R.style.Theme_ElonApp); super.onCreate(savedInstanceState)
        val fixture = GroupMemberFixture().apply { role = intent.getStringExtra("role") ?: "owner" }
        client = OkHttpClient.Builder().addInterceptor(fixture).build()
        GroupMemberScreen(this, GroupMemberRepository(this, client!!, "https://offline.invalid")).show("fixture-group")
    }
    override fun onDestroy() { client?.dispatcher?.cancelAll(); client?.dispatcher?.executorService?.shutdown(); super.onDestroy() }
}

internal fun groupMemberPreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.group.members"
    override val supportedScenarios = setOf("owner", "member")
    override fun createView(context: Context, request: UiRuntimePreviewRequest): View = Button(context).apply {
        text = "打开群成员离线预览（172 人）"
        setOnClickListener { context.startActivity(Intent(context, GroupMemberPreviewActivity::class.java).putExtra("role", request.scenario).putExtra("fontScale", request.fontScale).putExtra("theme", request.theme)) }
    }.uiNode("group.members.preview.open")
}
