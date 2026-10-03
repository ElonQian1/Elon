package com.elon.app.grid.sources

import android.app.Activity
import android.widget.ListView
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], qualifiers = "w375dp-h667dp-mdpi", application = android.app.Application::class)
class GridDeviceListViewTest {
    @Test fun checkboxesCanShowBothEitherOrNeitherWithoutMergingAccounts() {
        val controller = Robolectric.buildActivity(Activity::class.java).setup()
        val view = GridDeviceListView(controller.get()) {}
        val row = mapOf<String, Any?>("id" to "7", "symbol" to "BTCUSDT", "profit" to "1")
        val sources = listOf("android", "windows").map {
            GridDeviceSource(it, it, "00000000", 1, it, "primary", 1000, 2000, "fresh", listOf(row))
        }
        fun text(): String {
            val list = view.root.getChildAt(0) as ListView
            return (1 until list.adapter.count).joinToString("\n") { list.adapter.getItem(it).toString() }
        }
        view.show(sources, 1000); assertTrue(text().contains("APK 端")); assertTrue(text().contains("Win 端"))
        view.apk.isChecked = false; view.show(sources, 1000); assertFalse(text().contains("APK 端")); assertTrue(view.win.isChecked)
        view.win.isChecked = false; view.show(sources, 1000); assertTrue(text().contains("请选择"))
        view.apk.isChecked = true; view.show(sources, 2000); assertFalse(text().contains("BTCUSDT"))
        controller.pause().stop().destroy()
    }
}
