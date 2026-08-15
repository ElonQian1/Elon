package com.elon.app.googleweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class GoogleWebAdapterContractTest {
    @Test
    fun adapterExposesVisibleChatSemanticsWithoutCredentialsOrPrivateApis() {
        val adapter = read("android/app/src/main/assets/google_web_adapter.js")
        val extractor = read("android/app/src/main/assets/google_web_message_extractor.js")
        val queryPolicy = read("android/app/src/main/assets/google_web_query_policy.js")
        val composerBridge = read("android/app/src/main/assets/google_web_composer_bridge.js")
        val sendPolicy = read("android/app/src/main/assets/google_web_send_policy.js")
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt",
        )
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val officialActivity = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebOfficialActivity.kt",
        )
        val modeController = read(
            "android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt",
        )
        val pendingSendState = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPendingSendState.kt",
        )

        assertTrue(adapter.contains("providerId: 'google_web'"))
        assertTrue(adapter.contains("documentToken"))
        assertTrue(adapter.contains("type: 'message_snapshot'"))
        assertTrue(adapter.contains("messageExtractor.extract"))
        assertTrue(adapter.contains("'dom_diagnostics'"))
        assertTrue(adapter.contains("action === 'send_prompt'"))
        assertTrue(adapter.contains("sendPolicy.reconcile"))
        assertTrue(adapter.contains("ownedComposer === true"))
        assertTrue(adapter.contains("composerBridge.findSubmitAction"))
        assertTrue(adapter.contains("firstPromptNavigationUrl"))
        assertTrue(adapter.contains("navigationFallbackAllowed"))
        assertTrue(adapter.contains("messageExtractor.currentQueryMatches"))
        assertTrue(adapter.contains("messageExtractor.hasCurrentQuery"))
        assertTrue(adapter.contains("composerBridge.find()"))
        assertTrue(adapter.contains("composerBridge.findAction"))
        assertTrue(adapter.contains("function confirmSubmission(startedAt)"))
        assertTrue(adapter.contains("function submitWhenReady()"))
        assertTrue(adapter.contains("SUBMIT_READY_TIMEOUT_MS"))
        assertTrue(adapter.contains("messageExtractor.rememberQuery(prompt)"))
        assertTrue(adapter.contains("action === 'stop_generation'"))
        assertTrue(adapter.contains("action === 'new_conversation'"))
        assertTrue(adapter.contains("MutationObserver"))
        assertTrue(!adapter.contains("document.cookie"))
        assertTrue(!adapter.contains("Authorization"))
        assertTrue(!adapter.contains("fetch("))
        assertTrue(extractor.contains("answerCandidate"))
        assertTrue(extractor.contains("const answer = query ? answerCandidate"))
        assertTrue(extractor.contains("rememberQuery"))
        assertTrue(extractor.contains("queryPolicy.select"))
        assertTrue(extractor.contains("currentQueryMatches"))
        assertTrue(extractor.contains("hasCurrentQuery"))
        assertTrue(queryPolicy.contains("rememberedQuery"))
        assertTrue(queryPolicy.contains("explicitQuery"))
        assertTrue(queryPolicy.contains("urlQuery"))
        assertTrue(extractor.contains("diagnostics"))
        assertTrue(!extractor.contains("document.cookie"))
        assertTrue(!extractor.contains("Authorization"))
        assertTrue(!extractor.contains("fetch("))
        assertTrue(!extractor.contains("sessionStorage"))
        assertTrue(sendPolicy.contains("currentDraft === prompt"))
        assertTrue(sendPolicy.contains("queryMatches === true"))
        assertTrue(sendPolicy.contains("latestUserQueryMatches"))
        assertTrue(sendPolicy.contains("submissionStep"))
        assertTrue(sendPolicy.contains("ownedComposer === true"))
        assertTrue(sendPolicy.contains("navigationFallbackAllowed === true"))
        assertTrue(composerBridge.contains("node.shadowRoot"))
        assertTrue(composerBridge.contains("frame.contentDocument"))
        assertTrue(composerBridge.contains("findSubmitAction"))
        assertTrue(composerBridge.contains("scoreSubmitAction"))
        assertTrue(composerBridge.contains("Number(root.__elonGoogleWebComposerBridge.version"))
        assertTrue(pageAdapter.contains("WEB_MESSAGE_LISTENER"))
        assertTrue(pageAdapter.contains("ALLOWED_ORIGINS"))
        assertTrue(pageAdapter.contains("WebBridgeDocumentSession"))
        assertTrue(pageAdapter.contains("WebBridgeReadinessPolicy.stateAfterPageReady"))
        assertTrue(pageAdapter.contains("ChatGptWebProtocol.parseMessage"))
        assertTrue(pageAdapter.contains("ownedComposer = true"))
        assertTrue(session.contains("GoogleWebConversationStore"))
        assertTrue(session.contains("GoogleWebConversationSnapshotStore"))
        assertTrue(session.contains("GoogleWebSnapshotPresentation.loading"))
        assertTrue(session.contains("ChatGptWebProxyController"))
        assertTrue(session.contains("snapshot.composerReady && !snapshot.streaming"))
        assertTrue(session.contains("event.ok || event.action == \"send_prompt\""))
        assertTrue(session.contains("fun currentOfficialUrl(): String?"))
        assertTrue(session.contains("conversationStore::restorableUrl"))
        assertTrue(officialActivity.contains("intent.getStringExtra(EXTRA_START_URL)"))
        assertTrue(officialActivity.contains("sanitizeRestorableUrl(requestedUrl)"))
        assertTrue(modeController.contains("officialFallbackUrl()"))
        val controller = read("android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt")
        assertTrue(controller.contains("session.currentOfficialUrl()"))
        assertTrue(controller.contains("pendingSend.confirmSubmission()"))
        assertTrue(controller.contains("restorePrompt(failedPrompt)"))
        assertTrue(controller.contains("pendingSend.onConfirmationTimeout(generation)"))
        assertTrue(Regex(
            """TimeoutAction\.KEEP_WAITING\s*->\s*\{.*?scheduleSubmissionConfirmationWatchdog\(generation\)""",
            RegexOption.DOT_MATCHES_ALL,
        ).containsMatchIn(controller))
        assertTrue(controller.contains("TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION"))
        assertTrue(controller.contains("pendingSend.requiresOfficialConfirmation()"))
        assertTrue(controller.contains("binding.root::removeCallbacks"))
        assertTrue(controller.contains("session.requestConversationIndex()"))
        assertTrue(pendingSendState.contains("TimeoutAction.KEEP_WAITING"))
        assertTrue(pendingSendState.contains("TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION"))
        assertTrue(pendingSendState.contains("TimeoutAction.RESTORE"))
    }

    private fun read(relative: String): String =
        String(Files.readAllBytes(root().resolve(relative)), StandardCharsets.UTF_8)

    private fun root(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
