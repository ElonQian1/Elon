package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebProjectJsonTest {
    @Test
    fun exposesStableNativeProjectActions() {
        val value = ChatGptWebProjectJson.encode(
            listOf(ChatGptWebProject("g-p-demo", "移动项目", "/g/g-p-demo/project", true)),
        ).getJSONObject(0)

        assertEquals("g-p-demo", value.getString("id"))
        assertEquals("/g/g-p-demo/project", value.getString("path"))
        assertEquals(true, value.getBoolean("active"))
        assertEquals("open_web_chat_project", value.getString("native_action"))
        assertEquals(
            "chatgpt-project:g-p-demo:移动项目",
            value.getString("native_adb_content_description"),
        )
    }
}
