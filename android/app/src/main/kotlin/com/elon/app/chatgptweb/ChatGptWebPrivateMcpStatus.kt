package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebPrivateMcpStatus {
    fun stream(snapshot: ChatGptWebSnapshot?): JSONObject = JSONObject()
        .put("observed", snapshot?.privateStreamObserved ?: false)
        .put("revision", snapshot?.privateStreamRevision ?: 0L)
        .put("state", snapshot?.privateStreamState ?: "idle")

    fun readAloud(snapshot: ChatGptWebSnapshot?): JSONObject = JSONObject()
        .put("ready", snapshot?.privateReadAloudReady ?: false)
        .put("state", snapshot?.privateReadAloudState ?: "idle")
        .put("context_id", snapshot?.privateReadAloudContextId.orEmpty())

    fun observeOfficialControl(semantic: String) {
        if (semantic == "read_aloud") ChatGptWebPrivateResearchEventRecorder.beginVoiceWindow()
    }
}
