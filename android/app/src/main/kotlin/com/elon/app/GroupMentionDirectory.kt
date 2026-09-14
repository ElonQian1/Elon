package com.elon.app

import android.content.Context
import okhttp3.Call
import okhttp3.Callback
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.net.URLEncoder
import java.text.Collator
import java.util.Locale

internal data class GroupMentionTarget(
    val id: String,
    val name: String,
    val avatar: String? = null,
    val isAi: Boolean = false,
)

internal fun filterGroupMentions(items: List<GroupMentionTarget>, query: String): List<GroupMentionTarget> {
    val needle = query.trim().removePrefix("@").removePrefix("＠")
    return items.filter { needle.isBlank() || it.name.contains(needle, true) ||
        (it.isAi && "群AI 一龙 助手 EL".contains(needle, true)) }
}

internal class GroupMentionDirectory(private val context: Context, private val http: OkHttpClient, private val serverUrl: String) {
    fun load(groupId: String, selfId: String, done: (Result<List<GroupMentionTarget>>) -> Unit): Call {
        val id = URLEncoder.encode(groupId, "UTF-8")
        val request = AuthManager.applyAuth(context, Request.Builder()
            .url("$serverUrl/api/me/groups/$id/members").get()).build()
        return http.newCall(request).also { call ->
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) { if (!call.isCanceled()) done(Result.failure(e)) }
                override fun onResponse(call: Call, response: Response) {
                    val result = runCatching {
                        response.use {
                            val json = JSONObject(it.body?.string().orEmpty())
                            check(it.isSuccessful) { json.optString("error", "加载群成员失败") }
                            val people = parse(json.optJSONArray("members"), false).filter { member -> member.id != selfId }
                            val ai = parse(json.optJSONArray("ai_members"), true)
                            val collator = Collator.getInstance(Locale.CHINA)
                            (ai + people.sortedWith { a, b -> collator.compare(a.name, b.name) }).distinctBy { member -> member.id }
                        }
                    }
                    if (!call.isCanceled()) done(result)
                }
            })
        }
    }

    private fun parse(array: JSONArray?, ai: Boolean): List<GroupMentionTarget> = buildList {
        for (index in 0 until (array?.length() ?: 0)) {
            val value = array?.optJSONObject(index) ?: continue
            val id = value.optString("id").trim()
            val name = value.optString("display_name").trim()
            if (id.isNotEmpty() && name.isNotEmpty()) add(GroupMentionTarget(id, name,
                value.optString("avatar_data_url").takeIf { it.startsWith("data:image/") }, ai))
        }
    }
}
