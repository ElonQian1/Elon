package com.elon.app.chatgptweb

import java.nio.file.Files
import java.nio.file.Paths
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

internal fun assertChatGptAssetsInOrder(vararg required: String) {
    var previous = -1
    required.forEach { name ->
        val index = ChatGptWebAdapterAssets.names.indexOf(name)
        assertTrue("Missing adapter asset: $name", index >= 0)
        assertTrue("Adapter asset is out of dependency order: $name", index > previous)
        previous = index
    }
}

class ChatGptWebAdapterAssetsTest {
    @Test
    fun registryContainsUniqueExistingAssetsAndIsConsumedByThePageAdapter() {
        val root = generateSequence(Paths.get(System.getProperty("user.dir")).toAbsolutePath()) { it.parent }
            .first { Files.isDirectory(it.resolve("android/app/src/main/assets")) }
        val names = ChatGptWebAdapterAssets.names
        assertTrue(names.isNotEmpty())
        assertEquals(names.size, names.toSet().size)
        names.forEach { name ->
            assertTrue("Invalid adapter asset name: $name", name.matches(Regex("chatgpt_web_[a-z0-9_]+\\.js")))
            assertTrue("Missing bundled adapter asset: $name", Files.isRegularFile(root.resolve("android/app/src/main/assets/$name")))
        }
        assertEquals("chatgpt_web_adapter_bootstrap.js", names.first())
        assertEquals("chatgpt_web_adapter.js", names.last())
        val page = String(Files.readAllBytes(root.resolve(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt",
        )), Charsets.UTF_8)
        assertTrue(page.contains("private val ADAPTER_ASSETS = ChatGptWebAdapterAssets.names"))
        assertTrue(page.contains("ADAPTER_ASSETS.joinToString"))
        assertTrue(page.contains("context.assets.open(asset)"))
    }

    @Test
    fun writingBlockParsersAndPoliciesLoadBeforeTheirConsumers() {
        assertChatGptAssetsInOrder(
            "chatgpt_web_text_blocks.js",
            "chatgpt_web_adapter_messages.js",
            "chatgpt_web_private_history_projection.js",
            "chatgpt_web_writing_block_policy.js",
            "chatgpt_web_writing_block_context.js",
            "chatgpt_web_private_writing_blocks.js",
            "chatgpt_web_adapter.js",
        )
    }
}
