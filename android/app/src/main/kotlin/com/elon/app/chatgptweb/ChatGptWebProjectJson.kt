package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebProjectJson {
    fun encode(values: List<ChatGptWebProject>): JSONArray = JSONArray().apply {
        values.forEach { project ->
            put(
                JSONObject()
                    .put("id", project.id)
                    .put("title", project.title)
                    .put("path", project.path)
                    .put("active", project.active)
                    .put("native_action", "open_web_chat_project")
                    .put(
                        "native_adb_content_description",
                        ChatGptNativeNavigationSelector.project(project),
                    ),
            )
        }
    }
}
