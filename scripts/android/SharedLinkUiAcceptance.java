package com.elon.acceptance;

import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;
import org.json.JSONArray;

/** External native share acceptance. Canvas scope is read/copy/cancel only. No content export. */
public final class SharedLinkUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }
    private UiObject text(String value) {
        return new UiObject(new UiSelector().packageName(APP).text(value));
    }
    private AccessibilityNodeInfo node(UiObject value) throws Exception {
        assertTrue("semantic_control_missing", value.waitForExists(5000));
        assertTrue("semantic_control_disabled", value.isEnabled());
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo info = (AccessibilityNodeInfo) method.invoke(value, 5000L);
        assertNotNull("semantic_node_missing", info);
        assertEquals("semantic_owner_mismatch", APP, String.valueOf(info.getPackageName()));
        return info;
    }
    private void click(UiObject value) throws Exception {
        AccessibilityNodeInfo info = node(value);
        try {
            for (int depth = 0; !info.isClickable() && depth < 4; depth++) {
                AccessibilityNodeInfo parent = info.getParent();
                assertNotNull("clickable_parent_missing", parent);
                if (!APP.contentEquals(parent.getPackageName())) { parent.recycle(); fail("foreign_parent"); }
                info.recycle(); info = parent;
            }
            assertTrue("semantic_click_failed", info.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    private String expectedUrl() {
        String value = new String(android.util.Base64.decode(getParams().getString("url_b64", ""),
            android.util.Base64.DEFAULT), StandardCharsets.UTF_8);
        String pattern = canvas() ? "^https://chatgpt\\.com/canvas/shared/[A-Za-z0-9_-]{1,128}$"
            : "^https://chatgpt\\.com/share/[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}$";
        assertTrue("invalid_share_url", value.matches(pattern));
        return value;
    }
    private boolean canvas() {
        String resource = getParams().getString("resource", "conversation");
        assertTrue("invalid_share_resource", resource.equals("conversation") || resource.equals("canvas"));
        return resource.equals("canvas");
    }
    private UiObject shareList() {
        return description(canvas() ? "web-chat-canvas-share-links-list" : "web-chat-account-share-links-list");
    }
    private void assertSelected(String expected) throws Exception {
        assertTrue("selected_share_mismatch", text(expected).waitForExists(5000));
        assertTrue("copy_action_missing", description("web-chat-existing-share-copy").exists());
        assertTrue("revoke_action_missing", description("web-chat-share-revoke").exists());
    }
    private void selectFirst(String expected) throws Exception {
        UiObject list = shareList();
        UiObject first = list.getChild(new UiSelector().index(0));
        AccessibilityNodeInfo row = node(first);
        try { row.performAction(AccessibilityNodeInfo.ACTION_CLICK); }
        finally { row.recycle(); }
        if (!text(expected).waitForExists(1200)) {
            // Some OEM ListView delegates advertise click but reject ACTION_CLICK.
            AccessibilityNodeInfo focus = node(list);
            try { if (!focus.isFocused()) focus.performAction(AccessibilityNodeInfo.ACTION_FOCUS); }
            finally { focus.recycle(); }
            long focusDeadline = android.os.SystemClock.elapsedRealtime() + 2000;
            while (!list.isFocused() && android.os.SystemClock.elapsedRealtime() < focusDeadline) Thread.sleep(100);
            assertTrue("share_list_focus_failed", list.isFocused());
            getUiDevice().pressKeyCode(android.view.KeyEvent.KEYCODE_MOVE_HOME);
            long deadline = android.os.SystemClock.elapsedRealtime() + 2000;
            while (!first.isSelected() && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(100);
            assertTrue("first_share_row_not_selected", first.isSelected());
            getUiDevice().pressDPadCenter();
        }
        assertSelected(expected);
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        String expected = step.equals("open_canvas_list") || step.equals("inspect_list") || step.equals("close_list") ||
            step.equals("inspect") && !getParams().containsKey("url_b64")
            ? "" : expectedUrl();
        JSONObject result = new JSONObject().put("step", step).put("content_exported", false);
        switch (step) {
            case "open_canvas_list":
                assertTrue("canvas_resource_required", canvas());
                assertTrue("share_menu_missing", description("web-chat-conversation-share-options").exists());
                click(text("\u7ba1\u7406\u753b\u5e03\u516c\u5f00\u94fe\u63a5"));
                result.put("native_entry_clicked", true);
                break;
            case "inspect_list":
                AccessibilityNodeInfo listInfo = node(shareList());
                try {
                    result.put("native_list_visible", listInfo.isVisibleToUser())
                        .put("row_count", listInfo.getChildCount());
                    assertTrue("native_list_wrong_type", String.valueOf(listInfo.getClassName()).endsWith("ListView"));
                    for (int i = 0; i < listInfo.getChildCount(); i++) {
                        AccessibilityNodeInfo child = listInfo.getChild(i);
                        try {
                            assertNotNull("native_share_row_missing", child);
                            assertTrue("native_share_row_not_visible", child.isVisibleToUser());
                            assertTrue("native_share_row_disabled", child.isEnabled());
                        } finally { if (child != null) child.recycle(); }
                    }
                } finally { listInfo.recycle(); }
                break;
            case "close_list":
                assertTrue("share_list_missing", shareList().exists());
                click(text("\u5173\u95ed"));
                assertTrue("share_list_not_closed", shareList().waitUntilGone(5000));
                result.put("list_closed", true);
                break;
            case "select_first":
                selectFirst(expected);
                result.put("selected_matches", true);
                break;
            case "copy":
                assertSelected(expected);
                click(description("web-chat-existing-share-copy"));
                assertTrue("copy_dialog_not_closed", description("web-chat-existing-share-copy").waitUntilGone(5000));
                result.put("copy_clicked", true);
                break;
            case "paste_check":
                UiObject input = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/inputEdit"));
                if (!input.exists()) click(text("\u8f93\u5165\u5185\u5bb9"));
                assertTrue("native_input_missing", input.waitForExists(5000));
                String before = input.getText();
                // A hint can be exposed as text. Never paste over an actual pre-existing draft.
                AccessibilityNodeInfo info = node(input);
                try {
                    assertTrue("native_draft_not_empty", before == null || before.isEmpty() || info.isShowingHintText());
                    if (!info.isFocused()) info.performAction(AccessibilityNodeInfo.ACTION_FOCUS);
                    assertTrue("input_paste_failed", info.performAction(AccessibilityNodeInfo.ACTION_PASTE));
                } finally { info.recycle(); }
                long deadline = android.os.SystemClock.elapsedRealtime() + 3000;
                while (!expected.equals(input.getText()) && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(100);
                try { assertTrue("copied_share_mismatch", expected.equals(input.getText())); }
                finally {
                    AccessibilityNodeInfo clear = node(input);
                    try {
                        Bundle args = new Bundle();
                        args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "");
                        assertTrue("clear_test_draft_failed", clear.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args));
                    } finally { clear.recycle(); }
                }
                result.put("clipboard_exact_match", true).put("test_draft_cleared", true);
                break;
            case "revoke":
                assertFalse("canvas_write_not_permitted", canvas());
                assertSelected(expected);
                click(description("web-chat-share-revoke"));
                assertTrue("revoke_confirmation_missing", description("web-chat-share-revoke-confirm").waitForExists(5000));
                UiObject message = new UiObject(new UiSelector().packageName(APP).resourceId("android:id/message"));
                assertTrue("revoke_confirmation_target_mismatch", message.exists() && message.getText().endsWith("\n\n" + expected));
                click(description("web-chat-share-revoke-confirm"));
                result.put("revoke_confirmed_once", true);
                break;
            case "cancel_revoke":
                assertSelected(expected);
                click(description("web-chat-share-revoke"));
                assertTrue("revoke_confirmation_missing", description("web-chat-share-revoke-confirm").waitForExists(5000));
                UiObject confirmation = new UiObject(new UiSelector().packageName(APP).resourceId("android:id/message"));
                assertTrue("revoke_confirmation_target_mismatch", confirmation.exists() && confirmation.getText().endsWith("\n\n" + expected));
                if (canvas()) assertTrue("canvas_retention_notice_missing", confirmation.getText().contains("\u539f\u753b\u5e03\u4e0d\u4f1a\u5220\u9664"));
                click(text("\u4fdd\u7559\u94fe\u63a5"));
                assertTrue("revoke_confirmation_not_closed", description("web-chat-share-revoke-confirm").waitUntilGone(5000));
                assertSelected(expected);
                result.put("confirmation_canceled", true);
                break;
            case "back_to_list":
                assertSelected(expected);
                click(text("\u8fd4\u56de\u5217\u8868"));
                assertTrue("share_list_not_restored", shareList().waitForExists(5000));
                result.put("list_restored", true);
                break;
            case "inspect":
                result.put("account_list", description("web-chat-account-share-links-list").exists())
                    .put("canvas_list", description("web-chat-canvas-share-links-list").exists())
                    .put("selected_dialog", description("web-chat-existing-share-copy").exists())
                    .put("revoke_dialog", description("web-chat-share-revoke-confirm").exists())
                    .put("selected_matches", !expected.isEmpty() && text(expected).exists());
                JSONArray editors = new JSONArray();
                for (int i = 0; i < 4; i++) {
                    UiObject editor = new UiObject(new UiSelector().packageName(APP).className("android.widget.EditText").instance(i));
                    if (!editor.exists()) break;
                    AccessibilityNodeInfo editorInfo = node(editor);
                    try { editors.put(new JSONObject().put("id", editorInfo.getViewIdResourceName())
                        .put("visible", editorInfo.isVisibleToUser()).put("focused", editorInfo.isFocused())); }
                    finally { editorInfo.recycle(); }
                }
                result.put("native_editors", editors);
                if (description("web-chat-account-share-links-list").exists()) {
                    AccessibilityNodeInfo rowInfo = node(description("web-chat-account-share-links-list")
                        .getChild(new UiSelector().index(0)));
                    JSONArray chain = new JSONArray();
                    for (int i = 0; rowInfo != null && i < 5; i++) {
                        chain.put(new JSONObject().put("type", String.valueOf(rowInfo.getClassName()))
                            .put("clickable", rowInfo.isClickable()).put("enabled", rowInfo.isEnabled())
                            .put("visible", rowInfo.isVisibleToUser()).put("actions", rowInfo.getActions()));
                        AccessibilityNodeInfo parent = rowInfo.getParent(); rowInfo.recycle(); rowInfo = parent;
                    }
                    if (rowInfo != null) rowInfo.recycle();
                    result.put("row_ancestry", chain);
                }
                break;
            default: fail("unsupported_step");
        }
        Bundle report = new Bundle();
        report.putString("stream", "SHARED_LINK_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
