package com.elon.app

import java.util.UUID

internal class WebBridgeDocumentSession(
    private val tokenFactory: (Long) -> String = { generation ->
        "doc_${generation.toString(36)}_${UUID.randomUUID().toString().replace("-", "")}"
    },
) {
    data class Snapshot(
        val pageGeneration: Long,
        val adapterGeneration: Long,
        val documentToken: String,
    ) {
        val adapterCurrent: Boolean
            get() = pageGeneration > 0 && adapterGeneration == pageGeneration
    }

    private var pageGeneration = 0L
    private var adapterGeneration = 0L
    private var documentToken = ""

    fun beginPage(): Snapshot {
        pageGeneration++
        adapterGeneration = 0L
        documentToken = tokenFactory(pageGeneration).also { token ->
            require(DOCUMENT_TOKEN.matches(token)) { "Invalid web bridge document token" }
        }
        return snapshot()
    }

    fun ensurePage(): Snapshot = if (pageGeneration == 0L) beginPage() else snapshot()

    fun accept(token: String): Snapshot? {
        if (token != documentToken || !DOCUMENT_TOKEN.matches(token)) return null
        adapterGeneration = pageGeneration
        return snapshot()
    }

    fun snapshot(): Snapshot = Snapshot(pageGeneration, adapterGeneration, documentToken)

    companion object {
        internal val DOCUMENT_TOKEN = Regex("doc_[a-z0-9_]{3,80}")
    }
}
