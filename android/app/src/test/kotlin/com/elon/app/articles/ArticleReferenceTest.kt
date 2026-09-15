package com.elon.app.articles

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ArticleReferenceTest {
    private fun reference(id: String = "article_example", revision: Any = 2) = ArticleApi.PREFIX + JSONObject()
        .put("schema", 1).put("article_id", id).put("revision", revision)
        .put("title", "图文周报").put("summary", "给群友的摘要").toString()
    @Test fun validCardKeepsExactPublishedRevision() {
        val result = ArticleApi.reference(reference())!!
        assertEquals("article_example", result.getString("article_id"))
        assertEquals(2L, result.getLong("revision"))
    }
    @Test fun ordinaryMessagesAndMalformedCardsRemainText() {
        assertNull(ArticleApi.reference("普通群消息"))
        assertNull(ArticleApi.reference(ArticleApi.PREFIX + "{broken"))
        assertNull(ArticleApi.reference(reference().replace("\"schema\":1", "\"schema\":9")))
    }
    @Test fun CardCannotRequestArbitraryPathOrFractionalRevision() {
        assertNull(ArticleApi.reference(reference("../../private")))
        assertNull(ArticleApi.reference(reference(revision = 0)))
        assertNull(ArticleApi.reference(reference(revision = 1.5)))
        assertNull(ArticleApi.reference(reference(revision = "2")))
    }
}
