package com.elon.acceptance;

import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;
import org.json.JSONArray;

/** External production-menu acceptance. Never exports conversation text or publishes links. */
public final class ConversationUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private UiObject description(String value) { return new UiObject(new UiSelector().packageName(APP).description(value)); }
    private UiObject text(String value) { return new UiObject(new UiSelector().packageName(APP).text(value)); }
    private UiObject modelButton() {
        return new UiObject(new UiSelector().packageName(APP).descriptionMatches(
            "\u804a\u5929\u6a21\u5f0f\uff1b\u63d0\u4f9b\u65b9\uff1aChatGPT.*\uff1b\u6a21\u578b\uff1a.*"));
    }
    private void click(UiObject node) throws Exception {
        assertTrue("semantic_control_missing", node.waitForExists(8000));
        assertTrue("semantic_control_disabled", node.isEnabled());
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) method.invoke(node, 8000L);
        assertNotNull("semantic_node_missing", info);
        try {
            for (int depth = 0; !info.isClickable() && depth < 4; depth++) {
                AccessibilityNodeInfo parent = info.getParent();
                if (parent == null) break;
                if (!APP.contentEquals(parent.getPackageName() == null ? "" : parent.getPackageName())) {
                    parent.recycle(); break;
                }
                info.recycle(); info = parent;
            }
            assertTrue("semantic_click_failed", info.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    private boolean accountTitle() {
        return text("\u5168\u90e8\u516c\u5f00\u5206\u4eab\u94fe\u63a5").exists() ||
            text("\u5168\u90e8\u516c\u5f00\u5206\u4eab\u94fe\u63a5\uff08\u90e8\u5206\uff09").exists();
    }
    private String retrySelector() {
        String selector = new String(android.util.Base64.decode(
            getParams().getString("selector_b64", ""), android.util.Base64.DEFAULT),
            java.nio.charset.StandardCharsets.UTF_8);
        assertTrue("invalid_reply_retry_selector", selector.matches(
            "^web-chat-message-action:chatgpt_web:[A-Za-z0-9_.:-]{1,160}:regenerate$"));
        return selector;
    }
    private JSONObject inspectReplyActions(String selector) throws Exception {
        String rowSelector = selector.replace("web-chat-message-action:", "web-chat-message:");
        rowSelector = rowSelector.substring(0, rowSelector.length() - ":regenerate".length());
        JSONArray buttons = new JSONArray();
        for (String kind : new String[] { "Copy", "Regenerate", "More" }) {
            for (int i = 0; i < 32; i++) {
                UiObject button = new UiObject(new UiSelector().packageName(APP)
                    .resourceId(APP + ":id/webChatMessage" + kind).instance(i));
                if (!button.exists()) break;
                android.graphics.Rect bounds = button.getVisibleBounds();
                buttons.put(new JSONObject().put("kind", kind).put("enabled", button.isEnabled())
                    .put("matches_target", selector.equals(button.getContentDescription()))
                    .put("bounds", new JSONArray(new int[] {bounds.left, bounds.top, bounds.right, bounds.bottom})));
            }
        }
        UiObject list = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList"));
        return new JSONObject().put("expected_button", description(selector).exists())
            .put("expected_row", description(rowSelector).exists())
            .put("native_list", list.exists()).put("native_list_children", list.exists() ? list.getChildCount() : -1)
            .put("native_message_text", new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/messageText")).exists())
            .put("skin_exit", description("web-chat-skin-exit:chatgpt").exists())
            .put("buttons", buttons);
    }
    private void setModelLevel() throws Exception {
        UiObject slider = description("web-chat-model-level-slider");
        assertTrue("model_level_slider_missing", slider.waitForExists(5000));
        assertTrue("model_level_slider_disabled", slider.isEnabled());
        int level = Integer.parseInt(getParams().getString("level", "-1"));
        assertTrue("invalid_model_level", level >= 0 && level <= 5);
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) method.invoke(slider, 5000L);
        assertNotNull("model_level_node_missing", info);
        try {
            assertEquals("model_level_owner_mismatch", APP, String.valueOf(info.getPackageName()));
            assertEquals("model_level_class_mismatch", "android.widget.SeekBar", String.valueOf(info.getClassName()));
            AccessibilityNodeInfo.RangeInfo range = info.getRangeInfo();
            assertNotNull("model_level_range_missing", range);
            assertTrue("model_level_out_of_range", level >= range.getMin() && level <= range.getMax());
            android.os.Bundle arguments = new android.os.Bundle();
            arguments.putFloat(AccessibilityNodeInfo.ACTION_ARGUMENT_PROGRESS_VALUE, level);
            assertTrue("model_level_action_failed", info.performAction(
                AccessibilityNodeInfo.AccessibilityAction.ACTION_SET_PROGRESS.getId(), arguments));
        } finally { info.recycle(); }
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        JSONObject replyActions = null;
        switch (step) {
            case "model":
                click(modelButton());
                assertTrue("model_menu_missing", description("web-chat-model-control").waitForExists(5000));
                break;
            case "model_advanced": click(description("web-chat-model-advanced")); break;
            case "model_level": setModelLevel(); break;
            case "select_model":
                String selector = new String(android.util.Base64.decode(
                    getParams().getString("selector_b64", ""), android.util.Base64.DEFAULT),
                    java.nio.charset.StandardCharsets.UTF_8);
                assertTrue("invalid_model_selector", selector.matches(
                    "(web-chat-model-(option|parent|preset):|chatgpt-option:model:)[A-Za-z0-9_.:-]{1,140}") || selector.matches(
                    "^chatgpt-composer-option:model:[A-Za-z0-9_.-]{1,96}:[^\\r\\n]{1,120}$"));
                click(description(selector)); break;
            case "tools": click(description("web-chat-composer-tools:chatgpt_web")); break;
            case "image": click(description("web-chat-composer-tool:chatgpt_web:image_generation")); break;
            case "search": click(description("web-chat-composer-tool:chatgpt_web:web_search")); break;
            case "clear_image": click(description("\u5173\u95ed\u521b\u5efa\u56fe\u7247")); break;
            case "clear_search": click(description("\u5173\u95ed\u7f51\u9875\u641c\u7d22")); break;
            case "header": click(description("web-chat-page-actions:chatgpt_web")); break;
            case "current_settings":
                click(text("\u4f1a\u8bdd\u8bbe\u7f6e"));
                assertTrue("conversation_actions_missing", description("web-chat-conversation-action-files").waitForExists(5000));
                break;
            case "temporary": click(description("chatgpt-native:temporary-chat:\u4e34\u65f6\u804a\u5929")); break;
            case "retry_session": click(description("web-chat-consumer-retry")); break;
            case "regenerate":
                click(description(retrySelector())); break;
            case "inspect_reply_actions":
                replyActions = inspectReplyActions(retrySelector()); break;
            case "conversation_actions":
                click(new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("chatgpt-conversation-actions:")));
                assertTrue("conversation_actions_missing", description("web-chat-conversation-action-share").waitForExists(5000));
                break;
            case "files":
                click(description("web-chat-conversation-action-files"));
                assertTrue("conversation_files_missing", description("web-chat-conversation-files-status").waitForExists(5000));
                break;
            case "files_refresh": click(description("web-chat-conversation-files-refresh")); break;
            case "file_row":
                int fileIndex = Integer.parseInt(getParams().getString("file_index", "-1"));
                assertTrue("invalid_file_index", fileIndex >= 0 && fileIndex < 160);
                assertTrue("conversation_files_missing", description("web-chat-conversation-files-status").exists());
                click(description("web-chat-conversation-file-" + fileIndex));
                assertTrue("conversation_file_download_missing", text("\u4e0b\u8f7d").waitForExists(5000));
                break;
            case "files_wait":
                UiObject status = description("web-chat-conversation-files-status");
                assertTrue("conversation_files_missing", status.waitForExists(5000));
                long filesDeadline = android.os.SystemClock.elapsedRealtime() + 18000;
                while (text("\u6b63\u5728\u66f4\u65b0").exists() && android.os.SystemClock.elapsedRealtime() < filesDeadline) Thread.sleep(250);
                assertFalse("conversation_files_still_loading", text("\u6b63\u5728\u66f4\u65b0").exists());
                assertFalse("conversation_files_failed", text("\u8bfb\u53d6\u5931\u8d25\uff0c\u53ef\u91cd\u8bd5").exists());
                assertTrue("conversation_files_result_missing", text("\u6b64\u4f1a\u8bdd\u6682\u65e0\u9644\u4ef6").exists() ||
                    text("\u90e8\u5206\u9644\u4ef6").exists() || new UiObject(new UiSelector().packageName(APP)
                        .textMatches("[0-9]+ \u4e2a\u9644\u4ef6")).exists());
                break;
            case "share":
                click(description("web-chat-conversation-action-share"));
                assertTrue("share_menu_missing", description("web-chat-conversation-share-options").waitForExists(5000));
                break;
            case "account_shares":
                click(text("\u7ba1\u7406\u5168\u90e8\u516c\u5f00\u94fe\u63a5"));
                break;
            case "share_list":
                long deadline = android.os.SystemClock.elapsedRealtime() + 22000;
                while (text("\u6b63\u5728\u8bfb\u53d6").exists() && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(250);
                assertTrue("account_share_page_missing", accountTitle());
                assertFalse("account_share_list_still_loading", text("\u6b63\u5728\u8bfb\u53d6").exists());
                break;
            case "next_shares": click(description("web-chat-account-shares-next")); break;
            case "previous_shares": click(description("web-chat-account-shares-previous")); break;
            case "close_shares":
                assertTrue("account_share_page_missing", accountTitle());
                click(text("\u5173\u95ed")); break;
            case "back": getUiDevice().pressBack(); break;
            case "inspect": break;
            default: fail("unsupported_step");
        }
        JSONObject result = new JSONObject().put("step", step)
            .put("reply_actions", replyActions)
            .put("file_index_visible", description("web-chat-conversation-files-status").exists())
            .put("file_index_first_row", description("web-chat-conversation-file-0").exists())
            .put("file_index_empty", text("\u6b64\u4f1a\u8bdd\u6682\u65e0\u9644\u4ef6").exists())
            .put("model_button", modelButton().exists())
            .put("model_menu", description("web-chat-model-control").exists())
            .put("level_slider", description("web-chat-model-level-slider").exists())
            .put("model_advanced", description("web-chat-model-advanced").exists())
            .put("model_preset", description("web-chat-model-preset:auto").exists())
            .put("image_option", description("web-chat-composer-tool:chatgpt_web:image_generation").exists())
            .put("search_option", description("web-chat-composer-tool:chatgpt_web:web_search").exists())
            .put("image_active", description("\u5df2\u542f\u7528\u521b\u5efa\u56fe\u7247").exists())
            .put("search_active", description("\u5df2\u542f\u7528\u7f51\u9875\u641c\u7d22").exists())
            .put("recovery_visible", description("web-chat-consumer-status").exists())
            .put("retry_visible", description("web-chat-consumer-retry").exists())
            .put("share_menu", description("web-chat-conversation-share-options").exists())
            .put("account_share_page", accountTitle())
            .put("account_share_rows", description("web-chat-account-share-links-list").exists())
            .put("account_share_empty", text("\u8fd9\u4e2a\u8d26\u53f7\u6ca1\u6709\u516c\u5f00\u5206\u4eab\u94fe\u63a5\u3002").exists())
            .put("share_failed", text("\u5206\u4eab\u94fe\u63a5\u5c1a\u672a\u786e\u8ba4").exists());
        UiObject recoveryText = description("web-chat-consumer-status").getChild(
            new UiSelector().className("android.widget.TextView").index(0));
        if (recoveryText.exists()) result.put("recovery_message", recoveryText.getText());
        android.os.Bundle report = new android.os.Bundle();
        report.putString("stream", "CONVERSATION_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
