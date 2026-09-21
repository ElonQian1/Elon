package com.elon.acceptance;

import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

/** Scoped external-reader acceptance. Semantic navigation only; never exports chat or page text. */
public final class SocialLinkUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private UiObject description(String value) { return new UiObject(new UiSelector().packageName(APP).description(value)); }
    private UiObject control(String value) { return new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/external_reader_" + value)); }
    private UiObject text(String value) { return new UiObject(new UiSelector().packageName(APP).text(value)); }
    private String argument(String key) {
        String value = new String(android.util.Base64.decode(getParams().getString(key + "_b64", ""), android.util.Base64.DEFAULT), StandardCharsets.UTF_8);
        assertTrue("missing_target", !value.isEmpty() && value.length() <= 300);
        return value;
    }
    private AccessibilityNodeInfo node(UiObject value) throws Exception {
        assertTrue("semantic_control_missing", value.waitForExists(3000));
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) method.invoke(value, 3000L);
        assertNotNull("semantic_node_missing", info);
        assertEquals("semantic_owner_mismatch", APP, String.valueOf(info.getPackageName()));
        assertTrue("semantic_control_disabled", info.isEnabled());
        return info;
    }
    private void click(UiObject value) throws Exception {
        AccessibilityNodeInfo info = node(value);
        try {
            for (int depth = 0; !info.isClickable() && depth < 5; depth++) {
                AccessibilityNodeInfo parent = info.getParent();
                assertNotNull("clickable_parent_missing", parent);
                if (!APP.contentEquals(parent.getPackageName())) { parent.recycle(); fail("foreign_parent"); }
                info.recycle(); info = parent;
            }
            assertTrue("semantic_click_failed", info.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        JSONObject result = new JSONObject().put("step", step).put("content_exported", false);
        switch (step) {
            case "open_group":
                click(text(argument("group")));
                assertTrue("chat_list_missing", new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList")).waitForExists(5000));
                break;
            case "open_article":
                // The preview may gain an author after read-back; match a supplied public-title prefix.
                UiObject article = new UiObject(new UiSelector().packageName(APP).descriptionStartsWith(argument("article")));
                UiObject list = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList"));
                for (int i = 0; !article.exists() && i < 12; i++) {
                    AccessibilityNodeInfo info = node(list);
                    boolean moved;
                    try { moved = info.performAction(AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD); }
                    finally { info.recycle(); }
                    if (!moved) break;
                    Thread.sleep(350);
                }
                click(article);
                assertTrue("reader_missing", control("refresh").waitForExists(5000));
                break;
            case "refresh": case "close": case "chat": case "original":
                click(control(step));
                break;
            case "inspect":
                result.put("reader_visible", control("refresh").exists())
                    .put("original_available", control("original").exists())
                    .put("close_available", control("close").exists());
                break;
            default: fail("unsupported_step");
        }
        Bundle report = new Bundle();
        report.putString("stream", "SOCIAL_LINK_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
