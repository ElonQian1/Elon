package com.elon.app

import android.app.Application
import android.content.Context
import java.io.IOException
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import okio.Buffer
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], manifest = Config.NONE, application = Application::class)
class AiConversationShareApiTest {
    private val context: Application get() = RuntimeEnvironment.getApplication()
    private val operations get() = context.getSharedPreferences("ai_conversation_share_operations", Context.MODE_PRIVATE)
    private val group = "group_synthetic"
    private val server = "https://share-api.invalid"
    private val attempts = mutableListOf<Attempt>()
    private var failNextTransport = false
    private lateinit var client: OkHttpClient

    @Before fun setup() {
        assertTrue(AuthManager.prefs(context).edit().clear().commit())
        assertTrue(operations.edit().clear().commit())
        client = OkHttpClient.Builder()
            .dns(object : okhttp3.Dns {
                override fun lookup(hostname: String): List<java.net.InetAddress> =
                    throw AssertionError("This synthetic test must never perform DNS or real HTTP")
            })
            .addInterceptor(SyntheticPublishInterceptor())
            .build()
        signIn("account-a")
    }

    @After fun cleanup() {
        if (::client.isInitialized) {
            client.connectionPool.evictAll()
            client.dispatcher.executorService.shutdownNow()
        }
        operations.edit().clear().commit()
        AuthManager.prefs(context).edit().clear().commit()
    }

    @Test fun successfulUnacknowledgedPublishRetainsKeyAcrossNewApiInstanceRetry() {
        val document = document()
        val session = socialSession(context)
        val first = api().publish(group, document, session)
        assertEquals(1, attempts.size)
        val key = attempts.single().key
        assertEquals(key, operations.all.values.single())

        // A successful transport can still lose its UI handoff to cancellation. Do not acknowledge.
        val retried = api().publish(group, JSONObject(document.toString()), session)
        assertEquals(2, attempts.size)
        assertEquals(key, attempts.last().key)
        assertEquals(first, retried)
        assertEquals(key, operations.all.values.single())
    }

    @Test fun acknowledgementAllowsADeliberateNewShareOfTheSameDocumentToUseANewKey() {
        val document = document()
        val session = socialSession(context)
        val publishingApi = api()
        val first = publishingApi.publish(group, document, session)
        val firstKey = attempts.single().key
        assertEquals(1, operations.all.size)

        publishingApi.acknowledgePublish(group, document, session)
        assertTrue(operations.all.isEmpty())
        val second = api().publish(group, document, session)
        assertEquals(2, attempts.size)
        assertNotEquals(firstKey, attempts.last().key)
        assertNotEquals(first.id, second.id)
        assertEquals(attempts.last().key, operations.all.values.single())
    }

    @Test fun transportFailureRetainsTheKeyForANewApiInstanceRetry() {
        val document = document()
        val session = socialSession(context)
        failNextTransport = true
        val failure = assertThrows(IOException::class.java) { api().publish(group, document, session) }
        assertEquals("Synthetic transport failure", failure.message)
        assertEquals(1, attempts.size)
        val failedKey = attempts.single().key
        assertEquals(failedKey, operations.all.values.single())

        val retried = api().publish(group, document, session)
        assertEquals(2, attempts.size)
        assertEquals(failedKey, attempts.last().key)
        assertEquals("ai_snapshot_${failedKey.replace("-", "")}", retried.id)
        assertEquals(failedKey, operations.all.values.single())
    }

    @Test fun accountSwitchCannotReuseOrAcknowledgeAnotherAccountsPendingKey() {
        val document = document()
        val accountASession = socialSession(context)
        val accountAApi = api()
        accountAApi.publish(group, document, accountASession)
        val accountAKey = attempts.single().key

        signIn("account-b")
        val accountBSession = socialSession(context)
        val accountBApi = api()
        accountBApi.publish(group, document, accountBSession)
        val accountBKey = attempts.last().key
        assertNotEquals(accountAKey, accountBKey)
        assertEquals(setOf(accountAKey, accountBKey), operations.all.values.toSet())
        val stale = assertThrows(AiConversationShareApiException::class.java) {
            accountAApi.acknowledgePublish(group, document, accountASession)
        }
        assertEquals(401, stale.status)
        assertEquals(setOf(accountAKey, accountBKey), operations.all.values.toSet())

        api().publish(group, document, accountBSession)
        assertEquals(accountBKey, attempts.last().key)
        accountBApi.acknowledgePublish(group, document, accountBSession)
        assertEquals(accountAKey, operations.all.values.single())

        signIn("account-a", revision = "new-login")
        api().publish(group, document, socialSession(context))
        assertEquals(accountAKey, attempts.last().key)
        assertEquals(listOf("account-a", "account-b", "account-b", "account-a"), attempts.map { it.owner })
        assertEquals(accountAKey, operations.all.values.single())
    }

    private fun api() = AiConversationShareApi(context, client, server)

    private fun signIn(owner: String, revision: String = "initial") {
        // Match existing AuthManager test fixtures without starting auth/background services.
        assertTrue(AuthManager.prefs(context).edit().putString("auth_user_id", owner)
            .putString("auth_token", "synthetic-$owner").putString("auth_session_revision", "$owner-$revision")
            .putLong("auth_expires_at", 0L).commit())
        assertTrue(AuthManager.isLoggedIn(context))
    }

    private fun document(): JSONObject {
        val selected = ChatMessage("friend", "Selected synthetic answer", id = "chatgpt_web:selected", createdAtMs = 123)
        val draft = AiConversationShareDraft("chatgpt", "Selected title", "Selected summary", listOf(selected), emptySet())
        return AiConversationShareCodec.document(draft, draft.title, draft.summary, emptyMap(), false)
    }

    private data class Attempt(val key: String, val owner: String)

    private inner class SyntheticPublishInterceptor : Interceptor {
        override fun intercept(chain: Interceptor.Chain): Response {
            val request = chain.request()
            assertEquals("POST", request.method)
            assertEquals("share-api.invalid", request.url.host)
            assertEquals("/api/me/groups/$group/ai-snapshots", request.url.encodedPath)
            assertEquals("no-cache", request.header("Cache-Control"))
            val owner = AuthManager.userId(context)!!
            assertEquals("Bearer synthetic-$owner", request.header("Authorization"))
            val buffer = Buffer()
            requireNotNull(request.body).writeTo(buffer)
            val payload = JSONObject(buffer.readUtf8())
            assertEquals(setOf("idempotency_key", "document"), payload.keys().asSequence().toSet())
            val key = payload.getString("idempotency_key")
            assertFalse(key.isBlank())
            attempts += Attempt(key, owner)
            if (failNextTransport) {
                failNextTransport = false
                throw IOException("Synthetic transport failure")
            }
            val document = payload.getJSONObject("document")
            val snapshot = "ai_snapshot_" + key.replace("-", "")
            val card = JSONObject().put("schema", AiConversationShareCodec.SCHEMA).put("snapshot_id", snapshot)
                .put("group_id", group).put("title", document.getString("title")).put("summary", document.getString("summary"))
                .put("provider", "chatgpt").put("sender_name", "Synthetic sharer")
                .put("message_count", document.getJSONArray("messages").length())
            val response = JSONObject().put("snapshot_id", snapshot).put("message",
                JSONObject().put("content", AiConversationShareCodec.PREFIX + card.toString()))
            // Never call chain.proceed: every response and failure is owned by this fixture.
            return Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(200).message("Synthetic OK")
                .body(response.toString().toResponseBody("application/json".toMediaType())).build()
        }
    }
}
