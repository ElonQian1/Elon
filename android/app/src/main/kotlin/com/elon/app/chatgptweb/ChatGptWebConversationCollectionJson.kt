package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebConversationCollectionJson {
    fun encode(value: ChatGptWebConversationCollection): JSONObject = JSONObject()
        .put("scroller_found", value.scrollerFound)
        .put("scrolled", value.scrolled)
        .put("scroll_restored", value.scrollRestored)
        .put("reached_end", value.reachedEnd)
        .put("truncated", value.truncated)
        .put("timed_out", value.timedOut)
        .put("complete", value.isComplete)
        .put("observed_count", value.observedCount)
        .put("steps", value.steps)
        .put("source", value.source)
        .put("stale", value.stale)
        .put("official_load_state", value.officialLoadState)
        .put("cached_at_ms", value.cachedAtMs)
}
