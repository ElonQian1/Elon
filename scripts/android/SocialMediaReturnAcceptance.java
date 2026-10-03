package com.elon.acceptance;

import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

/** Opens an explicitly selected production card. Never reads or exports message bodies. */
public final class SocialMediaReturnAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private String argument(String key) {
        String value = new String(android.util.Base64.decode(getParams().getString(key + "_b64", ""), 0), StandardCharsets.UTF_8);
        assertTrue("missing_target", !value.isEmpty() && value.length() <= 300);
        return value;
    }
    private AccessibilityNodeInfo node(UiObject control) throws Exception {
        assertTrue("control_missing", control.waitForExists(2000));
        java.lang.reflect.Method find = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        find.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) find.invoke(control, 1000L);
        assertNotNull("node_missing", info);
        assertEquals("owner_mismatch", APP, String.valueOf(info.getPackageName()));
        return info;
    }
    private boolean scroll(int direction) throws Exception {
        AccessibilityNodeInfo list = node(new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList")));
        try { return list.performAction(direction); } finally { list.recycle(); }
    }
    public void testStep() throws Exception {
        String expected = getParams().getString("expected_package", "");
        assertTrue("invalid_platform", expected.equals("tv.danmaku.bili") || expected.equals("com.xingin.xhs") ||
            expected.equals("com.ss.android.ugc.aweme"));
        assertEquals("foreground_mismatch", APP, getUiDevice().getCurrentPackageName());
        assertTrue("group_mismatch", new UiObject(new UiSelector().packageName(APP)
            .resourceId(APP + ":id/topTitleText").text(argument("group"))).exists());
        UiObject card = new UiObject(new UiSelector().packageName(APP).description(argument("card")));
        String direction = getParams().getString("direction", "older");
        assertTrue("invalid_direction", direction.equals("older") || direction.equals("newer"));
        for (int i = 0; !card.exists() && i < 45; i++) {
            if (!scroll(direction.equals("older") ? AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD : AccessibilityNodeInfo.ACTION_SCROLL_FORWARD)) break;
            Thread.sleep(300);
        }
        AccessibilityNodeInfo selected = node(card);
        try {
            assertTrue("card_not_visible", selected.isVisibleToUser());
            assertTrue("card_not_clickable", selected.isClickable() && selected.isEnabled());
            assertTrue("card_click_failed", selected.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { selected.recycle(); }
        long deadline = android.os.SystemClock.elapsedRealtime() + 10000;
        while (!expected.equals(getUiDevice().getCurrentPackageName()) && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(200);
        JSONObject value = new JSONObject().put("card_clicked", true).put("content_exported", false)
            .put("expected_app_foreground", expected.equals(getUiDevice().getCurrentPackageName()));
        Bundle report = new Bundle();
        report.putString("stream", "SOCIAL_MEDIA_RETURN_RESULT=" + value + "\n");
        getAutomationSupport().sendStatus(0, report);
        assertTrue("target_app_not_open", value.getBoolean("expected_app_foreground"));
    }
}
