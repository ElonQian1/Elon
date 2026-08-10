package com.elon.app.chatgptweb

internal interface ChatGptWebMcpCommandPort {
    fun sendInput(requestId: String)
    fun invokeControl(controlId: String, requestId: String)
    fun newConversation(requestId: String)
    fun stopGeneration(requestId: String)
    fun cancelDictation(requestId: String)
    fun submitDictation(requestId: String)
    fun refreshControls(requestId: String)
    fun listConversations(requestId: String)
    fun requestComposerOptions(section: String, requestId: String)
    fun selectComposerOption(section: String, optionId: String, requestId: String)
    fun requestFeatures(requestId: String)
    fun selectFeature(featureId: String, requestId: String)
    fun openConversation(path: String, requestId: String)
}
