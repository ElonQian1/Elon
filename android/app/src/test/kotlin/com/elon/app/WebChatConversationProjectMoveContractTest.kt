package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatConversationProjectMoveContractTest {
    @Test
    fun pendingDraftIsProtectedBeforeRecoveryOrOfficialNavigationStarts() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun beginMove")
        val end = source.indexOf("private fun pollUntilReady", start)
        assertTrue(start >= 0 && end > start)
        val begin = source.substring(start, end)
        val guard = begin.indexOf("blocksForDraft(targetPath)")

        assertTrue(guard >= 0)
        assertTrue(guard < begin.indexOf("holdConversationRefresh()"))
        assertTrue(guard < begin.indexOf("recoveryStore.prepare(conversation, destination)"))
        assertTrue(guard < begin.indexOf("openConversation(targetPath)"))
        assertTrue(source.contains("ui.showDraftBlocked()"))
        assertTrue(source.contains("navigationRequestId = navigation.requestId"))
        assertTrue(source.contains("WebChatConsumerCommandStatus.FAILED"))
    }

    @Test
    fun controlRefreshPollsManifestWithoutTreatingDispatchReceiptAsDomReadiness() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveReadTransition.kt",
        )
        val start = source.indexOf("fun refreshControls(")
        val end = source.lastIndexOf('}')
        assertTrue(start >= 0 && end > start)
        val refresh = source.substring(start, end)

        assertTrue(refresh.contains("port.requestControls()"))
        assertTrue(refresh.contains("host.postDelayed"))
        assertTrue(refresh.contains("if (isCurrent(epoch)) continuation()"))
        assertFalse(refresh.contains("waitForCommand("))
    }

    @Test
    fun menuControlsUseObservedStateInsteadOfWaitingForFragileDomReceipts() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveReadTransition.kt",
        )
        val start = source.indexOf("fun invoke(")
        val end = source.indexOf("fun refreshControls(", start)
        assertTrue(start >= 0 && end > start)
        val transition = source.substring(start, end)

        assertTrue(transition.contains("port.invokeControl(control.control.id"))
        assertTrue(transition.contains("if (!result.accepted)"))
        assertTrue(transition.contains("if (isCurrent(epoch)) onAccepted()"))
        assertFalse(transition.contains("port.requestControls()"))
        assertFalse(transition.contains("waitForCommand("))
        assertFalse(transition.contains("requestId"))
    }

    @Test
    fun bothReadOnlyMenuLevelsUseTheReceiptIndependentTransition() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val transitionSource = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveReadTransition.kt",
        )
        val optionsStart = source.indexOf("private fun openConversationOptions")
        val optionsEnd = source.indexOf("private fun pollForConversationOptions", optionsStart)
        val triggerStart = transitionSource.indexOf("fun waitForMoveTrigger")
        val triggerEnd = transitionSource.lastIndexOf('}')
        assertTrue(optionsStart >= 0 && optionsEnd > optionsStart)
        assertTrue(triggerStart >= 0 && triggerEnd > triggerStart)
        val options = source.substring(optionsStart, optionsEnd)
        val trigger = transitionSource.substring(triggerStart, triggerEnd)

        assertTrue(options.contains("readTransition.invoke("))
        assertTrue(options.contains("waitForMoveTrigger("))
        assertTrue(source.contains("readTransition.waitForMoveTrigger("))
        assertTrue(trigger.contains("invoke("))
        assertTrue(trigger.contains("shouldRetryConversationOptions("))
        assertTrue(trigger.contains("retryableConversationOptions("))
        assertTrue(trigger.contains("optionsOpenRetries = optionsOpenRetries + 1"))
        assertTrue(trigger.contains("userConfirmed = false"))
        assertFalse(options.contains("waitForCommand("))
        assertFalse(trigger.contains("waitForCommand("))
    }

    @Test
    fun projectMoveProgressDoesNotPlaceAModalDialogOverTheBackgroundWebView() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveUi.kt",
        )
        val start = source.indexOf("fun showProgress")
        val end = source.indexOf("fun complete", start)
        assertTrue(start >= 0 && end > start)
        val progress = source.substring(start, end)

        assertTrue(progress.contains("Snackbar.make("))
        assertTrue(progress.contains("Snackbar.LENGTH_INDEFINITE"))
        assertTrue(progress.contains("web-chat-conversation-project-move-progress"))
        assertFalse(progress.contains("WebChatActionSheet.showUpdatable("))
        assertFalse(progress.contains("BottomSheetDialog"))
    }

    @Test
    fun nativeSheetsDismissAndSettleBeforeDrivingTheBackgroundOfficialPage() {
        val projectMove = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val projectMoveUi = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveUi.kt",
        )
        val projectPicker = projectMoveUi.substringAfter("fun showDestinationPicker")
            .substringBefore("fun showProgress")
        assertTrue(projectPicker.contains("var selectedDestination"))
        assertTrue(projectPicker.contains("onDismissed = {"))
        assertTrue(projectPicker.contains("host.postDelayed"))
        assertTrue(projectPicker.contains("ACTION_SHEET_HANDOFF_SETTLE_MS"))
        assertTrue(projectMove.contains("onSelected = { destination ->"))
        assertTrue(projectMove.contains("beginMove(conversation, destination)"))

        val actions = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationActions.kt",
        )
        val actionPicker = actions.substringAfter("fun show(conversation")
            .substringBefore("private fun showPageActionsFor")
        assertTrue(actionPicker.contains("var selectedActionId"))
        assertTrue(actionPicker.contains("onDismissed = {"))
        assertTrue(actionPicker.contains("host.postDelayed"))
        assertTrue(actionPicker.contains("dispatchSelectedAction(actionId, conversation)"))
        assertTrue(actionPicker.contains("ACTION_SHEET_HANDOFF_SETTLE_MS"))
    }

    @Test
    fun aLostWriteReceiptUsesReadOnlyReconciliationWithoutReplayingTheChoice() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun waitForProjectChoice")
        val end = source.indexOf("private fun invokeAndWait", start)
        assertTrue(start >= 0 && end > start)
        val choice = source.substring(start, end)

        assertTrue(
            Regex("""onSucceeded\s*=\s*\{\s*beginReadOnlyReconciliation""")
                .containsMatchIn(choice),
        )
        assertTrue(choice.contains("onFailed = {"))
        assertTrue(choice.contains("sourceProjectId(conversation)"))
        assertTrue(choice.contains("private fun beginReadOnlyReconciliation"))
        assertTrue(choice.contains("reconciler.begin("))
        assertTrue(choice.split("port.invokeControl(choice.control.id").size - 1 == 1)
        assertFalse(choice.contains("onFailed = { fail(conversation, destination"))
    }

    @Test
    fun durableRecoveryIsWrittenBeforeTheSingleOfficialWrite() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val prepare = source.indexOf("recoveryStore.prepare(conversation, destination)")
        val arm = source.indexOf("recoveryStore.armWrite()")
        val write = source.indexOf("port.invokeControl(choice.control.id")

        assertTrue(prepare >= 0)
        assertTrue(arm > prepare)
        assertTrue(write > arm)
        assertTrue(source.split("port.invokeControl(choice.control.id").size - 1 == 1)
    }

    @Test
    fun virtualizedProjectChoicesAreRevealedReadOnlyBeforeTheWriteIsArmed() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun waitForProjectChoice")
        val end = source.indexOf("private fun beginReadOnlyReconciliation", start)
        assertTrue(start >= 0 && end > start)
        val choice = source.substring(start, end)
        val reveal = choice.indexOf("port.revealProjectChoice(destination.title)")
        val arm = choice.indexOf("recoveryStore.armWrite()")

        assertTrue(reveal >= 0)
        assertTrue(arm >= 0)
        assertTrue(choice.contains("if (!projectChoiceRevealRequested)"))
        assertTrue(choice.split("port.revealProjectChoice(destination.title)").size - 1 == 1)
        assertTrue(choice.split("port.invokeControl(choice.control.id").size - 1 == 1)
        assertTrue(arm < reveal)
        assertFalse(choice.substring(reveal).contains("recoveryStore.armWrite()"))
    }

    @Test
    fun unavailableCachedDestinationFallsBackToOfficiallyRenderedProjectsBeforeAnyWrite() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveDestinationFallback.kt",
        )
        val start = source.indexOf("fun show(")
        val end = source.lastIndexOf('}')
        assertTrue(start >= 0 && end > start)
        val fallback = source.substring(start, end)

        assertTrue(fallback.contains("officialDestinations("))
        assertTrue(fallback.contains("ui.showDestinationPicker("))
        assertTrue(fallback.contains("recoveryStore.clear()"))
        assertTrue(fallback.indexOf("recoveryStore.prepare(conversation, selected)") >
            fallback.indexOf("recoveryStore.clear()"))
        assertFalse(fallback.contains("recoveryStore.armWrite()"))
        assertFalse(fallback.contains("port.invokeControl("))
    }

    @Test
    fun failedReadOnlyRevealReceiptOffersOfficialDestinationsWithoutWaitingForTimeout() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun waitForProjectChoice")
        val end = source.indexOf("private fun beginReadOnlyReconciliation", start)
        assertTrue(start >= 0 && end > start)
        val choice = source.substring(start, end)

        assertTrue(choice.contains("projectChoiceRevealRequestId = result.requestId"))
        assertTrue(choice.contains("commandStatus("))
        assertTrue(choice.contains("WebChatConsumerCommandStatus.FAILED"))
        assertTrue(choice.contains("WebChatConsumerCommandStatus.TIMED_OUT"))
        assertTrue(choice.contains("destinationFallback.show("))
    }

    @Test
    fun acceptedWriteWithoutARequestIdReconcilesInsteadOfReportingNotSubmitted() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun waitForProjectChoice")
        val end = source.indexOf("private fun beginReadOnlyReconciliation", start)
        assertTrue(start >= 0 && end > start)
        val choice = source.substring(start, end)

        assertTrue(choice.contains("if (result.requestId.isNullOrBlank())"))
        assertTrue(choice.contains("sourceProjectId(conversation)"))
        assertFalse(choice.contains("!result.accepted || result.requestId.isNullOrBlank()"))
    }

    @Test
    fun restartRecoveryIsReadOnlyAndNeverReplaysTheProjectChoice() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("fun recoverPending")
        val end = source.indexOf("fun cancelPending", start)
        assertTrue(start >= 0 && end > start)
        val recovery = source.substring(start, end)

        assertTrue(recovery.contains("beginReadOnlyReconciliation("))
        assertTrue(recovery.contains("sourceProjectId = record.sourceProjectId"))
        assertTrue(recovery.contains("allowConfirmation = false"))
        assertFalse(recovery.contains("invokeControl("))
        assertFalse(recovery.contains("armWrite("))
    }

    @Test
    fun readOnlyReconciliationUsesPrivateMembershipEvidenceBeforeDirectoryFallback() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatConversationProjectMoveReconciler.kt",
        )
        val start = source.indexOf("private fun requestMembershipReconciliation")
        assertTrue(start >= 0)
        val reconciliation = source.substring(start)

        assertTrue(reconciliation.contains(
            "probeConversationProject(conversation.path, destination.id)",
        ))
        assertTrue(reconciliation.contains("refreshConversationIndex(destination.id)"))
        assertTrue(reconciliation.contains("restartConversationRefreshGlobally()"))
        assertTrue(reconciliation.contains("refreshConversationIndex(null)"))
        assertFalse(reconciliation.contains("invokeControl("))
    }

    @Test
    fun authoritativeSourceMembershipClearsRecoveryAndOffersAUserRetry() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun settleNotApplied")
        val end = source.indexOf("private fun fail", start)
        assertTrue(start >= 0 && end > start)
        val settlement = source.substring(start, end)

        assertTrue(settlement.contains("writeAttempted = false"))
        assertTrue(settlement.contains("recoveryActive = false"))
        assertTrue(settlement.contains("recoveryStore.clear()"))
        assertTrue(settlement.contains("ui.showNotApplied("))
        assertTrue(settlement.contains("onRetry = { show(conversation) }"))
        assertFalse(settlement.contains("invokeControl("))
    }

    @Test
    fun reconciliationFailureCanRetryOnTheNextHostResume() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/" +
                "WebChatProductionConversationProjectMove.kt",
        )
        val start = source.indexOf("private fun fail(")
        val end = source.indexOf("private fun blocksForDraft", start)
        assertTrue(start >= 0 && end > start)
        val failure = source.substring(start, end)

        assertTrue(failure.contains("lastRecoveryAttemptKey = null"))
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
