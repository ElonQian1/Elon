package com.elon.app.chatgptweb

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSendOwnershipContractTest {
    @Test
    fun businessModulesCannotBypassTheProviderSendOwners() {
        val sourceRoot = root().resolve("android/app/src/main/kotlin")
        val allowed = setOf(
            "com/elon/app/chatgptweb/ChatGptWebSendOwner.kt",
            "com/elon/app/GoogleWebSocialChatController.kt",
            "com/elon/app/googleweb/GoogleWebBackgroundSession.kt",
        )
        val violations = mutableListOf<String>()
        Files.walk(sourceRoot).use { paths ->
            paths.filter { Files.isRegularFile(it) && it.fileName.toString().endsWith(".kt") }
                .forEach { file ->
                    val relative = sourceRoot.relativize(file).toString().replace('\\', '/')
                    Files.readAllLines(file).forEachIndexed { index, line ->
                        if (".sendPrompt(" in line && relative !in allowed) {
                            violations += "$relative:${index + 1}"
                        }
                    }
                }
        }
        violations.sort()

        assertEquals(emptyList<String>(), violations)
    }

    @Test
    fun chatGptSessionRoutesEverySendEntryThroughOneOwner() {
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val owner = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSendOwner.kt",
        )

        assertTrue(session.contains("private val sendOwner = ChatGptWebSendOwner("))
        assertTrue(session.contains("sendOwner.dispatchSocial(prompt)"))
        assertTrue(session.contains("sendOwner.dispatchMcp(inputText().trim(), requestId)"))
        assertTrue(session.contains("sendOwner.beginAttachments(prompt, attachments)"))
        assertTrue(session.contains("sendOwner.acceptCommandResult(event)"))
        assertFalse(session.contains(".sendPrompt("))
        assertEquals(1, Regex("\\.sendPrompt\\(").findAll(owner).count())
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
