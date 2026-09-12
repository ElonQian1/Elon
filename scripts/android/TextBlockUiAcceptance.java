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

/** Production editor only. Local mutations and exports require the exact synthetic body hash. */
public final class TextBlockUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private static final String MARKER = "ELON_TEXT_BLOCK_ACCEPTANCE_V1";
    private UiObject desc(String name) { return new UiObject(new UiSelector().packageName(APP).description(name)); }
    private UiObject text(String name) { return new UiObject(new UiSelector().packageName(APP).text(name)); }
    private UiObject body() { return desc("web-chat-text-block-body"); }
    private AccessibilityNodeInfo node(UiObject target) throws Exception {
        assertTrue("native_control_missing", target.waitForExists(5000));
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo value = (AccessibilityNodeInfo) method.invoke(target, 5000L);
        assertNotNull("native_node_missing", value);
        assertEquals("native_owner_mismatch", APP, String.valueOf(value.getPackageName()));
        return value;
    }
    private void click(UiObject target) throws Exception {
        AccessibilityNodeInfo value = node(target);
        try {
            assertTrue("native_control_disabled", value.isEnabled());
            assertTrue("native_control_hidden", value.isVisibleToUser());
            for (int depth = 0; !value.isClickable() && depth < 4; depth++) {
                AccessibilityNodeInfo parent = value.getParent();
                assertNotNull("native_click_parent_missing", parent);
                if (!APP.contentEquals(parent.getPackageName())) { parent.recycle(); fail("foreign_click_parent"); }
                value.recycle(); value = parent;
            }
            assertTrue("native_click_failed", value.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { value.recycle(); }
    }
    private void setText(UiObject target, String content) throws Exception {
        AccessibilityNodeInfo value = node(target);
        try {
            assertTrue("native_input_disabled", value.isEnabled() && value.isEditable());
            Bundle args = new Bundle();
            args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, content);
            assertTrue("native_edit_failed", value.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args));
        } finally { value.recycle(); }
    }
    private String hash(String content) throws Exception {
        StringBuilder result = new StringBuilder();
        for (byte b : MessageDigest.getInstance("SHA-256").digest(content.getBytes(StandardCharsets.UTF_8))) {
            result.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
        }
        return result.toString();
    }
    private String fixture() throws Exception {
        String value = body().getText();
        assertTrue("unowned_text_block", value != null && value.contains(MARKER) && value.length() < 8192);
        return value;
    }
    private String guarded() throws Exception {
        String value = fixture();
        String expected = getParams().getString("expected_hash", "");
        assertTrue("fixture_hash_required", expected.matches("[a-f0-9]{64}"));
        assertTrue("fixture_changed", expected.equals(hash(value)));
        return value;
    }
    private JSONObject inspect() throws Exception {
        AccessibilityNodeInfo value = node(body());
        try {
            Rect bounds = new Rect(); value.getBoundsInScreen(bounds);
            assertTrue("native_body_hidden", value.isVisibleToUser() && bounds.width() > 0 && bounds.height() > 0);
            assertEquals("native_editor_required", "android.widget.EditText", String.valueOf(value.getClassName()));
        } finally { value.recycle(); }
        String content = fixture();
        return new JSONObject().put("native_editor", true).put("length", content.length()).put("sha256", hash(content))
            .put("edit_enabled", desc("web-chat-text-block-edit").isEnabled())
            .put("export_enabled", desc("web-chat-text-block-export").isEnabled());
    }
    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        JSONObject result = new JSONObject().put("step", step).put("private_content_exported", false);
        switch (step) {
            case "open":
                String selector = getParams().getString("selector", "");
                assertTrue("invalid_block_selector", selector.matches("web-chat-message-part:chatgpt_web:[A-Za-z0-9_.:-]+:[0-9]+:(code|writing_block)"));
                click(desc(selector)); result.put("body", inspect()); break;
            case "inspect": result.put("body", inspect()); break;
            case "edit":
                String original = guarded();
                click(desc("web-chat-text-block-edit"));
                setText(body(), original + "\nELON_LOCAL_EDIT_V1\n");
                result.put("body", inspect()); break;
            case "reset":
                guarded(); click(desc("web-chat-text-block-reset"));
                click(text("\u6062\u590d")); result.put("body", inspect()); break;
            case "export":
                String exported = guarded();
                String stem = getParams().getString("stem", "");
                String extension = getParams().getString("extension", "txt");
                assertTrue("invalid_fixture_file", stem.matches("elon-block-acceptance-[a-f0-9]{16}"));
                assertTrue("invalid_export_extension", extension.matches("md|py|txt"));
                click(desc("web-chat-text-block-export"));
                assertTrue("export_format_dialog_missing", text("\u5bfc\u51fa\u526f\u672c").waitForExists(3000));
                if (!new UiObject(new UiSelector().packageName(APP).textContains("(." + extension + ")")).exists()) {
                    extension = "txt";
                }
                click(new UiObject(new UiSelector().packageName(APP).textContains("(." + extension + ")")));
                setText(desc("web-chat-text-block-file-name"), stem);
                click(text("\u5bfc\u51fa"));
                long deadline = android.os.SystemClock.elapsedRealtime() + 5000;
                while (!"\u526f\u672c\u5df2\u5bfc\u51fa".equals(desc("web-chat-text-block-status").getText()) &&
                    android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(100);
                assertTrue("native_export_unconfirmed", "\u526f\u672c\u5df2\u5bfc\u51fa".equals(desc("web-chat-text-block-status").getText()));
                result.put("exported", true).put("sha256", hash(exported)).put("extension", extension); break;
            case "cancel_export":
                assertTrue("export_format_dialog_missing", text("\u5bfc\u51fa\u526f\u672c").exists());
                click(text("\u53d6\u6d88")); result.put("body", inspect()); break;
            case "close":
                if (body().exists()) {
                    fixture(); click(text("\u8fd4\u56de"));
                    if (text("\u4fee\u6539\u5c1a\u672a\u786e\u8ba4").exists()) {
                        fail("unsaved_fixture_requires_explicit_cleanup");
                    }
                    assertTrue("native_editor_still_open", body().waitUntilGone(3000));
                }
                result.put("closed", true); break;
            default: fail("unknown_text_block_step");
        }
        System.out.println("TEXT_BLOCK_UI_RESULT=" + result.toString());
    }
}
