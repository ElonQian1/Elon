package com.elon.acceptance;

import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** External consumer-file-picker acceptance, restricted to one synthetic fixture. */
public final class AttachmentPickerUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private static final String FILE = "elon-chatgpt-local-append-fixture-v1.txt";
    private static final String PROMPT = "ELON_LIBRARY_LOCAL_APPEND_ACCEPTANCE_V1";

    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }

    private String pickerPackage() {
        String owner = getUiDevice().getCurrentPackageName();
        return owner.equals("com.android.documentsui") || owner.equals("com.google.android.documentsui") ||
            owner.equals("com.android.fileexplorer") ? owner : "";
    }

    private AccessibilityNodeInfo node(UiObject target) throws Exception {
        assertTrue("semantic_control_missing", target.waitForExists(8000));
        assertTrue("semantic_control_disabled", target.isEnabled());
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo value = (AccessibilityNodeInfo) method.invoke(target, 8000L);
        assertNotNull("semantic_node_missing", value);
        return value;
    }

    private void click(UiObject target, String owner) throws Exception {
        AccessibilityNodeInfo value = node(target);
        try {
            assertEquals("semantic_owner_mismatch", owner, String.valueOf(value.getPackageName()));
            for (int depth = 0; !value.isClickable() && depth < 5; depth++) {
                AccessibilityNodeInfo parent = value.getParent();
                if (parent == null) break;
                if (!owner.equals(String.valueOf(parent.getPackageName()))) { parent.recycle(); break; }
                value.recycle(); value = parent;
            }
            assertTrue("semantic_click_failed", value.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { value.recycle(); }
    }

    private UiObject fixture(String owner) {
        return new UiObject(new UiSelector().packageName(owner).text(FILE));
    }

    private void search(String owner) throws Exception {
        if (fixture(owner).exists()) return;
        click(new UiObject(new UiSelector().packageName(owner).resourceId(owner + ":id/menu_search")), owner);
        UiObject edit = new UiObject(new UiSelector().packageName(owner).className("android.widget.EditText"));
        AccessibilityNodeInfo value = node(edit);
        android.os.Bundle args = new android.os.Bundle();
        args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, FILE);
        try { assertTrue("fixture_search_failed", value.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)); }
        finally { value.recycle(); }
        getUiDevice().pressKeyCode(android.view.KeyEvent.KEYCODE_ENTER);
        assertTrue("fixture_not_found", fixture(owner).waitForExists(10000));
    }

    private void scrollTo(UiObject target, String owner) throws Exception {
        Thread.sleep(500);
        for (int attempt = 0; attempt < 20; attempt++) {
            if (target.exists()) return;
            UiObject fileList = new UiObject(new UiSelector().packageName(owner).resourceId("android:id/list"));
            AccessibilityNodeInfo list = node(fileList.exists() ? fileList :
                new UiObject(new UiSelector().packageName(owner).scrollable(true)));
            boolean moved;
            try { moved = list.performAction(AccessibilityNodeInfo.ACTION_SCROLL_FORWARD); }
            finally { list.recycle(); }
            Thread.sleep(250);
            if (!moved) break;
        }
        assertTrue("fixture_location_not_found", target.exists());
    }

    private JSONObject pickerControls(String owner) throws Exception {
        JSONObject result = new JSONObject();
        if (owner.isEmpty()) return result;
        AccessibilityNodeInfo root = node(new UiObject(new UiSelector().packageName(owner)));
        while (true) {
            AccessibilityNodeInfo parent = root.getParent();
            if (parent == null) break;
            root.recycle(); root = parent;
        }
        java.util.ArrayDeque<AccessibilityNodeInfo> pending = new java.util.ArrayDeque<>();
        java.util.LinkedHashSet<String> ids = new java.util.LinkedHashSet<>();
        java.util.LinkedHashSet<String> labels = new java.util.LinkedHashSet<>();
        pending.add(root);
        int count = 0;
        while (!pending.isEmpty()) {
            AccessibilityNodeInfo value = pending.remove();
            try {
                if (count++ >= 256) continue;
                if (owner.equals(String.valueOf(value.getPackageName())) && value.isVisibleToUser()) {
                    String id = value.getViewIdResourceName();
                    if (id != null && ids.size() < 80) ids.add(id);
                    String label = String.valueOf(value.getText());
                    if (label.matches("(Downloads|Recent|Search|Internal storage|\\u4e0b\\u8f7d|\\u6700\\u8fd1|\\u6d4f\\u89c8|\\u641c\\u7d22|\\u624b\\u673a|\\u6587\\u6863|\\u5185\\u90e8\\u5b58\\u50a8|\\u5168\\u90e8\\u6587\\u4ef6)")) labels.add(label);
                }
                for (int i = 0; i < value.getChildCount(); i++) {
                    AccessibilityNodeInfo child = value.getChild(i);
                    if (child != null) pending.add(child);
                }
            } finally { value.recycle(); }
        }
        return result.put("resource_ids", new org.json.JSONArray(ids)).put("navigation_labels", new org.json.JSONArray(labels));
    }

    public void testStep() throws Exception {
        String step = getParams().getString("step", "inspect");
        String picker = pickerPackage();
        if (step.equals("open_document") || step.equals("send")) {
            assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        } else {
            assertTrue("foreground_package_mismatch", !picker.isEmpty() || APP.equals(getUiDevice().getCurrentPackageName()));
        }
        switch (step) {
            case "open_document":
                if (!description("attachment-action-files").exists()) click(description("web-chat-attachment:chatgpt_web"), APP);
                click(description("attachment-action-files"), APP);
                long deadline = android.os.SystemClock.elapsedRealtime() + 8000;
                while (pickerPackage().isEmpty() && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(100);
                assertFalse("document_picker_not_open", pickerPackage().isEmpty());
                break;
            case "search_fixture":
                assertFalse("document_picker_not_open", picker.isEmpty());
                search(picker);
                break;
            case "browse_picker":
                assertEquals("unsupported_picker_browser", "com.android.fileexplorer", picker);
                click(new UiObject(new UiSelector().packageName(picker).resourceId("android:id/text1").text("\u6d4f\u89c8")), picker);
                break;
            case "download_fixture":
                assertEquals("unsupported_picker_browser", "com.android.fileexplorer", picker);
                UiObject downloads = new UiObject(new UiSelector().packageName(picker).text("Download"));
                scrollTo(downloads, picker);
                click(downloads, picker);
                scrollTo(fixture(picker), picker);
                break;
            case "select_fixture":
                assertFalse("document_picker_not_open", picker.isEmpty());
                click(fixture(picker), picker);
                UiObject confirm = new UiObject(new UiSelector().packageName(picker).resourceId(picker + ":id/btnConfirm"));
                if (picker.equals("com.android.fileexplorer") && confirm.waitForExists(1000) && confirm.isEnabled()) click(confirm, picker);
                assertTrue("native_composer_not_restored", description("web-chat-composer-input:chatgpt_web").waitForExists(10000));
                assertTrue("local_fixture_not_staged", description(FILE).waitForExists(8000));
                break;
            case "send":
                UiObject input = description("web-chat-composer-input:chatgpt_web");
                assertTrue("fixture_prompt_missing", input.exists() && input.getText().startsWith(PROMPT));
                assertTrue("local_fixture_not_staged", description(FILE).exists());
                assertTrue("library_reference_not_staged", new UiObject(new UiSelector().packageName(APP)
                    .descriptionStartsWith("web-chat-composer-attachment:")).exists());
                click(description("web-chat-send"), APP);
                break;
            case "cancel_picker":
                assertFalse("document_picker_not_open", picker.isEmpty());
                getUiDevice().pressBack();
                break;
            case "inspect": break;
            default: fail("unsupported_step");
        }
        picker = pickerPackage();
        JSONObject result = new JSONObject().put("step", step)
            .put("native_foreground", APP.equals(getUiDevice().getCurrentPackageName()))
            .put("document_picker", !picker.isEmpty())
            .put("picker_controls", step.equals("inspect") ? pickerControls(picker) : new JSONObject())
            .put("fixture_row", !picker.isEmpty() && fixture(picker).exists())
            .put("local_fixture_staged", description(FILE).exists())
            .put("library_reference_staged", new UiObject(new UiSelector().packageName(APP)
                .descriptionStartsWith("web-chat-composer-attachment:")).exists());
        android.os.Bundle report = new android.os.Bundle();
        report.putString("stream", "ATTACHMENT_PICKER_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
