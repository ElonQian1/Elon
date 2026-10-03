package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProjectWebEntryTest {
    @Test fun httpsWebAppIsAvailableAlongsideResources() {
        val content = parseProjectIntroduction(JSONObject().put("web_url", "https://shop.example/"))!!
        assertEquals("https://shop.example/", content.webUrl)
        assertEquals("打开网页端", content.resources.single().label)
    }

    @Test fun untrustedSchemesAndCredentialsDoNotCreateAnAppEntry() {
        for (url in listOf("http://shop.example/", "javascript:alert(1)", "//shop.example/",
            "https://user:password@shop.example/", "https://shop.example/\\other")) {
            assertNull(parseProjectIntroduction(JSONObject().put("web_url", url))!!.webUrl)
        }
    }

    @Test fun projectsWithoutWebAppsKeepTheirExistingIntroduction() {
        val content = parseProjectIntroduction(JSONObject().put("summary", "项目简介"))!!
        assertNull(content.webUrl)
        assertEquals("项目简介", content.summary)
    }
}
