package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupChatGptProjectCoordinatorTest {
    private class Harness(state: String = "empty") {
        val project = "g-p-" + "a".repeat(32)
        val commands = mutableListOf<Pair<JSONObject, String>>()
        val actions = mutableListOf<String>()
        val failures = mutableListOf<GroupWebAiFailureReason>()
        val binding = JSONObject().put("binding_id", "11111111-1111-4111-8111-111111111111")
            .put("generation", 1).put("state", state).put("lease_id", "lease").put("group_name", "Fixture")
        var url = "https://chatgpt.com/"
        var readyCount = 0
        var confirm: (() -> Unit)? = null
        val coordinator = GroupChatGptProjectCoordinator(
            command = { raw, id -> commands += JSONObject(raw) to id },
            server = { req, done ->
                actions += req.getString("action")
                when (req.getString("action")) {
                    "create_begin" -> binding.put("state", "creating")
                    "bind" -> binding.put("state", "ready").put("project_id", req.getString("project_id"))
                }
                done(Result.success(JSONObject(binding.toString())))
            }, navigate = { url = it }, changed = { readyCount++ }, failure = failures::add,
            confirmRebuild = { confirm = it }, schedule = { _, _ -> },
        )
        fun snapshot() = ChatGptWebSnapshot("", url, "", emptyList(), true, true, false,
            currentModel = "", attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY)
        fun start() {
            coordinator.snapshot(snapshot(), url)
            reply(JSONObject().put("ok", true).put("accountScope", "a".repeat(64)))
        }
        fun reply(payload: JSONObject) {
            coordinator.event(ChatGptWebEvent.CommandResult("group_project_request", payload.optBoolean("ok"),
                payload.toString(), commands.last().second))
        }
        fun success() = reply(JSONObject().put("ok", true).put("projectId", project))
    }

    @Test fun creationIsJournaledBeforePostAndNavigationNeedsFreshIdentityVerification() {
        val h = Harness(); h.start()
        assertEquals(listOf("acquire", "create_begin"), h.actions)
        assertEquals("create", h.commands.last().first.getString("operation"))
        h.success()
        assertEquals("https://chatgpt.com/g/${h.project}/project", h.url)
        assertFalse(h.coordinator.ready)
        h.coordinator.snapshot(h.snapshot(), h.url); h.success()
        assertTrue(h.coordinator.matches(h.url))
        assertFalse(h.coordinator.matches("https://chatgpt.com/"))
        assertEquals(1, h.readyCount)
        h.coordinator.close()
        assertEquals("release", h.actions.last())
        assertFalse(h.coordinator.ready)
    }

    @Test fun unknownCreateOnlyReconcilesAndLateReceiptsAfterCloseAreIgnored() {
        val h = Harness("creating"); h.start()
        assertEquals(listOf("acquire"), h.actions)
        assertEquals("reconcile", h.commands.last().first.getString("operation"))
        h.coordinator.close(); h.success()
        assertEquals("https://chatgpt.com/", h.url)
        assertFalse(h.actions.contains("bind"))
    }

    @Test fun authAndNetworkFailuresCannotOfferDestructiveRebuild() {
        for (code in listOf("project_auth_required", "project_unavailable", "project_rate_limited")) {
            val h = Harness("ready"); h.binding.put("project_id", h.project); h.start()
            h.reply(JSONObject().put("ok", false).put("code", code))
            assertNull(h.confirm)
            assertEquals(1, h.failures.size)
            assertFalse(h.actions.contains("rebuild"))
        }
    }

    @Test fun unavailableRuntimeDoesNotTellAnAuthenticatedUserToLoginAgain() {
        val h = Harness("ready"); h.binding.put("project_id", h.project); h.start()
        h.reply(JSONObject().put("ok", false).put("code", "project_identity_unavailable").put("identityReason", "account"))
        assertEquals(listOf(GroupWebAiFailureReason.PROJECT), h.failures)
        assertNull(h.confirm)
    }

    @Test fun completeAnswerIsNotDiscardedWhenThreadVerificationTemporarilyFails() {
        val h = Harness(); h.start(); h.success()
        h.coordinator.snapshot(h.snapshot(), h.url); h.success()
        var published = 0
        val conversation = "https://chatgpt.com/g/${h.project}/c/22222222-2222-4222-8222-222222222222"
        h.coordinator.complete(conversation) { published++ }
        assertEquals("remember", h.actions.last())
        h.reply(JSONObject().put("ok", false).put("code", "project_unavailable"))
        assertEquals(1, published)
        assertTrue(h.failures.isEmpty())
        h.coordinator.complete(conversation) { published++ }
        assertEquals(1, published)
    }
}
