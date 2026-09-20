package com.elon.acceptance;

import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** External semantic acceptance. Only synthetic fixture content may be written. */
public final class GroupAiUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private static final String GROUP = "ELON-GROUP-CONTEXT-ACCEPTANCE";
    private static final String FIRST = "ELON GROUP FIXTURE A: The release color is blue.";
    private static final String SECOND = "ELON GROUP FIXTURE B: The release day is Monday.";
    private UiObject text(String value) { return new UiObject(new UiSelector().packageName(APP).text(value)); }
    private UiObject desc(String value) { return new UiObject(new UiSelector().packageName(APP).description(value)); }
    private UiObject id(String value) { return new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/" + value)); }
    private AccessibilityNodeInfo node(UiObject control) throws Exception {
        assertTrue("group_control_missing", control.waitForExists(5000));
        java.lang.reflect.Method finder = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        finder.setAccessible(true);
        AccessibilityNodeInfo result = (AccessibilityNodeInfo) finder.invoke(control, 1000L);
        assertNotNull("group_node_missing", result);
        return result;
    }
    private void action(UiObject control, boolean longClick) throws Exception {
        AccessibilityNodeInfo info = node(control);
        try {
            for (int depth = 0; depth < 5 && !(longClick ? info.isLongClickable() : info.isClickable()); depth++) {
                AccessibilityNodeInfo parent = info.getParent();
                if (parent == null) break;
                if (!APP.contentEquals(parent.getPackageName() == null ? "" : parent.getPackageName())) {
                    parent.recycle(); break;
                }
                info.recycle(); info = parent;
            }
            assertTrue("group_control_disabled", info.isEnabled());
            assertTrue("group_action_failed", info.performAction(longClick ?
                AccessibilityNodeInfo.ACTION_LONG_CLICK : AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    private void fill(UiObject control, String value) throws Exception {
        AccessibilityNodeInfo info = node(control);
        try {
            Bundle args = new Bundle();
            args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, value);
            assertTrue("group_text_failed", info.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args));
        } finally { info.recycle(); }
    }
    private void fixtureOpen() throws Exception {
        assertTrue("synthetic_group_required", text(GROUP).exists());
        assertTrue("group_composer_missing", id("inputEdit").exists());
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        switch (step) {
            case "open_create":
                if (!text("发起群聊").exists()) action(desc("打开新建菜单").exists() ? desc("打开新建菜单") : id("addButton"), false);
                action(text("发起群聊"), false); break;
            case "create_fixture":
                assertTrue("create_group_dialog_missing", text("发起群聊").exists());
                UiObject ai = new UiObject(new UiSelector().packageName(APP).className("android.widget.CheckBox")
                    .textMatches("^一龙\\s*AI(?:\\s.*)?$"));
                assertTrue("isolated_ai_friend_missing", ai.exists());
                assertFalse("ambiguous_ai_friend", new UiObject(new UiSelector().packageName(APP)
                    .className("android.widget.CheckBox").textMatches("^一龙\\s*AI(?:\\s.*)?$").instance(1)).exists());
                fill(new UiObject(new UiSelector().packageName(APP).className("android.widget.EditText")), GROUP);
                action(ai, false); action(text("完成"), false); break;
            case "open_fixture": action(text(GROUP), false); break;
            case "send_first":
            case "send_second":
                fixtureOpen();
                assertTrue("existing_draft", id("inputEdit").getText().isEmpty() || id("inputEdit").getText().equals("输入内容"));
                fill(id("inputEdit"), step.equals("send_first") ? FIRST : SECOND);
                action(id("sendButton"), false); break;
            case "select_fixture":
                fixtureOpen(); action(text(FIRST), true); action(text("多选"), false);
                action(text(SECOND), false); action(text("AI 分析"), false); break;
            case "submit_selection":
                fill(desc("group-ai-selection-question"), "Reply exactly: ELON GROUP SELECTION PASSED");
                action(desc("group-ai-selection-submit"), false); break;
            case "share_answer":
                fixtureOpen(); action(text("ELON GROUP SELECTION PASSED"), true); action(text("转发"), false); break;
            case "cancel": action(text("取消"), false); break;
            case "back": getUiDevice().pressBack(); break;
            case "inspect": break;
            default: fail("group_step_unsupported");
        }
        getUiDevice().waitForIdle(1500);
        System.out.println("GROUP_AI_UI_RESULT=" + new JSONObject().put("step", step)
            .put("fixture_visible", text(GROUP).exists()).put("first_visible", text(FIRST).exists())
            .put("second_visible", text(SECOND).exists())
            .put("create_group", text("发起群聊").exists()).put("home_add", id("addButton").exists())
            .put("reply_visible", text("ELON GROUP SELECTION PASSED").exists())
            .put("selection_question", desc("group-ai-selection-question").exists())
            .put("private_continue", desc("ai-conversation-share-continue-private").exists())
            .put("share_send", desc("ai-conversation-share-send").exists()).toString());
    }
}
