package com.elon.acceptance;

import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** External semantic acceptance; never installed into the consumer APK. */
public final class LibraryUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";

    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }

    private UiObject text(String value) {
        return new UiObject(new UiSelector().packageName(APP).text(value));
    }

    private UiObject libraryFeature() {
        UiObject preset = description("web-chat-feature:library");
        if (preset.exists()) return preset;
        return new UiObject(new UiSelector().packageName(APP).descriptionMatches(
            "chatgpt-feature:[a-zA-Z0-9_]+:(\u8d44\u6599\u5e93|\u6587\u4ef6\u5e93)"));
    }

    private void click(UiObject node) throws Exception {
        assertTrue("semantic_control_missing", node.waitForExists(8000));
        assertTrue("semantic_control_disabled", node.isEnabled());
        android.view.accessibility.AccessibilityNodeInfo info = accessibilityNode(node);
        try {
            assertTrue("semantic_click_failed", info.performAction(
                android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }

    private android.view.accessibility.AccessibilityNodeInfo accessibilityNode(UiObject node) throws Exception {
        // Use the resolved accessibility node, not UiObject's coordinate injection.
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        android.view.accessibility.AccessibilityNodeInfo info =
            (android.view.accessibility.AccessibilityNodeInfo) method.invoke(node, 8000L);
        assertNotNull("semantic_node_missing", info);
        return info;
    }

    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        String handle = getParams().getString("handle", "");
        switch (step) {
            case "browse":
                click(description("web-chat-feature-navigation:chatgpt_web"));
                click(libraryFeature());
                assertTrue("library_not_visible", description("web-chat-library-browser").waitForExists(8000));
                break;
            case "features":
                click(description("web-chat-feature-navigation:chatgpt_web"));
                break;
            case "library":
                click(libraryFeature());
                assertTrue("library_not_visible", description("web-chat-library-browser").waitForExists(8000));
                break;
            case "query":
                assertTrue("library_not_visible", description("web-chat-library-browser").exists());
                android.view.accessibility.AccessibilityNodeInfo query = accessibilityNode(description("web-chat-library-query"));
                android.os.Bundle value = new android.os.Bundle();
                value.putCharSequence(android.view.accessibility.AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "ELON");
                try { assertTrue("query_not_set", query.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_SET_TEXT, value)); }
                finally { query.recycle(); }
                click(description("web-chat-library-search"));
                break;
            case "file":
                assertTrue("invalid_handle", handle.matches("[a-zA-Z0-9_-]{1,160}"));
                click(description("web-chat-library-entry:" + handle));
                break;
            case "attach":
                // This only stages a reference. It does not send, rename or delete a file.
                click(text("\u52a0\u5165\u5f53\u524d\u804a\u5929"));
                break;
            case "remove_staged":
                assertTrue("invalid_attachment_handle", handle.matches("(?:private_)?attachment_[a-zA-Z0-9_-]{1,160}"));
                click(description("web-chat-composer-attachment-remove:" + handle));
                break;
            case "close_detail":
                click(text("\u5173\u95ed"));
                break;
            case "back":
                click(description("web-chat-library-back"));
                break;
            case "inspect":
            case "inspect_entry":
                break;
            default:
                fail("unsupported_step");
        }
        JSONObject result = new JSONObject();
        result.put("step", step);
        result.put("library_visible", description("web-chat-library-browser").exists());
        result.put("search_visible", description("web-chat-library-query").exists());
        result.put("refresh_visible", description("web-chat-library-refresh").exists());
        result.put("back_visible", description("web-chat-library-back").exists());
        result.put("attach_visible", text("\u52a0\u5165\u5f53\u524d\u804a\u5929").exists());
        result.put("download_visible", text("\u4e0b\u8f7d").exists());
        result.put("rename_visible", text("\u91cd\u547d\u540d").exists());
        result.put("trash_visible", text("\u79fb\u5230\u6700\u8fd1\u5220\u9664").exists());
        result.put("entry_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-library-entry:")).exists());
        result.put("composer_attachment_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-composer-attachment:")).exists());
        result.put("composer_remove_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-composer-attachment-remove:")).exists());
        if (step.equals("inspect_entry")) {
            android.view.accessibility.AccessibilityNodeInfo node = accessibilityNode(
                description("web-chat-library-entry:" + handle));
            try {
                android.graphics.Rect bounds = new android.graphics.Rect();
                node.getBoundsInScreen(bounds);
                result.put("entry_clickable", node.isClickable());
                result.put("entry_enabled", node.isEnabled());
                result.put("entry_visible", node.isVisibleToUser());
                result.put("entry_actions", node.getActions());
                result.put("entry_bounds", bounds.toShortString());
            } finally { node.recycle(); }
        }
        android.os.Bundle report = new android.os.Bundle();
        report.putString("stream", "LIBRARY_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
