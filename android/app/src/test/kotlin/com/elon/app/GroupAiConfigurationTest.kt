package com.elon.app

import com.elon.app.chatgptweb.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupAiConfigurationTest {
    @Test fun workAndWebSettingsSurviveIndependentEngineSwitches() {
        val web = GroupAiConfiguration(modelPath = listOf(GroupAiModelChoice("高", rangeIndex = 2, rangeCount = 4)))
        val work = web.copy(engine = GroupAiEngine.WORK, work = GroupWorkAiConfiguration("work-b", "Model B", false))
        val restored = GroupAiConfigurationStore.decode(GroupAiConfigurationStore.encode(work))
        assertEquals(work, restored)
        assertEquals("Model B", restored.label)
        assertEquals("高", restored.copy(engine = GroupAiEngine.CHATGPT).label)
        assertEquals("work-b", restored.work.request().getString("agent"))
        assertFalse(restored.work.request().getBoolean("allow_fallback"))
        assertEquals(setOf("agent", "allow_fallback"), restored.work.request().keys().asSequence().toSet())
    }

    @Test fun legacyGroupConfigGetsWorkDefaultsWithoutLosingWebLevels() {
        val restored = GroupAiConfigurationStore.decode(JSONObject("""{"engine":"CHATGPT","model_path":[{"label":"高"}]}"""))
        assertEquals("高", restored.label)
        assertNull(restored.work.agent)
        assertTrue(restored.work.allowFallback)
        assertTrue(restored.work.request().isNull("agent"))
    }

    @Test fun workModelCatalogIsBoundedAndDeduplicated() {
        val rows = org.json.JSONArray("""[{"id":"a","label":"Model A","model":"v1"},{"id":"a","label":"duplicate"},{"id":""},null]""")
        assertEquals(listOf(GroupWorkAiModel("a", "Model A", "v1")), GroupWorkAiModel.parse(rows))
    }

    @Test fun configurationRoundTripAndScopeAreIndependent() {
        val config = GroupAiConfiguration(GroupAiEngine.WORK,
            listOf(GroupAiModelChoice("高级", true), GroupAiModelChoice("高", rangeIndex = 2, rangeCount = 4)))
        assertEquals(config, GroupAiConfigurationStore.decode(GroupAiConfigurationStore.encode(config)))
        assertNotEquals(GroupAiConfigurationStore.key("server", "one", "group"),
            GroupAiConfigurationStore.key("server", "two", "group"))
        assertNotEquals(GroupAiConfigurationStore.key("server", "one", "group"),
            GroupAiConfigurationStore.key("server", "one", "another-group"))
        assertNotEquals(GroupAiConfigurationStore.key("a|b", "c"), GroupAiConfigurationStore.key("a", "b|c"))
        assertEquals(GroupAiEngine.CHATGPT, GroupAiConfigurationStore.decode(JSONObject()).engine)
        assertEquals("高", config.copy(engine = GroupAiEngine.CHATGPT).label)
    }

    @Test fun missingOrAmbiguousControlsNeverAuthorizeSending() {
        val port = FakePort()
        var ready = 0
        val config = GroupWebAiModelConfiguration(port, listOf(GroupAiModelChoice("高")), { ready++ }, {})
        config.start()
        config.event(ChatGptWebEvent.CommandResult("snapshot", true, ""))
        config.event(ChatGptWebEvent.CommandResult("snapshot", false, ""))
        config.event(options())
        config.event(options(option("a", "高"), option("b", "高")))
        assertEquals(0, ready)
        assertNull(port.request)
    }

    @Test fun modelAndLevelAreAppliedBeforeSendAndOnlyOnce() {
        val port = FakePort()
        var ready = 0
        var failed = 0
        val config = GroupWebAiModelConfiguration(port,
            listOf(GroupAiModelChoice("高级", true), GroupAiModelChoice("高", rangeIndex = 2, rangeCount = 4)),
            { ready++ }, { failed++ })
        config.start()
        config.event(options(option("advanced-live", "高级", true)))
        assertEquals("select:advanced-live", port.action)
        val first = port.request
        config.event(options(option("advanced-live", "高级", true)))
        assertEquals(first, port.request)
        config.event(ack(requireNotNull(first)))
        config.event(range(4))
        assertEquals("slider:level-live:2.0", port.action)
        assertEquals(0, ready)
        config.event(ack(requireNotNull(port.request)))
        assertEquals("dismiss", port.action)
        config.event(ack(requireNotNull(port.request)))
        config.event(ack(requireNotNull(port.request)))
        assertEquals(1, ready)
        assertEquals(0, failed)
    }

    @Test fun failedMutationNeverAuthorizesOrRetries() {
        val port = FakePort()
        var ready = 0
        var failed = 0
        val config = GroupWebAiModelConfiguration(port, listOf(GroupAiModelChoice("自动")), { ready++ }, { failed++ })
        config.start()
        config.event(options(option("new-auto-id", "自动")))
        config.event(ack(requireNotNull(port.request), false))
        config.event(options(option("new-auto-id", "自动")))
        assertEquals(0, ready)
        assertEquals(1, failed)
        assertEquals(1, port.writes)
    }

    @Test fun privateSubmenuCatalogMayArriveBeforeCommandReceipt() {
        val port = FakePort()
        var ready = 0
        val config = GroupWebAiModelConfiguration(port,
            listOf(GroupAiModelChoice("高级", true), GroupAiModelChoice("高")), { ready++ }, { fail() })
        config.start()
        config.event(options(option("advanced", "高级", true)))
        val parentRequest = requireNotNull(port.request)
        config.event(options(option("private-high", "高")))
        assertEquals("select:advanced", port.action)
        config.event(ack(parentRequest))
        assertEquals("select:private-high", port.action)
        assertEquals(0, ready)
        config.event(ack(requireNotNull(port.request)))
        config.event(ack(requireNotNull(port.request)))
        assertEquals(1, ready)
    }

    @Test fun changedRangeCannotSilentlySelectDifferentLevel() {
        val controls = GroupWebAiModelControls()
        controls.accept(range(3))
        assertNull(controls.resolve(GroupAiModelChoice("高", rangeIndex = 2, rangeCount = 4)))
        controls.accept(range(4))
        assertNotNull(controls.resolve(GroupAiModelChoice("高", rangeIndex = 2, rangeCount = 4)))
    }

    @Test fun defaultConfigurationNeedsNoModelNetworkRoundTrip() {
        val port = FakePort()
        var ready = 0
        GroupWebAiModelConfiguration(port, emptyList(), { ready++ }, { fail() }).start()
        assertEquals(1, ready)
        assertEquals(0, port.writes)
        assertEquals("", port.action)
    }

    private fun option(id: String, label: String, submenu: Boolean = false) =
        ChatGptWebComposerOption(id, label, false, "button", "model", submenu)
    private fun options(vararg values: ChatGptWebComposerOption) = ChatGptWebEvent.ComposerControls("model", "", values.toList())
    private fun ack(id: String, ok: Boolean = true) = ChatGptWebEvent.CommandResult("model", ok, "", id)
    private fun range(count: Int) = ChatGptWebEvent.UiManifest(ChatGptWebUiManifest(1, "chat", "", "compatible",
        listOf(ChatGptWebUiControl("level-live", "model", "档位", "overlay", "slider", true, false,
            inputKind = "range", writable = true, slider = ChatGptWebSlider(0.0, count - 1.0, 1.0, 0.0)))))

    private class FakePort : GroupAiModelPort {
        var action = ""
        var request: String? = null
        var writes = 0
        override fun list() {}
        override fun collect() {}
        override fun manifest() {}
        override fun select(id: String, request: String) { action = "select:$id"; this.request = request; writes++ }
        override fun slider(id: String, value: Double, request: String) { action = "slider:$id:$value"; this.request = request; writes++ }
        override fun dismiss(request: String) { action = "dismiss"; this.request = request }
    }
}
