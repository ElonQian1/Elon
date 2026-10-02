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
    private void revealForTouch(UiObject card) throws Exception {
        UiObject list = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList"));
        UiObject input = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/bottomBarContainer"));
        for (int attempt = 0; attempt < 4; attempt++) {
            android.graphics.Rect viewport = list.getBounds();
            if (input.exists()) viewport.bottom = Math.min(viewport.bottom, input.getBounds().top);
            android.graphics.Rect bounds = card.getBounds();
            int margin = Math.max(16, viewport.height() / 12);
            if (bounds.bottom <= viewport.bottom - margin && bounds.top >= viewport.top) return;
            // The list extends behind the composer; accessibility existence is not touch visibility.
            boolean above = bounds.top < viewport.top;
            int startY = above ? viewport.top + margin : viewport.bottom - margin;
            int endY = above ? Math.min(viewport.bottom - margin, startY + viewport.height() / 3)
                : Math.max(viewport.top + margin, startY - viewport.height() / 3);
            assertTrue("record_reveal_swipe_failed", getUiDevice().swipe(viewport.centerX(), startY, viewport.centerX(), endY, 30));
            Thread.sleep(350);
        }
        android.graphics.Rect bounds = card.getBounds();
        assertTrue("record_card_occluded", !input.exists() || bounds.bottom < input.getBounds().top);
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
        } else if (step.equals("inspect_targets")) {
            UiObject card = new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("查看聊天记录 ").instance(0));
            for (int n = 0; !card.exists() && n < 8; n++) scroll(false);
            for (int n = 0; !card.exists() && n < 16; n++) scroll(true);
            result.put("card_exists", card.exists());
            if (card.exists()) {
                revealForTouch(card);
                result.put("card_bounds", card.getBounds().toShortString());
                result.put("list_bounds", new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/chatList")).getBounds().toShortString());
                UiObject input = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/bottomBarContainer"));
                if (input.exists()) result.put("input_bounds", input.getBounds().toShortString());
                org.json.JSONArray targets = new org.json.JSONArray();
                for (int index = 0; index < 3; index++) {
                    UiObject child = card.getChild(new UiSelector().className("android.widget.TextView").index(index));
                    targets.put(new JSONObject().put("index", index).put("exists", child.exists())
                        .put("clickable", child.isClickable()).put("bounds", child.getBounds().toShortString()));
                }
                result.put("targets", targets);
            }
        } else if (step.equals("open") || step.equals("reopen") || step.startsWith("tap_")) {
            if (step.equals("reopen")) { assertTrue("record_reader_required", ready().exists()); click(desc("返回")); }
            UiObject card = new UiObject(new UiSelector().packageName(APP).descriptionStartsWith("查看聊天记录 ").instance(0));
            for (int n = 0; !card.exists() && n < 8; n++) scroll(false);
            for (int n = 0; !card.exists() && n < 16; n++) scroll(true);
            assertTrue("record_card_missing", card.exists());
            if (step.startsWith("tap_")) revealForTouch(card);
            long started = SystemClock.elapsedRealtime();
            if (step.startsWith("tap_")) {
                int index = step.equals("tap_title") ? 0 : step.equals("tap_summary") ? 1 : step.equals("tap_footer") ? 2 : -1;
                assertTrue("record_tap_target_unknown", index >= 0);
                // Exercise Android touch dispatch, not ACTION_CLICK on the parent card.
                UiObject child = card.getChild(new UiSelector().className("android.widget.TextView").index(index));
                assertTrue("record_text_target_missing", child.waitForExists(3000));
                assertTrue("record_text_tap_failed", child.click());
            } else { click(card); }
            result.put("ready", ready().waitForExists(15000));
            result.put("open_to_accessibility_ready_ms", SystemClock.elapsedRealtime() - started);
            assertTrue("record_reader_not_opened", result.getBoolean("ready"));
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
