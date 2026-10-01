package com.elon.acceptance;

import android.os.Bundle;
import android.view.accessibility.AccessibilityNodeInfo;
import com.android.uiautomator.core.UiObject;
import com.android.uiautomator.core.UiSelector;
import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import org.json.JSONObject;

/** Exercises only the pinned non-user long-image fixture. Never sends a message. */
public final class ImageViewerUiAcceptance extends UiAutomatorTestCase {
    private static final String APP = "com.elon.app";
    private static final String FILE = "elon-image-fidelity-v1.png";
    private UiObject desc(String value) { return new UiObject(new UiSelector().packageName(APP).description(value)); }
    private UiObject text(String value) { return new UiObject(new UiSelector().packageName(APP).text(value)); }
    private void click(UiObject object) throws Exception {
        assertTrue("image_control_missing", object.waitForExists(6000));
        java.lang.reflect.Method method = UiObject.class.getDeclaredMethod("findAccessibilityNodeInfo", long.class);
        method.setAccessible(true);
        AccessibilityNodeInfo node = (AccessibilityNodeInfo) method.invoke(object, 1000L);
        assertNotNull("image_node_missing", node);
        try {
            assertEquals("image_owner_mismatch", APP, String.valueOf(node.getPackageName()));
            assertTrue("image_control_disabled", node.isEnabled());
            assertTrue("image_click_failed", node.performAction(AccessibilityNodeInfo.ACTION_CLICK));
        } finally { node.recycle(); }
    }
    public void testStep() throws Exception {
        assertEquals("image_foreground_mismatch", APP, getUiDevice().getCurrentPackageName());
        String step = getParams().getString("step", "open");
        if (step.equals("open")) click(desc(FILE));
        if (step.equals("close")) {
            click(desc("关闭图片"));
            assertTrue("image_not_closed", desc("关闭图片").waitUntilGone(5000));
        } else {
            assertTrue("full_image_not_ready", text("1080 × 6000").waitForExists(15000));
            if (step.equals("original_scale")) click(desc("原始比例"));
            else if (step.equals("pinch")) {
                UiObject image = desc("高清图片：" + FILE);
                assertTrue("image_not_found", image.exists());
                assertTrue("image_pinch_failed", image.pinchOut(60, 30));
            } else if (step.equals("cache_menu")) {
                click(desc("图片选项"));
                assertTrue("image_cache_menu_missing", text("清理图片缓存").waitForExists(3000));
                getUiDevice().pressBack();
            } else assertEquals("unknown_image_step", "open", step);
            assertFalse("image_load_failed", text("图片加载失败").exists());
        }
        Bundle result = new Bundle();
        result.putString("stream", "IMAGE_VIEWER_UI_RESULT=" + new JSONObject()
            .put("step", step).put("ready", text("1080 × 6000").exists())
            .put("content_exported", false).toString() + "\n");
        getAutomationSupport().sendStatus(0, result);
    }
}
