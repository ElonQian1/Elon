package com.elon.app

import android.app.Application
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import org.json.JSONArray
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class AiConversationShareGroupDraftTest {
    private val context: Application get() = RuntimeEnvironment.getApplication()
    private var group = "group_fixture"
    private var status = 200
    private var switchAccount = false
    private lateinit var client: OkHttpClient

    @Before fun setup() {
        AuthManager.prefs(context).edit().clear().putString("auth_user_id", "fixture-owner")
            .putString("auth_token", "synthetic-test-only").putLong("auth_expires_at", 0L).commit()
        client = OkHttpClient.Builder().addInterceptor { chain ->
            val request = chain.request()
            assertEquals("GET", request.method)
            assertEquals("/api/me/groups/group_fixture/messages/gai_fixture/ai-context", request.url.encodedPath)
            if (switchAccount) AuthManager.prefs(context).edit().putString("auth_user_id", "other").commit()
            Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(status).message("fixture")
                .body(document().toString().toResponseBody("application/json".toMediaType())).build()
        }.build()
    }

    @After fun cleanup() {
        client.connectionPool.evictAll()
        client.dispatcher.executorService.shutdownNow()
        AuthManager.prefs(context).edit().clear().commit()
    }

    @Test fun selectedDraftPreservesMarkdownAndAnswerOrder() {
        val draft = api().groupReplyDraft("group_fixture", "gai_fixture")
        assertEquals("chatgpt", draft.provider)
        assertEquals(listOf("Selected **question**", "```kotlin\nval count = 2\n```"), draft.messages.map { it.content })
        assertEquals(listOf("user", "friend"), draft.messages.map { it.role })
        assertEquals("Selected discussion", draft.title)
    }

    @Test fun anotherGroupOrDeniedSourceCannotBecomeAShareDraft() {
        group = "other_group"
        assertThrows(IllegalArgumentException::class.java) { api().groupReplyDraft("group_fixture", "gai_fixture") }
        status = 409
        assertThrows(AiConversationShareApiException::class.java) { api().groupReplyDraft("group_fixture", "gai_fixture") }
    }

    @Test fun accountChangeDuringReadRejectsTheDraft() {
        switchAccount = true
        assertThrows(AiConversationShareApiException::class.java) { api().groupReplyDraft("group_fixture", "gai_fixture") }
    }

    private fun api() = AiConversationShareApi(context, client, "https://fixture.invalid")
    private fun document() = JSONObject().put("snapshot_id", "preview").put("group_id", group)
        .put("owner_id", "fixture-owner").put("owner_name", "Selected author")
        .put("document", JSONObject().put("schema", "elon.ai_conversation_share.v1").put("provider", "chatgpt")
            .put("title", "Selected discussion").put("summary", "Selected answer")
            .put("messages", JSONArray().put(row("source", "user", "Selected **question**"))
                .put(row("answer", "assistant", "```kotlin\nval count = 2\n```"))))
    private fun row(id: String, role: String, text: String) = JSONObject().put("id", id).put("role", role)
        .put("content", text).put("created_at_ms", 1L).put("parts", JSONArray())
}
