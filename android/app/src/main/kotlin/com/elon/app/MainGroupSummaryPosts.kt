package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import io.noties.markwon.Markwon
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class MainGroupSummaryPosts(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val onPostsChanged: () -> Unit
) {
    private val handler = Handler(Looper.getMainLooper())
    private var activeGroup: AppGroup? = null
    private var posts: List<GroupSummaryPost> = emptyList()
    private var loading = false

    fun openGroup(group: AppGroup) {
        activeGroup = group
        posts = emptyList()
        showLoading()
        bindStrip(null)
        refreshPosts(group, silent = false)
    }

    fun clear() {
        activeGroup = null
        posts = emptyList()
        loading = false
        handler.removeCallbacksAndMessages(null)
        binding.groupSummaryStrip.visibility = View.GONE
        binding.groupSummaryStrip.setOnClickListener(null)
        binding.groupSummaryActionButton.setOnClickListener(null)
    }

    fun showPosts(group: AppGroup? = activeGroup) {
        val target = group ?: return
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(8), 0, dp(8))
        }
        val scroll = ScrollView(activity).apply { addView(column) }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("${target.name} · AI 总结帖")
            .setView(scroll)
            .setNegativeButton("生成总结帖", null)
            .setPositiveButton("关闭", null)
            .create()
        dialog.setOnShowListener {
            renderPostRows(column, dialog)
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener {
                createSummaryPost(target, dialog)
            }
            refreshPosts(target, silent = true) { fresh ->
                if (dialog.isShowing) renderPostRows(column, dialog, fresh)
            }
        }
        dialog.show()
    }

    fun refreshPosts(group: AppGroup? = activeGroup, silent: Boolean = true) {
        val target = group ?: return
        refreshPosts(target, silent, onLoaded = null)
    }

    private fun refreshPosts(
        group: AppGroup,
        silent: Boolean,
        onLoaded: ((List<GroupSummaryPost>) -> Unit)?
    ) {
        if (!silent) showLoading()
        thread(name = "group-summary-posts-load") {
            val result = runCatching { fetchSummaryPosts(group) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                loading = false
                result
                    .onSuccess { loaded ->
                        posts = loaded
                        bindStrip(loaded.firstOrNull())
                        onLoaded?.invoke(loaded)
                    }
                    .onFailure { error ->
                        bindError(error.message ?: "加载总结帖失败")
                        if (!silent) {
                            Toast.makeText(activity, error.message ?: "加载总结帖失败", Toast.LENGTH_SHORT).show()
                        }
                    }
            }
        }
    }

    private fun createSummaryPost(group: AppGroup, dialog: AlertDialog? = null) {
        if (loading) return
        loading = true
        bindCreating()
        Toast.makeText(activity, "AI 正在生成群聊总结帖", Toast.LENGTH_SHORT).show()
        thread(name = "group-summary-post-create") {
            val result = runCatching { postSummaryCreate(group) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                loading = false
                result
                    .onSuccess { created ->
                        posts = listOf(created) + posts.filterNot { it.id == created.id }
                        bindStrip(created)
                        onPostsChanged()
                        dialog?.dismiss()
                        schedulePostRefresh(group, created.id)
                    }
                    .onFailure { error ->
                        bindError(error.message ?: "生成总结帖失败")
                        Toast.makeText(activity, error.message ?: "生成总结帖失败", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    private fun schedulePostRefresh(group: AppGroup, postId: String) {
        listOf(1800L, 4000L, 8000L).forEach { delay ->
            handler.postDelayed({
                if (activeGroup?.id == group.id) refreshPosts(group, silent = true)
            }, delay)
        }
        handler.postDelayed({
            val current = posts.firstOrNull { it.id == postId }
            if (activeGroup?.id == group.id && current != null && current.isGenerating()) {
                refreshPosts(group, silent = true)
            }
        }, 14000L)
    }

    private fun showLoading() {
        loading = true
        binding.groupSummaryStrip.visibility = View.VISIBLE
        binding.groupSummaryTitleText.text = "AI 总结帖"
        binding.groupSummaryMetaText.text = "正在读取群聊总结帖..."
        binding.groupSummaryActionButton.text = "查看"
        binding.groupSummaryStrip.setOnClickListener { showPosts() }
        binding.groupSummaryActionButton.setOnClickListener { showPosts() }
    }

    private fun bindCreating() {
        binding.groupSummaryStrip.visibility = View.VISIBLE
        binding.groupSummaryTitleText.text = "AI 正在生成总结帖"
        binding.groupSummaryMetaText.text = "系统正在打包最近聊天 Context Pack"
        binding.groupSummaryActionButton.text = "生成中"
    }

    private fun bindError(message: String) {
        binding.groupSummaryStrip.visibility = View.VISIBLE
        binding.groupSummaryTitleText.text = "AI 总结帖"
        binding.groupSummaryMetaText.text = message
        binding.groupSummaryActionButton.text = "重试"
        binding.groupSummaryStrip.setOnClickListener { activeGroup?.let { refreshPosts(it, silent = false) } }
        binding.groupSummaryActionButton.setOnClickListener { activeGroup?.let { refreshPosts(it, silent = false) } }
    }

    private fun bindStrip(post: GroupSummaryPost?) {
        binding.groupSummaryStrip.visibility = View.VISIBLE
        if (post == null) {
            binding.groupSummaryTitleText.text = "AI 总结帖"
            binding.groupSummaryMetaText.text = "暂无总结帖，点击生成最近群聊总结"
            binding.groupSummaryActionButton.text = "生成"
            binding.groupSummaryStrip.setOnClickListener { activeGroup?.let { createSummaryPost(it) } }
            binding.groupSummaryActionButton.setOnClickListener { activeGroup?.let { createSummaryPost(it) } }
            return
        }
        binding.groupSummaryTitleText.text = "${if (post.isPinned) "置顶总结" else "最新总结"} · ${post.title}"
        binding.groupSummaryMetaText.text = post.stripMeta()
        binding.groupSummaryActionButton.text = if (post.isGenerating()) "刷新" else "查看"
        binding.groupSummaryStrip.setOnClickListener { showPostDetail(post) }
        binding.groupSummaryActionButton.setOnClickListener {
            if (post.isGenerating()) activeGroup?.let { refreshPosts(it, silent = false) } else showPostDetail(post)
        }
    }

    private fun renderPostRows(
        column: LinearLayout,
        dialog: AlertDialog,
        rows: List<GroupSummaryPost> = posts
    ) {
        column.removeAllViews()
        if (rows.isEmpty()) {
            column.addView(textBlock("还没有总结帖。\n点击“生成总结帖”，AI 会把最近群聊内容打包成 Context Pack 后生成置顶帖。"))
            return
        }
        rows.forEach { post ->
            column.addView(postRow(post) {
                dialog.dismiss()
                showPostDetail(post)
            })
        }
        column.addView(textBlock("默认按置顶优先、更新时间排序。生成新帖会自动置顶。", quiet = true))
    }

    private fun showPostDetail(post: GroupSummaryPost) {
        val group = activeGroup ?: return
        val status = TextView(activity).apply {
            text = "正在读取总结帖..."
            setTextColor(Color.parseColor("#AFAFAF"))
            textSize = 14f
            gravity = Gravity.CENTER
            setPadding(dp(18), dp(36), dp(18), dp(36))
        }
        val scroll = ScrollView(activity).apply { addView(status) }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(post.title)
            .setView(scroll)
            .setNeutralButton(if (post.isPinned) "取消置顶" else "置顶", null)
            .setNegativeButton("刷新", null)
            .setPositiveButton("关闭", null)
            .create()
        dialog.setOnShowListener {
            loadPostDetail(group, post.id, scroll)
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener {
                loadPostDetail(group, post.id, scroll)
            }
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                patchPinned(group, post, !post.isPinned, dialog)
            }
        }
        dialog.show()
    }

    private fun loadPostDetail(group: AppGroup, postId: String, scroll: ScrollView) {
        val loadingView = textBlock("正在读取总结帖...")
        scroll.removeAllViews()
        scroll.addView(loadingView)
        thread(name = "group-summary-post-detail") {
            val result = runCatching { fetchSummaryPostDetail(group, postId) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                result
                    .onSuccess { detail ->
                        scroll.removeAllViews()
                        scroll.addView(detailView(detail))
                    }
                    .onFailure { error ->
                        scroll.removeAllViews()
                        scroll.addView(textBlock(error.message ?: "读取总结帖失败"))
                    }
            }
        }
    }

    private fun patchPinned(
        group: AppGroup,
        post: GroupSummaryPost,
        pinned: Boolean,
        dialog: AlertDialog
    ) {
        thread(name = "group-summary-post-pin") {
            val result = runCatching { updateSummaryPostPinned(group, post.id, pinned) }
            activity.runOnUiThread {
                if (activeGroup?.id != group.id) return@runOnUiThread
                result
                    .onSuccess {
                        dialog.dismiss()
                        Toast.makeText(activity, if (pinned) "已置顶总结帖" else "已取消置顶", Toast.LENGTH_SHORT).show()
                        refreshPosts(group, silent = true)
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, error.message ?: "更新置顶状态失败", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    private fun detailView(detail: GroupSummaryPostDetail): LinearLayout {
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(14), dp(18), dp(18))
        }
        column.addView(textBlock(detail.post.stripMeta(), quiet = true))
        val summary = TextView(activity).apply {
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 15f
            setLineSpacing(dp(3).toFloat(), 1f)
            linksClickable = true
        }
        markwon().setMarkdown(summary, detail.post.summary.ifBlank { "AI 正在生成总结帖，请稍后刷新。" })
        column.addView(summary)
        if (detail.sources.isNotEmpty()) {
            column.addView(sectionTitle("来源消息"))
            detail.sources.take(12).forEach { source ->
                column.addView(textBlock("${source.senderName} · ${source.createdAt}\n${source.content}", quiet = true))
            }
        }
        return column
    }

    private fun postRow(post: GroupSummaryPost, onClick: () -> Unit): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = roundedBg("#1F2023", dp(16))
            setPadding(dp(16), dp(14), dp(16), dp(14))
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
            lp.setMargins(dp(12), dp(6), dp(12), dp(6))
            layoutParams = lp
            addView(TextView(activity).apply {
                text = "${if (post.isPinned) "置顶" else "总结"} · ${post.title}"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 16f
                typeface = Typeface.DEFAULT_BOLD
                maxLines = 1
            })
            addView(TextView(activity).apply {
                text = post.stripMeta()
                setTextColor(Color.parseColor("#AFAFAF"))
                textSize = 13f
                setPadding(0, dp(8), 0, 0)
            })
            setOnClickListener { onClick() }
        }
    }

    private fun sectionTitle(text: String): TextView {
        return TextView(activity).apply {
            this.text = text
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 16f
            typeface = Typeface.DEFAULT_BOLD
            setPadding(0, dp(18), 0, dp(6))
        }
    }

    private fun textBlock(text: String, quiet: Boolean = false): TextView {
        return TextView(activity).apply {
            this.text = text
            setTextColor(Color.parseColor(if (quiet) "#AFAFAF" else "#D9D9D9"))
            textSize = if (quiet) 13f else 15f
            setLineSpacing(dp(3).toFloat(), 1f)
            setPadding(dp(16), dp(14), dp(16), dp(14))
        }
    }

    private fun fetchSummaryPosts(group: AppGroup): List<GroupSummaryPost> {
        return GroupSummaryPostsApi.fetchPosts(activity, http, serverUrl, group)
    }

    private fun fetchSummaryPostDetail(group: AppGroup, postId: String): GroupSummaryPostDetail {
        return GroupSummaryPostsApi.fetchDetail(activity, http, serverUrl, group, postId)
    }

    private fun postSummaryCreate(group: AppGroup): GroupSummaryPost {
        return GroupSummaryPostsApi.create(activity, http, serverUrl, group)
    }

    private fun updateSummaryPostPinned(group: AppGroup, postId: String, pinned: Boolean) {
        GroupSummaryPostsApi.updatePinned(activity, http, serverUrl, group, postId, pinned)
    }

    private fun markwon(): Markwon {
        return Markwon.builder(activity)
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(activity))
            .build()
    }

    private fun roundedBg(color: String, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(color))
            cornerRadius = radius.toFloat()
        }
    }

    private fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density + 0.5f).toInt()
    }
}
