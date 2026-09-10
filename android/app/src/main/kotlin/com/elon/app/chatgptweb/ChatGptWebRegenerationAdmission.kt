package com.elon.app.chatgptweb

/** Native command admission is not a DOM scan. The runtime still validates the write owner. */
internal object ChatGptWebRegenerationAdmission {
    fun rejection(snapshot: ChatGptWebSnapshot?): String? {
        if (snapshot?.streaming == true) return "generation_in_progress"
        val assistant = snapshot?.messages?.lastOrNull { it.role == "assistant" }
        if (assistant == null || assistant.id.isBlank() || assistant.state != "completed") {
            return "regenerate_unavailable"
        }
        return null
    }
}
