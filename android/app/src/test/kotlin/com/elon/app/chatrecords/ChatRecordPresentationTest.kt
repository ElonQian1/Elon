package com.elon.app.chatrecords

import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import android.app.Application

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = Application::class)
class ChatRecordPresentationTest {
    @Test fun sharedPresentationCasesKeepSourceUnchanged() {
        val cases = JSONArray(javaClass.classLoader!!.getResourceAsStream("chat-record-presentation.json")!!.bufferedReader().readText())
        for (i in 0 until cases.length()) {
            val c = cases.getJSONObject(i)
            val row = RecordRow("id", null, "示例", "", c.getString("kind"), c.getString("text"), c.optString("filename"))
            val raw = row.text
            val previews = c.optJSONArray("cards") ?: JSONArray()
            val cards = (0 until previews.length()).map { j -> previews.getJSONObject(j).let { ChatRecordPresentation.Card(it.getString("url"), it.getString("title"), it.getString("site")) } }
            assertEquals("case $i", c.getString("expected"), ChatRecordPresentation.text(row, cards))
            assertEquals(raw, row.text)
        }
    }
    @Test fun identitiesAndDurationAreStable() {
        assertEquals(ChatRecordPresentation.color("示例"), ChatRecordPresentation.color(" 示例 "))
        assertEquals("示", ChatRecordPresentation.initial("示例"))
        assertEquals("😀", ChatRecordPresentation.initial("😀示例"))
        assertEquals("0:56", ChatRecordPresentation.duration(56000))
        assertEquals("", ChatRecordPresentation.duration(0))
    }
}
