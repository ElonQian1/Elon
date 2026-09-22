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
    private static final String CARD = "验收：群聊精选 AI 分析";
    private String targetGroup() {
        String encoded = getParams().getString("group_b64", "");
        if (encoded.isEmpty()) return GROUP;
        String value = new String(android.util.Base64.decode(encoded, android.util.Base64.DEFAULT),
            java.nio.charset.StandardCharsets.UTF_8);
        assertTrue("invalid_group_label", !value.trim().isEmpty() && value.length() <= 120 &&
            !value.contains("\n") && !value.contains("\r"));
        return value;
    }
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
        assertTrue("authorized_group_required", new UiObject(new UiSelector().packageName(APP)
            .resourceId(APP + ":id/topTitleText").text(targetGroup())).waitForExists(3000));
    }
    private void openFixture() throws Exception {
        if (!text(targetGroup()).exists() && text("群聊").exists()) action(text("群聊"), false);
        for (int attempt = 0; attempt < 5 && !text(targetGroup()).exists(); attempt++) {
            UiObject list = new UiObject(new UiSelector().packageName(APP).scrollable(true)
                .classNameMatches("(android.widget.(ScrollView|ListView)|androidx.recyclerview.widget.RecyclerView|androidx.core.widget.NestedScrollView)"));
            assertTrue("group_list_missing", list.exists());
            AccessibilityNodeInfo info = node(list);
            boolean moved;
            try { moved = info.performAction(AccessibilityNodeInfo.ACTION_SCROLL_FORWARD); }
            finally { info.recycle(); }
            getUiDevice().waitForIdle(1000);
            if (!moved) break;
        }
        action(text(targetGroup()), false);
        fixtureOpen();
    }
    private UiObject openComposer() throws Exception {
        if (!id("inputEdit").exists()) action(text("输入内容"), false);
        assertTrue("group_composer_missing", id("inputEdit").waitForExists(3000));
        return id("inputEdit");
    }
    private UiObject findMessage(String value, boolean forward) throws Exception {
        for (int attempt = 0; attempt < 4 && !text(value).exists(); attempt++) {
            AccessibilityNodeInfo list = node(id("chatList"));
            boolean moved;
            try { moved = list.performAction(forward ? AccessibilityNodeInfo.ACTION_SCROLL_FORWARD :
                AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD); }
            finally { list.recycle(); }
            getUiDevice().waitForIdle(1000);
            if (!moved) break;
        }
        return text(value);
    }
    private org.json.JSONArray modelMenuLabels() throws Exception {
        org.json.JSONArray values = new org.json.JSONArray();
        UiObject panel = desc("web-chat-model-control");
        if (!panel.exists()) return values;
        java.util.ArrayList<AccessibilityNodeInfo> pending = new java.util.ArrayList<>();
        pending.add(node(panel));
        for (int i = 0; i < pending.size() && i < 100; i++) {
            AccessibilityNodeInfo info = pending.get(i);
            CharSequence label = info.getText();
            if (label != null && label.length() <= 120) values.put(label.toString());
            for (int n = 0; n < info.getChildCount() && pending.size() < 100; n++) {
                AccessibilityNodeInfo child = info.getChild(n);
                if (child != null) pending.add(child);
            }
        }
        for (AccessibilityNodeInfo info : pending) info.recycle();
        return values;
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
            case "open_fixture": openFixture(); break;
            case "open_model": fixtureOpen(); action(id("modelButton"), false); break;
            case "model_default": action(desc("web-chat-model-default"), false); break;
            case "model_latest": action(text("最新"), false); break;
            case "send_first":
            case "send_second":
                fixtureOpen();
                UiObject composer = openComposer();
                assertTrue("existing_draft", composer.getText().isEmpty() || composer.getText().equals("输入内容"));
                fill(composer, step.equals("send_first") ? FIRST : SECOND);
                action(id("sendButton"), false); break;
            case "select_fixture":
                fixtureOpen(); action(findMessage(FIRST, false), true); action(text("多选"), false);
                action(findMessage(SECOND, true), false); action(text("AI 分析"), false); break;
            case "submit_selection":
                if (desc("group-ai-selection-project-memory").exists() && desc("group-ai-selection-project-memory").isChecked()) action(desc("group-ai-selection-project-memory"), false);
                fill(desc("group-ai-selection-question"), "Reply exactly: ELON GROUP SELECTION PASSED");
                action(desc("group-ai-selection-submit"), false); break;
            case "submit_project_selection":
                assertTrue("project_memory_choice_required", desc("group-ai-selection-project-memory").exists());
                if (!desc("group-ai-selection-project-memory").isChecked()) action(desc("group-ai-selection-project-memory"), false);
                fill(desc("group-ai-selection-question"), "Reply exactly: ELON GROUP PROJECT PASSED");
                action(desc("group-ai-selection-submit"), false); break;
            case "retry_selection":
                assertTrue("analysis_failure_required", text("AI 回答未发到群聊").exists());
                action(desc("group-ai-analysis-retry"), false); break;
            case "confirm_project_restart":
                assertTrue("project_restart_confirmation_required", text("上次项目创建尚未确认").exists());
                action(text("重新创建"), false); break;
            case "share_answer":
                fixtureOpen(); action(text("ELON GROUP SELECTION PASSED"), true); action(text("转发"), false); break;
            case "share_target":
                assertTrue("share_preview_required", desc("ai-conversation-share-send").exists());
                fill(desc("卡片标题"), CARD);
                action(desc("ai-conversation-share-target"), false); break;
            case "share_choose_group":
                assertTrue("share_target_picker_required", text("发送到群聊").exists());
                action(text(targetGroup()), false); break;
            case "share_submit":
                assertEquals("share_target_mismatch", targetGroup(), desc("ai-conversation-share-target").getText());
                assertEquals("share_title_mismatch", CARD, desc("卡片标题").getText());
                action(desc("ai-conversation-share-send"), false); break;
            case "open_card": fixtureOpen(); action(text(CARD), false); break;
            case "open_sources":
                fixtureOpen(); action(desc("group-ai-source-records-2"), false);
                assertTrue("source_reader_missing", id("ai_conversation_share_list").waitForExists(5000)); break;
            case "open_sharing":
                fixtureOpen();
                assertTrue("synthetic_answer_required", text("ELON GROUP SELECTION PASSED").exists());
                assertFalse("ambiguous_sharing_control", new UiObject(new UiSelector().packageName(APP)
                    .description("group-ai-discussion-sharing").instance(1)).exists());
                action(desc("group-ai-discussion-sharing"), false);
                assertTrue("sharing_dialog_missing", text("讨论分享设置").waitForExists(5000)); break;
            case "allow_discussion":
            case "disable_discussion":
                assertTrue("sharing_dialog_required", text("讨论分享设置").exists());
                action(text(step.equals("allow_discussion") ? "允许继续讨论" : "关闭分享"), false); break;
            case "continue_private": action(desc("ai-conversation-share-continue-private"), false); break;
            case "confirm_private":
                assertTrue("private_confirmation_required", text("用自己的 ChatGPT 继续讨论").exists());
                action(text("进入私人会话"), false); break;
            case "return_group":
                assertTrue("discard_confirmation_required", text("放弃未发送的私人草稿？").exists());
                action(text("放弃并返回"), false); break;
            case "cancel": action(text("取消"), false); break;
            case "back": getUiDevice().pressBack(); break;
            case "inspect": break;
            default: fail("group_step_unsupported");
        }
        getUiDevice().waitForIdle(1500);
        System.out.println("GROUP_AI_UI_RESULT=" + new JSONObject().put("step", step)
            .put("model_label", id("modelButton").exists() ? id("modelButton").getText() : "")
            .put("model_menu_labels", modelMenuLabels())
            .put("fixture_visible", text(targetGroup()).exists()).put("first_visible", text(FIRST).exists())
            .put("second_visible", text(SECOND).exists())
            .put("create_group", text("发起群聊").exists()).put("home_add", id("addButton").exists())
            .put("reply_visible", text("ELON GROUP SELECTION PASSED").exists())
            .put("project_reply_visible", text("ELON GROUP PROJECT PASSED").exists())
            .put("project_memory_selected", desc("group-ai-selection-project-memory").exists() && desc("group-ai-selection-project-memory").isChecked())
            .put("card_visible", text(CARD).exists())
            .put("source_records_visible", desc("group-ai-source-records-2").exists())
            .put("continue_discussion_visible", desc("group-ai-continue-discussion").exists())
            .put("sharing_dialog", text("讨论分享设置").exists())
            .put("allow_discussion", text("允许继续讨论").exists())
            .put("disable_discussion", text("关闭分享").exists())
            .put("reader_visible", id("ai_conversation_share_list").exists())
            .put("private_confirmation", text("用自己的 ChatGPT 继续讨论").exists())
            .put("return_confirmation", text("放弃未发送的私人草稿？").exists())
            .put("selection_question", desc("group-ai-selection-question").exists())
            .put("analysis_failed", text("AI 回答未发到群聊").exists())
            .put("analysis_retry", desc("group-ai-analysis-retry").exists())
            .put("private_continue", desc("ai-conversation-share-continue-private").exists())
            .put("share_send", desc("ai-conversation-share-send").exists()).toString());
    }
}
