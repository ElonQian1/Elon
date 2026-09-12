package com.elon.acceptance;

import android.graphics.Rect;
import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import org.json.JSONObject;

/** Native Canvas acceptance. Mutations require a caller-owned synthetic body and exact hash. */
public final class CanvasUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private static final String PREFIX = "ELON_CANVAS_ACCEPTANCE_V1";
    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }
    private UiObject text(String value) {
        return new UiObject(new UiSelector().packageName(APP).text(value));
    }
    private UiObject input() {
        UiObject semantic = description("web-chat-composer-input:chatgpt_web");
        return semantic.exists() ? semantic : new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/inputEdit"));
    }
    private void focusComposer() throws Exception {
        if (!input().exists()) {
            click(composerPreview());
        }
        assertTrue("fixture_input_missing", input().waitForExists(5000));
    }
    private AccessibilityNodeInfo node(UiObject value) throws Exception {
        assertTrue("semantic_control_missing", value.waitForExists(5000));
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
            assertTrue("semantic_control_disabled", info.isEnabled());
            for (int depth = 0; !info.isClickable() && depth < 4; depth++) {
                AccessibilityNodeInfo parent = info.getParent();
                assertNotNull("clickable_parent_missing", parent);
                if (!APP.contentEquals(parent.getPackageName())) { parent.recycle(); fail("foreign_parent"); }
                info.recycle(); info = parent;
            }
            assertTrue("semantic_click_failed", info.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }
    private String hash(String value) throws Exception {
        StringBuilder out = new StringBuilder();
        for (byte b : MessageDigest.getInstance("SHA-256").digest(value.getBytes(StandardCharsets.UTF_8))) {
            out.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
        }
        return out.toString();
    }
    private String body(String selector) throws Exception {
        AccessibilityNodeInfo info = node(description(selector));
        try { return info.getText() == null ? "" : info.getText().toString(); }
        finally { info.recycle(); }
    }
    private void revealEntry() throws Exception {
        UiObject title = text("\u4f1a\u8bdd\u64cd\u4f5c");
        AccessibilityNodeInfo titleNode = node(title);
        int window;
        try { window = titleNode.getWindowId(); } finally { titleNode.recycle(); }
        for (int attempt = 0; attempt < 4; attempt++) {
            UiObject target = description("web-chat-conversation-action-canvas");
            if (target.exists()) { click(target); return; }
            AccessibilityNodeInfo scroll = node(new UiObject(new UiSelector().packageName(APP)
                .className("android.widget.ScrollView").scrollable(true)));
            try {
                assertEquals("wrong_scroll_window", window, scroll.getWindowId());
                assertTrue("canvas_entry_scroll_failed", scroll.performAction(AccessibilityNodeInfo.ACTION_SCROLL_FORWARD));
            } finally { scroll.recycle(); }
            Thread.sleep(250);
        }
        fail("canvas_entry_missing");
    }
    private JSONObject inspectBody(String selector, String kind) throws Exception {
        AccessibilityNodeInfo info = node(description(selector));
        try {
            String value = info.getText() == null ? "" : info.getText().toString();
            Rect bounds = new Rect(); info.getBoundsInScreen(bounds);
            assertTrue("canvas_body_not_visible", info.isVisibleToUser() && bounds.width() > 0 && bounds.height() > 0);
            assertTrue("canvas_body_not_native", String.valueOf(info.getClassName()).endsWith(kind));
            return new JSONObject().put("native_body", true).put("length", value.length())
                .put("sha256", hash(value)).put("fixture", value.contains(PREFIX));
        } finally { info.recycle(); }
    }
    private void fixtureGuard(String value) throws Exception {
        String expected = getParams().getString("expected_hash", "");
        assertEquals("fixture_write_not_authorized", "true", getParams().getString("allow_fixture_write", "false"));
        assertTrue("not_owned_canvas_fixture", value.contains(PREFIX) && value.length() < 4096);
        assertTrue("fixture_hash_required", expected.matches("[a-f0-9]{64}"));
        assertEquals("fixture_body_changed", expected, hash(value));
    }
    private void selectHistoryFirst() throws Exception {
        UiObject list = description("web-chat-canvas-history-list");
        UiObject first = list.getChild(new UiSelector().index(0));
        AccessibilityNodeInfo row = node(first);
        try { row.performAction(AccessibilityNodeInfo.ACTION_CLICK); } finally { row.recycle(); }
        if (!description("web-chat-canvas-history-content").waitForExists(1000)) {
            AccessibilityNodeInfo focus = node(list);
            try { focus.performAction(AccessibilityNodeInfo.ACTION_FOCUS); } finally { focus.recycle(); }
            assertTrue("history_focus_failed", list.isFocused());
            getUiDevice().pressKeyCode(android.view.KeyEvent.KEYCODE_MOVE_HOME);
            assertTrue("history_first_not_selected", first.isSelected());
            getUiDevice().pressDPadCenter();
        }
        assertTrue("history_preview_missing", description("web-chat-canvas-history-content").waitForExists(5000));
    }
    private UiObject composerPreview() throws Exception {
        UiObject panel = new UiObject(new UiSelector().packageName(APP).resourceId(APP + ":id/inputLayout"));
        UiObject fixture = panel.getChild(new UiSelector().className("android.widget.TextView").clickable(true)
            .textStartsWith("ELON_EXTENDED_TOOL_ACCEPTANCE_V1"));
        return fixture.exists() ? fixture : panel.getChild(new UiSelector().className("android.widget.TextView")
            .clickable(true).text("\u8f93\u5165\u5185\u5bb9"));
    }
    private JSONObject inspectComposer() throws Exception {
        UiObject editor = input();
        JSONObject result = new JSONObject().put("editor", editor.exists())
            .put("collapsed_preview", composerPreview().exists())
            .put("send", description("web-chat-send").exists())
            .put("voice", description("web-chat-composer-command:chatgpt_web:start-realtime-voice").exists());
        if (editor.exists()) {
            AccessibilityNodeInfo info = node(editor);
            try {
                Rect bounds = new Rect(); info.getBoundsInScreen(bounds);
                result.put("focused", info.isFocused()).put("enabled", info.isEnabled())
                    .put("height", bounds.height()).put("visible", info.isVisibleToUser())
                    .put("text_length", info.getText() == null ? 0 : info.getText().length());
            } finally { info.recycle(); }
        }
        return result;
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        JSONObject result = new JSONObject().put("step", step).put("content_exported", false);
        switch (step) {
            case "inspect_composer": result.put("composer", inspectComposer()); break;
            case "probe_composer_focus":
                result.put("before", inspectComposer());
                click(composerPreview());
                result.put("after_click", inspectComposer());
                Thread.sleep(300);
                result.put("after_300ms", inspectComposer());
                Thread.sleep(1700);
                result.put("after_2s", inspectComposer());
                break;
            case "set_composer_fixture":
                focusComposer();
                AccessibilityNodeInfo composer = node(input());
                try {
                    assertTrue("composer_not_editable", composer.isEditable() && composer.isEnabled());
                    assertTrue("composer_not_empty", composer.getText() == null || composer.getText().length() == 0 ||
                        composer.isShowingHintText());
                    Bundle args = new Bundle();
                    args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,
                        "ELON_EXTENDED_TOOL_ACCEPTANCE_V1: Reply exactly COMPOSER_READY");
                    assertTrue("composer_edit_failed", composer.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args));
                } finally { composer.recycle(); }
                result.put("draft_set", true); break;
            case "collapse_composer":
                assertTrue("composer_fixture_required", input().exists() && String.valueOf(input().getText())
                    .startsWith("ELON_EXTENDED_TOOL_ACCEPTANCE_V1"));
                getUiDevice().pressBack();
                assertTrue("composer_not_collapsed", input().waitUntilGone(5000));
                result.put("composer", inspectComposer()); break;
            case "focus_composer":
                focusComposer(); break;
            case "clear_fixture_draft":
                focusComposer();
                AccessibilityNodeInfo draftInput = node(input());
                try {
                    String value = String.valueOf(draftInput.getText());
                    assertTrue("not_owned_draft", value.startsWith("ELON_EXTENDED_TOOL_ACCEPTANCE_V1"));
                    Bundle clear = new Bundle(); clear.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, "");
                    assertTrue("fixture_clear_failed", draftInput.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, clear));
                } finally { draftInput.recycle(); }
                result.put("cleared", true); break;
            case "send_fixture":
                UiObject input = input();
                UiObject visibleDraft = input.exists() ? input : composerPreview();
                assertTrue("fixture_input_missing", visibleDraft.waitForExists(5000));
                long deadline = android.os.SystemClock.elapsedRealtime() + 3000;
                while (!String.valueOf(visibleDraft.getText()).startsWith("ELON_EXTENDED_TOOL_ACCEPTANCE_V1") &&
                    android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(100);
                assertTrue("fixture_prompt_missing", String.valueOf(visibleDraft.getText()).startsWith("ELON_EXTENDED_TOOL_ACCEPTANCE_V1"));
                click(description("web-chat-send")); result.put("sent_once", true); break;
            case "open_documents": revealEntry(); result.put("native_entry", true); break;
            case "open_first":
                click(description("web-chat-canvas-document-0"));
                assertTrue("native_editor_missing", description("web-chat-canvas-editor-body").waitForExists(5000));
                break;
            case "inspect_editor":
                result.put("body", inspectBody("web-chat-canvas-editor-body", "EditText"))
                    .put("save_enabled", description("web-chat-canvas-editor-save").isEnabled())
                    .put("history_enabled", description("web-chat-canvas-editor-history").isEnabled())
                    .put("share_enabled", description("web-chat-canvas-editor-share").isEnabled());
                break;
            case "check": click(description("web-chat-canvas-editor-check")); break;
            case "history": click(description("web-chat-canvas-editor-history")); break;
            case "preview_first": selectHistoryFirst(); break;
            case "inspect_history":
                result.put("body", inspectBody("web-chat-canvas-history-content", "TextView")); break;
            case "history_back": click(text("\u8fd4\u56de\u5386\u53f2")); break;
            case "return_editor":
                click(text("\u8fd4\u56de\u7f16\u8f91"));
                assertTrue("editor_not_restored", description("web-chat-canvas-editor-body").waitForExists(5000));
                break;
            case "share": click(description("web-chat-canvas-editor-share")); break;
            case "cancel_create":
                click(description("web-chat-canvas-original-share-primary"));
                assertTrue("create_confirmation_missing", description("web-chat-canvas-original-share-confirm").waitForExists(5000));
                click(text("\u53d6\u6d88"));
                assertTrue("share_dialog_not_restored", description("web-chat-canvas-original-share-primary").waitForExists(5000));
                result.put("canceled", true); break;
            case "append_fixture_save":
                String original = body("web-chat-canvas-editor-body"); fixtureGuard(original);
                String suffix = "\nELON_CANVAS_NATIVE_EDIT_V1\n";
                assertFalse("fixture_already_edited", original.contains("ELON_CANVAS_NATIVE_EDIT_V1"));
                AccessibilityNodeInfo editor = node(description("web-chat-canvas-editor-body"));
                try {
                    Bundle args = new Bundle();
                    args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, original + suffix);
                    assertTrue("fixture_edit_failed", editor.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args));
                } finally { editor.recycle(); }
                assertEquals("fixture_edit_mismatch", hash(original + suffix), hash(body("web-chat-canvas-editor-body")));
                click(description("web-chat-canvas-editor-save"));
                result.put("expected_hash", hash(original + suffix)).put("save_clicked_once", true); break;
            case "cancel_restore":
                click(description("web-chat-canvas-history-restore"));
                assertTrue("restore_confirmation_missing", description("web-chat-canvas-history-restore-confirm").waitForExists(5000));
                click(text("\u53d6\u6d88"));
                assertTrue("history_preview_not_restored", description("web-chat-canvas-history-content").waitForExists(5000));
                result.put("canceled", true); break;
            case "restore_fixture":
                fixtureGuard(body("web-chat-canvas-history-content"));
                click(description("web-chat-canvas-history-restore"));
                click(description("web-chat-canvas-history-restore-confirm"));
                result.put("restore_clicked_once", true); break;
            case "view_shared": click(description("web-chat-canvas-share-view")); break;
            case "inspect_shared":
                result.put("body", inspectBody("web-chat-canvas-content-body", "TextView")); break;
            case "return_link": click(text("\u8fd4\u56de\u94fe\u63a5")); break;
            case "close_editor":
                assertTrue("native_editor_missing", description("web-chat-canvas-editor-body").exists());
                click(text("\u8fd4\u56de"));
                assertTrue("native_editor_not_closed", description("web-chat-canvas-editor-body").waitUntilGone(5000));
                break;
            case "inspect":
                result.put("first_document", description("web-chat-canvas-document-0").exists())
                    .put("empty", description("web-chat-canvas-documents-empty").exists())
                    .put("list_status", description("web-chat-canvas-documents-status").exists())
                    .put("editor", description("web-chat-canvas-editor-body").exists())
                    .put("history_list", description("web-chat-canvas-history-list").exists())
                    .put("history_empty", text("\u6ca1\u6709\u66f4\u65e9\u7684\u7248\u672c").exists())
                    .put("history_preview", description("web-chat-canvas-history-content").exists())
                    .put("shared_content", description("web-chat-canvas-content-body").exists());
                break;
            default: fail("unsupported_step");
        }
        Bundle report = new Bundle();
        report.putString("stream", "CANVAS_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
