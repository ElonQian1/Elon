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
        val richContent = read("android/app/src/main/assets/google_web_rich_content.js")
        val composerBridge = read("android/app/src/main/assets/google_web_composer_bridge.js")
        val sendPolicy = read("android/app/src/main/assets/google_web_send_policy.js")
        val privateDirectory = read(
            "android/app/src/main/assets/google_web_private_thread_directory.js",
        )
        val pageAdapter = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt",
        )
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val responseRefresh = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebResponseRefreshCoordinator.kt",
        )
        val officialActivity = read(
            "android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebOfficialActivity.kt",
        )
        val modeController = read(
            "android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt",
        )
        val pendingSendState = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatPendingSendState.kt",
        )
        val sendCoordinator = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatSendCoordinator.kt",
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
        assertTrue(adapter.contains("function dispose()"))
        assertTrue(adapter.contains("emitPrivateDirectorySnapshot"))
        assertTrue(adapter.contains("providerUrl: value.providerUrl"))
        assertTrue(adapter.contains("observer.disconnect()"))
        assertTrue(pageAdapter.contains("typeof b.dispose==='function'"))
        assertTrue(!adapter.contains("document.cookie"))
        assertTrue(!adapter.contains("Authorization"))
        assertTrue(!adapter.contains("fetch("))
        assertTrue(privateDirectory.contains("AimThreadsService/ListThreads"))
        assertTrue(privateDirectory.contains("csuir.replace(active.id, row.id)"))
        assertTrue(!privateDirectory.contains("document.cookie"))
        assertTrue(!privateDirectory.contains("setRequestHeader"))
        assertTrue(extractor.contains("answerCandidate"))
        assertTrue(extractor.contains("[id^=\"aim-chrome-initial-inline-async-container\"]"))
        assertTrue(extractor.contains("const queries = queryEntries()"))
        assertTrue(extractor.contains("richContent.parts(answer.node, answer.text, entry.text)"))
        assertTrue(richContent.contains("__elonGoogleWebRichContent"))
        assertTrue(extractor.contains("answerCandidate(composer, entry.text, entry.node"))
        assertTrue(extractor.contains("rememberQuery"))
        assertTrue(extractor.contains("queryPolicy.select"))
        assertTrue(extractor.contains("currentQueryMatches"))
        assertTrue(extractor.contains("hasCurrentQuery"))
        assertTrue(extractor.contains("findQueryAnchor"))
        assertTrue(extractor.contains("afterQuery: followsQuery"))
        assertTrue(extractor.contains("trustedAnswerContainer: node.matches"))
        assertTrue(extractor.contains("TRUSTED_ANSWER_SELECTORS"))
        assertTrue(extractor.contains("rememberedOwned: rememberedQueryOwned"))
        assertTrue(extractor.contains("candidatePolicy.select(candidates)"))
        assertTrue(extractor.contains("interactive: !!node.closest"))
        assertTrue(extractor.contains("if (candidate.text.length < 8) return true"))
        assertTrue(queryPolicy.contains("rememberedQuery"))
        assertTrue(queryPolicy.contains("rememberedOwned"))
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
        assertTrue(pageAdapter.contains("privateThreadDirectoryScript"))
        assertTrue(pageAdapter.contains("DOCUMENT_START_SCRIPT"))
        assertTrue(pageAdapter.contains("ALLOWED_ORIGINS"))
        assertTrue(pageAdapter.contains("WebBridgeDocumentSession"))
        assertTrue(pageAdapter.contains("WebBridgeReadinessPolicy.stateAfterPageReady"))
        assertTrue(pageAdapter.contains("ChatGptWebProtocol.parseMessage"))
        assertTrue(pageAdapter.contains("ownedComposer = true"))
        assertTrue(session.contains("GoogleWebConversationStore"))
        assertTrue(session.contains("conversationStore.acceptOfficial(event.conversations)"))
        assertTrue(session.contains("GoogleWebConversationSnapshotStore"))
        assertTrue(session.contains("GoogleWebSnapshotPresentation.opening"))
        val openConversation = session.substringAfter("fun openConversation(path: String): Boolean")
            .substringBefore("fun openProject(path: String): Boolean")
        assertTrue(
            openConversation.indexOf("conversationSnapshotStore.restore(path)") <
                openConversation.indexOf("onSnapshot"),
        )
        assertTrue(openConversation.indexOf("onSnapshot") < openConversation.indexOf("loadUrl"))
        assertTrue(session.contains("ChatGptWebProxyController"))
        assertTrue(session.contains("nextSnapshot.composerReady && !nextSnapshot.streaming"))
        assertTrue(session.contains("event.ok || event.action == \"send_prompt\""))
        assertTrue(session.contains("responseRefresh.onSendConfirmed()"))
        assertTrue(session.contains("responseRefresh.onSnapshot("))
        assertTrue(responseRefresh.contains("DEFAULT_DELAYS_MS"))
        assertTrue(session.contains("fun currentOfficialUrl(): String?"))
        assertTrue(session.contains("conversationStore::restorableUrl"))
        assertTrue(officialActivity.contains("intent.getStringExtra(EXTRA_START_URL)"))
        assertTrue(officialActivity.contains("sanitizeRestorableUrl(requestedUrl)"))
        assertTrue(modeController.contains("officialFallbackUrl()"))
        val controller = read("android/app/src/main/kotlin/com/elon/app/GoogleWebSocialChatController.kt")
        assertTrue(controller.contains("session.currentOfficialUrl()"))
        assertTrue(controller.contains("OfficialPageWebChatSendTransport("))
        assertTrue(controller.contains("WebChatSendCoordinator("))
        assertTrue(controller.contains("sendCoordinator.acceptCommandResult(event.ok)"))
        assertTrue(controller.contains("sendCoordinator.observeSnapshot(snapshot)"))
        assertTrue(controller.contains("session.onSubmissionObserved()"))
        assertTrue(session.contains("fun onSubmissionObserved() = responseRefresh.onSendConfirmed()"))
        assertTrue(controller.contains("sendCoordinator.status()"))
        assertTrue(controller.contains("?.sendStatus = pendingStatus"))
        assertTrue(controller.contains("session.currentSnapshot()?.let(::renderSnapshot)"))
        assertTrue(controller.contains("restorePrompt(failedPrompt)"))
        assertTrue(sendCoordinator.contains("state.onConfirmationTimeout(generation)"))
        assertTrue(sendCoordinator.contains("transport::reconcile"))
        assertTrue(sendCoordinator.contains("armWatchdog(generation)"))
        assertTrue(controller.contains("TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION"))
        assertTrue(controller.contains("sendCoordinator.requiresOfficialConfirmation()"))
        assertTrue(sendCoordinator.contains("removeCallbacks"))
        assertTrue(controller.contains("session.requestConversationIndex()"))
        assertTrue(pendingSendState.contains("TimeoutAction.KEEP_WAITING"))
        assertTrue(pendingSendState.contains("TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION"))
        assertTrue(pendingSendState.contains("TimeoutAction.RESTORE"))
        assertTrue(pendingSendState.contains("OFFICIAL_CONFIRMATION"))
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
