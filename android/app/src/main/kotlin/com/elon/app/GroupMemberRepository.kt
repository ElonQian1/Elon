package com.elon.app

import android.content.Context
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.net.URLEncoder

internal data class RosterPerson(val id: String, val name: String, val avatar: String?, val role: String, val joinedAt: String) {
    val roleLabel: String get() = when (role) { "owner" -> "群主"; "admin" -> "管理员"; else -> "成员" }
}
internal data class GroupRoster(
    val groupId: String, val name: String, val total: Int, val matched: Int, val revision: Long,
    val selfId: String, val selfRole: String, val policy: String, val canInvite: Boolean,
    val canManage: Boolean, val isOwner: Boolean, val pending: Int,
    val people: List<RosterPerson>, val next: String?,
) {
    fun canRemove(person: RosterPerson) = person.id != selfId && person.role != "owner" && (isOwner || (canManage && person.role == "member"))
}
internal class RosterFailure(val status: Int, message: String) : IOException(message)

internal class GroupMemberRepository(private val context: Context, private val http: OkHttpClient, private val serverUrl: String) {
    fun owner() = AuthManager.userId(context)
    fun roster(group: String, query: String, filter: String, cursor: String?, done: (Result<GroupRoster>) -> Unit): Call {
        val path = groupPath(group) + "/roster?q=${encode(query.trim())}&filter=${encode(filter)}" + (cursor?.let { "&cursor=${encode(it)}" } ?: "")
        return request(path, null) { result -> done(result.mapCatching(::parseRoster)) }
    }
    fun command(group: String, value: JSONObject, done: (Result<JSONObject>) -> Unit) = request(groupPath(group) + "/membership", value, done)
    fun friends(done: (Result<List<RosterPerson>>) -> Unit) = request("/api/me/friends", null) { result ->
        done(result.mapCatching { json -> objects(json.getJSONArray("friends")).map { person ->
            RosterPerson(person.getString("id"), person.optString("nickname").takeIf { it.isNotBlank() && it != "null" } ?: person.optString("account", "好友"), nullable(person, "avatar_data_url"), "member", "")
        } })
    }
    fun addFriend(person: RosterPerson, done: (Result<JSONObject>) -> Unit) = request("/api/me/friends", JSONObject().put("query", person.id).put("search_type", "user_id"), done)
    fun invitations(group: String, offset: Int, done: (Result<JSONObject>) -> Unit) = request(groupPath(group) + "/invitations?offset=$offset", null, done)
    private fun groupPath(group: String) = "/api/me/groups/${encode(group)}"
    private fun request(path: String, body: JSONObject?, done: (Result<JSONObject>) -> Unit): Call {
        val owner = AuthManager.userId(context)
        val builder = Request.Builder().url(serverUrl.trimEnd('/') + path).header("Cache-Control", "no-cache")
        if (body != null) builder.post(body.toString().toRequestBody("application/json".toMediaType())) else builder.get()
        return http.newCall(AuthManager.applyAuth(context, builder).build()).also { call -> call.enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) { if (!call.isCanceled() && owner == AuthManager.userId(context)) done(Result.failure(e)) }
            override fun onResponse(call: Call, response: Response) {
                val result = runCatching { response.use {
                    val json = JSONObject(it.body?.string().orEmpty())
                    if (!it.isSuccessful) throw RosterFailure(it.code, json.optString("error", "加载失败").replace(Regex("^[A-Z_]+: "), ""))
                    json
                } }
                if (!call.isCanceled() && owner == AuthManager.userId(context)) done(result)
            }
        }) }
    }
    companion object {
        fun encode(value: String): String = URLEncoder.encode(value, "UTF-8")
        fun nullable(json: JSONObject, key: String): String? = json.optString(key).takeIf { it.isNotBlank() && it != "null" }
        fun objects(array: JSONArray): List<JSONObject> = (0 until array.length()).mapNotNull(array::optJSONObject)
        fun parseRoster(json: JSONObject): GroupRoster {
            val permissions = json.getJSONObject("permissions")
            return GroupRoster(json.getString("group_id"), json.getString("name"), json.getInt("total_count"), json.getInt("matched_count"), json.getLong("revision"),
                json.getString("viewer_id"), json.getString("viewer_role"), json.getString("invitation_policy"), permissions.getBoolean("invite"), permissions.getBoolean("manage"), permissions.getBoolean("owner"), json.getInt("pending_count"),
                objects(json.getJSONArray("members")).map { person -> RosterPerson(person.getString("id"), person.getString("display_name"), nullable(person, "avatar_data_url"), person.getString("role"), person.getString("joined_at")) }, nullable(json, "next_cursor"))
        }
    }
}
