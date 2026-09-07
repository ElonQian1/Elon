package com.elon.app.chatgptweb

import com.elon.app.ChatGptConsumerModelOptionMapper
import com.elon.app.WebChatModelControlPolicy
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebModelCatalogProtocolTest {
    @Test
    fun catalogSemanticsSurviveTheProductionParserAndMapper() {
        val event = ChatGptWebProtocol.parse(
            """{"schema":"yilong.ai.ui.v1","event":{
                "type":"composer_controls_snapshot","section":"model","currentModel":"Fast",
                "options":[
                    {"id":"private_model_2_back","label":"Back","opensSubmenu":true,"semantic":"model"},
                    {"id":"private_model_2_0","label":"Fast","semantic":"model_catalog"},
                    {"id":"private_model_2_1","label":"High","semantic":"model_catalog","selected":true},
                    {"id":"private_model_2_next","label":"Next","opensSubmenu":true,"semantic":"model"}
                ]
            }}""".trimIndent(),
        ) as ChatGptWebEvent.ComposerControls
        val options = event.options.mapNotNull(ChatGptConsumerModelOptionMapper::map)
        assertEquals("model_catalog", options[1].semantic)
        assertFalse(WebChatModelControlPolicy.resolve(options, "Fast").usesLevelSlider)
        assertFalse(WebChatModelControlPolicy.isSelected(options[1], "Fast"))
        assertTrue(WebChatModelControlPolicy.isSelected(options[2], "Fast"))
        assertEquals("private_model_2_next", options.last().id)
    }
}
