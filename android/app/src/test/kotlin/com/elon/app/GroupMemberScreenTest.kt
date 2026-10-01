package com.elon.app

import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.ListView
import android.widget.TextView
import okhttp3.OkHttpClient
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.time.Duration

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], application = android.app.Application::class)
class GroupMemberScreenTest {
    private fun views(root: View): List<View> = listOf(root) + if (root is ViewGroup) (0 until root.childCount).flatMap { views(root.getChildAt(it)) } else emptyList()
    private fun await(condition: () -> Boolean) {
        repeat(400) { shadowOf(Looper.getMainLooper()).idle(); if (condition()) return; Thread.sleep(10) }
        fail("Member screen state did not arrive")
    }
    @Test fun paginates172AndSearchesLastMemberWithoutCountingOnlyThePage() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val fixture = GroupMemberFixture(); val http = OkHttpClient.Builder().addInterceptor(fixture).build()
        try {
            val screen = GroupMemberScreen(host.get(), GroupMemberRepository(host.get(), http, "https://offline.invalid"))
            val dialog = screen.show("fixture-group"); val all = views(dialog.window!!.decorView); val list = all.filterIsInstance<ListView>().single()
            await { list.adapter.count == 51 }
            assertTrue(all.filterIsInstance<TextView>().any { it.text == "群成员（172）" })
            val more = views(list.adapter.getView(list.adapter.count - 1, null, list)).filterIsInstance<Button>().single { it.text.startsWith("加载更多") }
            for (size in listOf(101, 151, 173)) { more.performClick(); await { list.adapter.count == size } }
            val search = all.filterIsInstance<EditText>().single(); search.setText("成员0171"); shadowOf(Looper.getMainLooper()).idleFor(Duration.ofMillis(300))
            await { list.adapter.count == 2 }
            assertEquals("成员0171", (list.adapter.getItem(0) as RosterPerson).name)
            assertTrue(fixture.requests.any { it.contains("cursor=150") }); dialog.dismiss()
        } finally { http.dispatcher.executorService.shutdown(); host.pause().stop().destroy() }
    }
    @Test fun ordinaryMemberHasNoManagementAndDeniedRefreshErasesRoster() {
        val host = Robolectric.buildActivity(AiConversationShareReaderTestActivity::class.java).setup()
        val fixture = GroupMemberFixture().apply { role = "member" }; val http = OkHttpClient.Builder().addInterceptor(fixture).build()
        try {
            val dialog = GroupMemberScreen(host.get(), GroupMemberRepository(host.get(), http, "https://offline.invalid")).show("fixture-group")
            val list = views(dialog.window!!.decorView).filterIsInstance<ListView>().single(); await { list.adapter.count == 51 }
            assertFalse(views(dialog.window!!.decorView).filterIsInstance<Button>().any { it.text == "管理成员" })
            fixture.denied = true
            views(dialog.window!!.decorView).filterIsInstance<EditText>().single().setText("触发重新加载")
            shadowOf(Looper.getMainLooper()).idleFor(Duration.ofMillis(300)); await { views(dialog.window!!.decorView).filterIsInstance<TextView>().any { it.text == "你已不在这个群聊中" } }
            assertEquals(1, list.adapter.count)
            val retry = views(dialog.window!!.decorView).filterIsInstance<Button>().single { it.text == "重新加载" }
            fixture.denied = false; retry.performClick(); await { fixture.requests.size >= 3 }; dialog.dismiss()
        } finally { http.dispatcher.executorService.shutdown(); host.pause().stop().destroy() }
    }
}
