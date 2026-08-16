package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionMessageActionLayoutContractTest {
    @Test
    fun consumerMessageActionsKeepAccessibleTouchTargets() {
        assertActionTargets(
            "android/app/src/main/res/layout/item_message_friend.xml",
            listOf("webChatMessageCopy", "webChatMessageRegenerate", "webChatMessageMore"),
        )
        assertActionTargets(
            "android/app/src/main/res/layout/item_message_user.xml",
            listOf("webChatMessageCopy", "webChatMessageMore"),
        )
    }

    private fun assertActionTargets(relativePath: String, ids: List<String>) {
        val source = read(relativePath)
        assertTrue(source.element("webChatMessageActionBar").contains("android:layout_height=\"48dp\""))
        ids.forEach { id ->
            val element = source.element(id)
            assertTrue("$id width", element.contains("android:layout_width=\"48dp\""))
            assertTrue("$id height", element.contains("android:layout_height=\"48dp\""))
        }
    }

    private fun String.element(id: String): String = substringAfter("android:id=\"@+id/$id\"")
        .substringBefore("/>")

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
