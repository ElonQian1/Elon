package com.elon.acceptance;

import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

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
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        switch (step) {
            case "model":
                click(modelButton());
                assertTrue("model_menu_missing", description("web-chat-model-control").waitForExists(5000));
                break;
            case "model_advanced": click(description("web-chat-model-advanced")); break;
            case "select_model":
                String selector = getParams().getString("selector", "");
                assertTrue("invalid_model_selector", selector.matches(
                    "(web-chat-model-(option|parent|preset):|chatgpt-option:model:)[A-Za-z0-9_.:-]{1,140}"));
                click(description(selector)); break;
            case "tools": click(description("web-chat-composer-tools:chatgpt_web")); break;
            case "image": click(description("web-chat-composer-tool:chatgpt_web:image_generation")); break;
            case "search": click(description("web-chat-composer-tool:chatgpt_web:web_search")); break;
            case "conversation_actions":
                click(new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("chatgpt-conversation-actions:")));
                assertTrue("conversation_actions_missing", description("web-chat-conversation-action-share").waitForExists(5000));
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
            .put("model_button", modelButton().exists())
            .put("model_menu", description("web-chat-model-control").exists())
            .put("level_slider", description("web-chat-model-level-slider").exists())
            .put("model_advanced", description("web-chat-model-advanced").exists())
            .put("model_preset", description("web-chat-model-preset:auto").exists())
            .put("image_option", description("web-chat-composer-tool:chatgpt_web:image_generation").exists())
            .put("search_option", description("web-chat-composer-tool:chatgpt_web:web_search").exists())
            .put("share_menu", description("web-chat-conversation-share-options").exists())
            .put("account_share_page", accountTitle())
            .put("account_share_rows", description("web-chat-account-share-links-list").exists())
            .put("account_share_empty", text("\u8fd9\u4e2a\u8d26\u53f7\u6ca1\u6709\u516c\u5f00\u5206\u4eab\u94fe\u63a5\u3002").exists())
            .put("share_failed", text("\u5206\u4eab\u94fe\u63a5\u5c1a\u672a\u786e\u8ba4").exists());
        android.os.Bundle report = new android.os.Bundle();
        report.putString("stream", "CONVERSATION_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
