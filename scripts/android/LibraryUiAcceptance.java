package com.elon.acceptance;

import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** External semantic acceptance; never installed into the consumer APK. */
public final class LibraryUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";

    private UiObject description(String value) {
        return new UiObject(new UiSelector().packageName(APP).description(value));
    }

    private UiObject text(String value) {
        return new UiObject(new UiSelector().packageName(APP).text(value));
    }

    private UiObject libraryFeature() throws Exception {
        UiObject preset = description("web-chat-feature:library");
        UiObject observed = new UiObject(new UiSelector().packageName(APP).descriptionMatches(
            "chatgpt-feature:[a-zA-Z0-9_]+:(\u8d44\u6599\u5e93|\u6587\u4ef6\u5e93)"));
        // The drawer closes asynchronously before the feature sheet is attached.
        long deadline = android.os.SystemClock.elapsedRealtime() + 8000;
        do {
            if (preset.exists()) return preset;
            if (observed.exists()) return observed;
            Thread.sleep(100);
        } while (android.os.SystemClock.elapsedRealtime() < deadline);
        fail("library_feature_missing");
        return preset;
    }

    private UiObject galleryFeature() throws Exception {
        UiObject preset = description("web-chat-feature:images");
        UiObject observed = new UiObject(new UiSelector().packageName(APP).descriptionMatches(
            "chatgpt-feature:[a-zA-Z0-9_]+:(\u56fe\u50cf|\u56fe\u7247)"));
        long deadline = android.os.SystemClock.elapsedRealtime() + 8000;
        do {
            if (preset.exists()) return preset;
            if (observed.exists()) return observed;
            Thread.sleep(100);
        } while (android.os.SystemClock.elapsedRealtime() < deadline);
        fail("gallery_feature_missing");
        return preset;
    }

    private boolean galleryVisible() {
        return description("\u540c\u6b65\u56fe\u50cf").exists() &&
            description("\u8fd4\u56de\u804a\u5929").exists();
    }

    private boolean galleryLoading() {
        return new UiObject(new UiSelector().packageName(APP).textMatches(
            ".*(\u6b63\u5728\u540c\u6b65\u56fe\u50cf|\u6b63\u5728\u540e\u53f0\u540c\u6b65).*" )).exists();
    }

    private void galleryStep(String step) throws Exception {
        switch (step) {
            case "gallery":
                click(description("web-chat-feature-navigation:chatgpt_web"));
                click(galleryFeature());
                assertTrue("gallery_not_visible", description("\u540c\u6b65\u56fe\u50cf").waitForExists(8000));
                break;
            case "gallery_wait":
                assertTrue("gallery_not_visible", galleryVisible());
                long deadline = android.os.SystemClock.elapsedRealtime() + 25000;
                while (galleryLoading() && android.os.SystemClock.elapsedRealtime() < deadline) Thread.sleep(250);
                break;
            case "gallery_next":
                assertTrue("gallery_not_visible", galleryVisible());
                click(description("\u4e0b\u4e00\u9875"));
                break;
            case "gallery_previous":
                assertTrue("gallery_not_visible", galleryVisible());
                click(description("\u4e0a\u4e00\u9875"));
                break;
            case "gallery_preview":
                assertTrue("gallery_not_visible", galleryVisible());
                click(description("\u56fe\u50cf 1"));
                assertTrue("image_viewer_not_visible", text("\u00d7").waitForExists(5000));
                break;
            case "gallery_close_preview":
                click(text("\u00d7"));
                assertTrue("gallery_not_restored", description("\u540c\u6b65\u56fe\u50cf").waitForExists(5000));
                break;
            case "gallery_close":
                click(description("\u8fd4\u56de\u804a\u5929"));
                assertTrue("gallery_not_closed", description("\u540c\u6b65\u56fe\u50cf").waitUntilGone(5000));
                break;
            case "gallery_inspect":
                break;
            default:
                fail("unsupported_gallery_step");
        }
    }

    private JSONObject galleryResult(String step) throws Exception {
        JSONObject result = new JSONObject().put("step", step);
        result.put("gallery_visible", galleryVisible());
        result.put("loading", galleryLoading());
        result.put("ready", new UiObject(new UiSelector().packageName(APP).textMatches(
            "\u672c\u9875 [0-9]+ \u5f20\u56fe\u7247")).exists());
        result.put("empty", text("\u8fd8\u6ca1\u6709\u521b\u5efa\u7684\u56fe\u7247").exists());
        result.put("partial", new UiObject(new UiSelector().packageName(APP).textMatches(
            "\u5df2\u52a0\u8f7d [0-9]+ \u5f20\u56fe\u7247.*")).exists());
        result.put("failed", new UiObject(new UiSelector().packageName(APP).textMatches(
            ".*\u540c\u6b65\u5931\u8d25.*")).exists());
        UiObject page = new UiObject(new UiSelector().packageName(APP).textMatches("\u7b2c [0-9]+ \u9875"));
        result.put("page", page.exists() ? Integer.parseInt(page.getText().replaceAll("[^0-9]", "")) : 0);
        int count = 0;
        for (int i = 1; i <= 25; i++) if (description("\u56fe\u50cf " + i).exists()) count++;
        result.put("visible_images", count);
        UiObject next = description("\u4e0b\u4e00\u9875"), previous = description("\u4e0a\u4e00\u9875");
        result.put("next_enabled", next.exists() && next.isEnabled());
        result.put("previous_enabled", previous.exists() && previous.isEnabled());
        result.put("viewer_visible", text("\u00d7").exists() && description("\u56fe\u50cf 1").exists());
        return result;
    }

    private void click(UiObject node) throws Exception {
        assertTrue("semantic_control_missing", node.waitForExists(8000));
        assertTrue("semantic_control_disabled", node.isEnabled());
        android.view.accessibility.AccessibilityNodeInfo info = accessibilityNode(node);
        try {
            for (int depth = 0; !info.isClickable() && depth < 4; depth++) {
                android.view.accessibility.AccessibilityNodeInfo parent = info.getParent();
                if (parent == null) break;
                if (!APP.contentEquals(parent.getPackageName() == null ? "" : parent.getPackageName())) {
                    parent.recycle();
                    break;
                }
                info.recycle();
                info = parent;
            }
            assertTrue("semantic_click_failed", info.performAction(
                android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK));
        } finally { info.recycle(); }
    }

    private android.view.accessibility.AccessibilityNodeInfo accessibilityNode(UiObject node) throws Exception {
        // Use the resolved accessibility node, not UiObject's coordinate injection.
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        android.view.accessibility.AccessibilityNodeInfo info =
            (android.view.accessibility.AccessibilityNodeInfo) method.invoke(node, 8000L);
        assertNotNull("semantic_node_missing", info);
        return info;
    }

    private void setText(UiObject node, String text) throws Exception {
        android.view.accessibility.AccessibilityNodeInfo info = accessibilityNode(node);
        android.os.Bundle value = new android.os.Bundle();
        value.putCharSequence(android.view.accessibility.AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text);
        try { assertTrue("text_not_set", info.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_SET_TEXT, value)); }
        finally { info.recycle(); }
    }

    public void testStep() throws Exception {
        assertEquals("foreground_package_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "inspect");
        String handle = getParams().getString("handle", "");
        if (step.startsWith("gallery")) {
            galleryStep(step);
            android.os.Bundle report = new android.os.Bundle();
            report.putString("stream", "LIBRARY_UI_RESULT=" + galleryResult(step).toString() + "\n");
            getAutomationSupport().sendStatus(0, report);
            return;
        }
        switch (step) {
            case "browse":
                click(description("web-chat-feature-navigation:chatgpt_web"));
                click(libraryFeature());
                assertTrue("library_not_visible", description("web-chat-library-browser").waitForExists(8000));
                break;
            case "features":
                click(description("web-chat-feature-navigation:chatgpt_web"));
                break;
            case "library":
                click(libraryFeature());
                assertTrue("library_not_visible", description("web-chat-library-browser").waitForExists(8000));
                break;
            case "query":
                assertTrue("library_not_visible", description("web-chat-library-browser").exists());
                setText(description("web-chat-library-query"), "ELON");
                click(description("web-chat-library-search"));
                break;
            case "query_fixture":
                String fixture = getParams().getString("fixtureName", "");
                assertTrue("invalid_fixture_query", fixture.matches(
                    "elon-chatgpt-(?:attachment|media)-fixture-v1\\.(?:txt|png|pdf)"));
                setText(description("web-chat-library-query"), fixture);
                click(description("web-chat-library-search"));
                break;
            case "clear_query":
                setText(description("web-chat-library-query"), "");
                click(description("web-chat-library-search"));
                break;
            case "refresh":
                click(description("web-chat-library-refresh"));
                break;
            case "more":
                click(description("web-chat-library-more"));
                break;
            case "file":
                assertTrue("invalid_handle", handle.matches("[a-zA-Z0-9_-]{1,160}"));
                click(description("web-chat-library-entry:" + handle));
                break;
            case "attach":
                // This only stages a reference. It does not send, rename or delete a file.
                click(text("\u52a0\u5165\u5f53\u524d\u804a\u5929"));
                break;
            case "download":
                // This invokes the actual consumer button, not the MCP download handler.
                click(text("\u4e0b\u8f7d"));
                assertTrue("download_status_missing", description("web-chat-file-download-status").waitForExists(8000));
                break;
            case "wait_download":
                long deadline = android.os.SystemClock.elapsedRealtime() + 25000;
                while (android.os.SystemClock.elapsedRealtime() < deadline) {
                    assertTrue("download_status_missing", description("web-chat-file-download-status").exists());
                    String value = description("web-chat-file-download-status").getText();
                    if (value.equals("\u5df2\u4fdd\u5b58\u5230\u4e0b\u8f7d\u76ee\u5f55")) break;
                    assertTrue("download_failed_or_unconfirmed", value.equals("\u6b63\u5728\u51c6\u5907\u4e0b\u8f7d") ||
                        value.equals("\u6b63\u5728\u4e0b\u8f7d") || value.equals("\u6b63\u5728\u4fdd\u5b58"));
                    Thread.sleep(500);
                }
                assertEquals("download_not_saved", "\u5df2\u4fdd\u5b58\u5230\u4e0b\u8f7d\u76ee\u5f55",
                    description("web-chat-file-download-status").getText());
                break;
            case "close_download":
                click(description("web-chat-file-download-collapse"));
                assertTrue("download_dialog_not_closed", description("web-chat-file-download-status").waitUntilGone(5000));
                break;
            case "rename":
                click(text("\u91cd\u547d\u540d"));
                assertTrue("rename_input_missing", description("web-chat-library-rename-input").waitForExists(8000));
                break;
            case "set_fixture_name":
                String name = getParams().getString("fixtureName", "");
                assertTrue("invalid_fixture_name", name.matches("(?i)elon[-_][a-z0-9_.-]{1,150}"));
                setText(description("web-chat-library-rename-input"), name);
                assertEquals("fixture_name_not_applied", name, description("web-chat-library-rename-input").getText());
                break;
            case "confirm_rename":
                UiObject input = description("web-chat-library-rename-input");
                assertTrue("invalid_fixture_name", input.getText().matches("(?i)elon[-_][a-z0-9_.-]{1,150}"));
                click(description("web-chat-library-mutation-confirm"));
                break;
            case "upload_fixture_copy":
                android.view.accessibility.AccessibilityNodeInfo preview = accessibilityNode(
                    description("elon-chatgpt-attachment-fixture-v1.txt"));
                try { assertTrue("fixture_long_click_failed", preview.performAction(
                    android.view.accessibility.AccessibilityNodeInfo.ACTION_LONG_CLICK)); }
                finally { preview.recycle(); }
                // Android PopupMenu exposes its label separately from the clickable row.
            case "select_upload_copy":
                click(text("\u91cd\u65b0\u4e0a\u4f20\u4e00\u4efd"));
                assertTrue("upload_copy_not_selected", description(
                    "\u91cd\u65b0\u4e0a\u4f20\u4e00\u4efd\uff1aelon-chatgpt-attachment-fixture-v1.txt").waitForExists(8000));
                break;
            case "trash":
                click(text("\u79fb\u5230\u6700\u8fd1\u5220\u9664"));
                assertTrue("trash_confirmation_missing", description("web-chat-library-mutation-confirm").waitForExists(8000));
                break;
            case "confirm_fixture_trash":
                String disposable = getParams().getString("fixtureName", "");
                assertTrue("invalid_disposable_fixture", disposable.matches("ELON-library-disposable-[a-f0-9]{12}\\.txt"));
                assertTrue("disposable_confirmation_mismatch", text(
                    "\u5c06\u201c" + disposable + "\u201d\u79fb\u5230\u6700\u8fd1\u5220\u9664\uff1f").exists());
                click(description("web-chat-library-mutation-confirm"));
                break;
            case "close_mutation":
                assertTrue("mutation_status_missing", description("web-chat-library-mutation-status").exists());
                click(text("\u5173\u95ed"));
                break;
            case "remove_staged":
                assertTrue("invalid_attachment_handle", handle.matches("(?:private_)?attachment_[a-zA-Z0-9_-]{1,160}"));
                click(description("web-chat-composer-attachment-remove:" + handle));
                break;
            case "close_detail":
                click(text("\u5173\u95ed"));
                break;
            case "back":
                click(description("web-chat-library-back"));
                break;
            case "inspect":
            case "inspect_entry":
                break;
            default:
                fail("unsupported_step");
        }
        JSONObject result = new JSONObject();
        result.put("step", step);
        result.put("library_visible", description("web-chat-library-browser").exists());
        result.put("search_visible", description("web-chat-library-query").exists());
        result.put("refresh_visible", description("web-chat-library-refresh").exists());
        result.put("back_visible", description("web-chat-library-back").exists());
        result.put("attach_visible", text("\u52a0\u5165\u5f53\u524d\u804a\u5929").exists());
        result.put("download_visible", text("\u4e0b\u8f7d").exists());
        UiObject downloadStatus = description("web-chat-file-download-status");
        result.put("download_status_visible", downloadStatus.exists());
        result.put("download_saved", downloadStatus.exists() && downloadStatus.getText().equals(
            "\u5df2\u4fdd\u5b58\u5230\u4e0b\u8f7d\u76ee\u5f55"));
        result.put("download_cancel_visible", description("web-chat-file-download-cancel").exists());
        result.put("download_progress_visible", description("web-chat-file-download-progress").exists());
        result.put("download_bytes_visible", description("web-chat-file-download-bytes").exists());
        result.put("rename_visible", text("\u91cd\u547d\u540d").exists());
        result.put("trash_visible", text("\u79fb\u5230\u6700\u8fd1\u5220\u9664").exists());
        result.put("entry_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-library-entry:")).exists());
        result.put("composer_attachment_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-composer-attachment:")).exists());
        result.put("composer_remove_visible", new UiObject(new UiSelector().packageName(APP)
            .descriptionStartsWith("web-chat-composer-attachment-remove:")).exists());
        result.put("rename_input_visible", description("web-chat-library-rename-input").exists());
        result.put("mutation_confirm_visible", description("web-chat-library-mutation-confirm").exists());
        result.put("mutation_status_visible", description("web-chat-library-mutation-status").exists());
        result.put("upload_copy_menu_visible", text("\u91cd\u65b0\u4e0a\u4f20\u4e00\u4efd").exists());
        result.put("local_fixture_visible", description("elon-chatgpt-attachment-fixture-v1.txt").exists());
        if (step.equals("inspect_entry")) {
            android.view.accessibility.AccessibilityNodeInfo node = accessibilityNode(
                description("web-chat-library-entry:" + handle));
            try {
                android.graphics.Rect bounds = new android.graphics.Rect();
                node.getBoundsInScreen(bounds);
                result.put("entry_clickable", node.isClickable());
                result.put("entry_enabled", node.isEnabled());
                result.put("entry_visible", node.isVisibleToUser());
                result.put("entry_actions", node.getActions());
                result.put("entry_bounds", bounds.toShortString());
            } finally { node.recycle(); }
        }
        android.os.Bundle report = new android.os.Bundle();
        report.putString("stream", "LIBRARY_UI_RESULT=" + result.toString() + "\n");
        getAutomationSupport().sendStatus(0, report);
    }
}
