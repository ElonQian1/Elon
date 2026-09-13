package com.elon.acceptance;

import android.os.SystemClock;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONArray;
import org.json.JSONObject;

/** Reads the visible native directory page, without exporting titles or clicking conversations. */
public final class DirectoryUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }
    private void click(UiObject control) throws Exception {
        assertTrue("directory_control_missing", control.waitForExists(4000));
        java.lang.reflect.Method finder = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        finder.setAccessible(true);
        AccessibilityNodeInfo node = (AccessibilityNodeInfo) finder.invoke(control, 1000L);
        assertNotNull("directory_node_missing", node);
        try {
            assertTrue("directory_control_disabled", node.isEnabled());
            assertTrue("directory_click_failed", node.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { node.recycle(); }
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        switch (step) {
            case "open": click(description("web-chat-directory-browse")); break;
            case "next": click(description("\u4e0b\u4e00\u9875")); break;
            case "previous": click(description("\u4e0a\u4e00\u9875")); break;
            case "refresh": click(description("\u5237\u65b0\u5168\u90e8\u4f1a\u8bdd\u76ee\u5f55")); break;
            case "inspect": break;
            default: fail("directory_step_unsupported");
        }
        UiObject status = description("web-chat-directory-status");
        assertTrue("directory_status_missing", status.waitForExists(4000));
        long deadline = SystemClock.elapsedRealtime() + 20000;
        while (status.getText().startsWith("\u6b63\u5728\u8bfb\u53d6") && SystemClock.elapsedRealtime() < deadline) {
            Thread.sleep(300);
        }
        assertFalse("directory_still_loading", status.getText().startsWith("\u6b63\u5728\u8bfb\u53d6"));
        assertFalse("directory_failed", description("web-chat-directory-retry").exists());
        JSONArray rows = new JSONArray();
        for (int index = 0; index < 8; index++) {
            UiObject row = new UiObject(new UiSelector().packageName(APP)
                .descriptionStartsWith("web-chat-directory-conversation:").instance(index));
            if (!row.exists()) break;
            String id = row.getContentDescription().substring("web-chat-directory-conversation:".length());
            assertTrue("directory_id_invalid", id.matches("[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}"));
            rows.put(id);
        }
        System.out.println("DIRECTORY_UI_RESULT=" + new JSONObject().put("conversation_ids", rows)
            .put("next_enabled", description("\u4e0b\u4e00\u9875").isEnabled())
            .put("previous_enabled", description("\u4e0a\u4e00\u9875").isEnabled()).toString());
    }
}
