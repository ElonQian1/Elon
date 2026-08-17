package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatActionSheetContractTest {
    @Test
    fun primaryConsumerOptionListsUseTheSharedBottomSheet() {
        listOf(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionFeatureNavigation.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionPageActions.kt",
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
        ).forEach { path ->
            assertTrue("$path must use the consumer action sheet", read(path).contains("WebChatActionSheet.show"))
        }
    }

    @Test
    fun sharedSheetKeepsStableRowsAndDoesNotDependOnTheDiagnosticActivity() {
        val source = read("android/app/src/main/kotlin/com/elon/app/WebChatActionSheet.kt")

        assertTrue(source.contains("BottomSheetDialog"))
        assertTrue(source.contains("contentDescription = item.contentDescription"))
        assertTrue(source.contains("ITEM_HEIGHT_DP"))
        assertTrue(source.contains("onCancelled: () -> Unit"))
        assertTrue(source.contains("if (!handled) onCancelled()"))
        assertTrue(source.contains("onDismissed()"))
        assertFalse(source.contains("ChatGptWebTestActivity"))
    }

    @Test
    fun composerSheetsCloseTheHiddenOfficialMenuWhenCancelled() {
        val model = read("android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt")
        val tools = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt")

        assertTrue(model.contains("onCancelled = { socialConsumerPort.dismissComposerOptions() }"))
        assertTrue(tools.contains("onCancelled = { port.dismissComposerOptions() }"))
        val commandDispatch = tools.substringBefore("executeCommand(provider, port, it)")
        assertTrue(commandDispatch.endsWith("port.dismissComposerOptions()\r\n                ") ||
            commandDispatch.endsWith("port.dismissComposerOptions()\n                "))
    }

    @Test
    fun unavailableMessageActionsOfferTheOfficialConversationFallback() {
        val source = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
        )
        val noActions = source.substringAfter("if (actions.isEmpty())")
            .substringBefore("val byId")
        val dispatch = source.substringAfter("private fun dispatch")
            .substringBefore("private fun showFeedback")

        assertTrue(noActions.contains("showOfficialFallback"))
        assertTrue(dispatch.contains("setPositiveButton(\"打开官方页\")"))
        assertTrue(dispatch.contains("openOfficialFallback()"))
        assertFalse(noActions.contains("Toast.makeText"))
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(5) {
            if (Files.exists(current.resolve("android/app/src/main"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found from ${System.getProperty("user.dir")}")
    }
}
