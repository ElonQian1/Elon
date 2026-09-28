package com.elon.acceptance;

import android.os.Bundle;
import android.os.SystemClock;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** Read-only record acceptance. Exports readiness and timing, never message content. */
public final class ChatRecordUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private UiObject text(String label) { return new UiObject(new UiSelector().packageName(APP).text(label)); }
    private UiObject desc(String label) { return new UiObject(new UiSelector().packageName(APP).description(label)); }
    private UiObject ready() { return new UiObject(new UiSelector().packageName(APP).textMatches("^\\d+ 条记录 · 微信导出$")); }
    private AccessibilityNodeInfo node(UiObject control) throws Exception {
        assertTrue("record_control_missing", control.waitForExists(3000));
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) method.invoke(control, 1000L);
        assertNotNull("record_node_missing", info);
        assertEquals("record_owner_mismatch", APP, String.valueOf(info.getPackageName()));
        return info;
    }
    private void click(UiObject control) throws Exception {
        AccessibilityNodeInfo info = node(control);
        try {
            for (int n = 0; !info.isClickable() && n < 5; n++) {
                AccessibilityNodeInfo parent = info.getParent();
                assertNotNull("record_parent_missing", parent);
                assertEquals("record_parent_owner_mismatch", APP, String.valueOf(parent.getPackageName()));
                info.recycle(); info = parent;
            }
            assertTrue("record_control_disabled", info.isEnabled());
            assertTrue("record_click_failed", info.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    private void scroll(boolean forward) throws Exception {
        UiObject list = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList"));
        AccessibilityNodeInfo info = node(list);
        try { info.performAction(forward ? AccessibilityNodeInfo.ACTION_SCROLL_FORWARD : AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD); } finally { info.recycle(); }
        Thread.sleep(250);
    }
    public void testStep() throws Exception {
        assertEquals("record_foreground_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        JSONObject result = new JSONObject().put("step", step).put("content_exported", false);
        if (step.equals("open_group")) {
            String group = new String(android.util.Base64.decode(getParams().getString("group_b64", ""), 0), java.nio.charset.StandardCharsets.UTF_8);
            assertTrue("record_group_required", !group.isEmpty() && group.length() <= 120);
            if (!text(group).exists() && text("群聊").exists()) click(text("群聊"));
            click(text(group));
        } else if (step.equals("open") || step.equals("reopen")) {
            if (step.equals("reopen")) { assertTrue("record_reader_required", ready().exists()); click(desc("返回")); }
            UiObject card = new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("查看聊天记录 ").instance(0));
            for (int n = 0; !card.exists() && n < 8; n++) scroll(false);
            for (int n = 0; !card.exists() && n < 16; n++) scroll(true);
            assertTrue("record_card_missing", card.exists());
            long started = SystemClock.elapsedRealtime();
            click(card);
            result.put("ready", ready().waitForExists(15000));
            result.put("open_to_accessibility_ready_ms", SystemClock.elapsedRealtime() - started);
        } else if (step.equals("cache_menu")) {
            assertTrue("record_reader_required", ready().exists()); click(desc("更多")); click(text("缓存管理"));
            assertTrue("record_cache_dialog_missing", text("清理当前").waitForExists(5000));
            result.put("clear_current", text("清理当前").exists()).put("clear_all", text("清理全部").exists());
            click(text("关闭"));
        } else if (step.equals("back")) {
            click(desc("返回"));
        } else { assertEquals("record_step_unknown", "inspect", step); }
        result.put("record_card_visible", new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("查看聊天记录 ")).exists())
            .put("chat_list_visible", new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList")).exists())
            .put("record_visible", ready().exists()).put("loading", text("正在读取…").exists())
            .put("background_validation", text("正在后台校验…").exists());
        Bundle report = new Bundle(); report.putString("stream", "CHAT_RECORD_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
