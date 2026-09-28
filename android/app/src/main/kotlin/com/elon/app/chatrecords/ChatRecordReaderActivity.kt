package com.elon.app.chatrecords

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.LinearLayout
import android.widget.ImageButton
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AlertDialog
import androidx.lifecycle.ViewModelProvider
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.AuthManager
import com.elon.app.articles.ArticleUi
import com.elon.app.sharing.installShareWindow

class ChatRecordReaderActivity : AppCompatActivity() {
    private lateinit var model: ChatRecordReaderModel
    private lateinit var ui: ArticleUi
    private lateinit var header: LinearLayout
    private lateinit var list: RecyclerView
    private lateinit var rowView: ChatRecordRowView
    private val rows = mutableListOf<RecordRow>()
    private var video: android.widget.VideoView? = null
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        model = ViewModelProvider(this)[ChatRecordReaderModel::class.java]; ui = ArticleUi(this)
        rowView = ChatRecordRowView(ui, model, ::openMedia, ::move)
        if (savedInstanceState != null) model.parent = savedInstanceState.getString("parent")
        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        header = ui.column(); root.addView(header)
        list = RecyclerView(this).apply { layoutManager = LinearLayoutManager(this@ChatRecordReaderActivity); adapter = RecordAdapter() }
        root.addView(list, LinearLayout.LayoutParams(-1, 0, 1f)); installShareWindow(root)
        model.changed.observe(this) { render() }
        model.start(intent.getStringExtra("group").orEmpty(), intent.getStringExtra("record").orEmpty(), intent.getStringExtra("draft").orEmpty())
        onBackPressedDispatcher.addCallback(this, object : androidx.activity.OnBackPressedCallback(true) {
            override fun handleOnBackPressed() { back() }
        })
    }
    override fun onResume() { super.onResume(); if (::model.isInitialized && !model.sameAccount()) finish() }
    override fun onStop() { video?.stopPlayback(); super.onStop() }
    override fun onSaveInstanceState(outState: Bundle) { outState.putString("parent", model.parent); super.onSaveInstanceState(outState) }
    private fun move(parent: String?) {
        model.offsets[model.parent.orEmpty()] = (list.layoutManager as LinearLayoutManager).findFirstVisibleItemPosition().coerceAtLeast(0)
        model.parent = parent; render()
    }
    private fun back() {
        val current = model.parent
        if (current == null) finish() else move(model.document?.messages?.firstOrNull { it.id == current }?.parentId)
    }
    private fun render() {
        header.removeAllViews()
        val top = ui.row()
        top.addView(ImageButton(this).apply { setImageResource(androidx.appcompat.R.drawable.abc_ic_ab_back_material); contentDescription = "返回"; background = null; setColorFilter(com.elon.app.MobileColors(this@ChatRecordReaderActivity).text); setOnClickListener { back() } }, LinearLayout.LayoutParams(ui.dp(48), ui.dp(48)))
        top.addView(ui.text(if (model.parent == null) model.document?.title ?: "聊天记录" else "转发的聊天记录", 20f), LinearLayout.LayoutParams(0, -2, 1f))
        top.addView(ImageButton(this).apply { setImageResource(com.elon.app.R.drawable.ic_more_vertical); contentDescription = "更多"; background = null; setColorFilter(com.elon.app.MobileColors(this@ChatRecordReaderActivity).text); setOnClickListener { options(this) } }, LinearLayout.LayoutParams(ui.dp(48), ui.dp(48)))
        header.addView(top)
        if (model.loading) header.addView(ui.text("正在读取…", 14f, true))
        if (model.notice.isNotBlank()) header.addView(ui.text(model.notice, 14f))
        if (model.document == null && !model.loading) header.addView(ui.button("重试") { model.refresh() })
        val doc = model.document
        if (doc != null && model.parent == null) {
            header.addView(ui.text("${doc.children(null).size} 条记录 · 微信导出", 13f, true))
            if (doc.warnings.isNotEmpty()) header.addView(ui.button("${doc.warnings.size} 项导入提示") {
                AlertDialog.Builder(this).setTitle("导入提示").setMessage(doc.warnings.joinToString("\n")).setPositiveButton("关闭", null).show()
            })
        }
        rows.clear(); rows.addAll(doc?.children(model.parent).orEmpty()); list.adapter?.notifyDataSetChanged()
        list.scrollToPosition(model.offsets[model.parent.orEmpty()] ?: 0)
    }
    private fun options(anchor: View) {
        val doc = model.document ?: return
        android.widget.PopupMenu(this, anchor).apply {
            menu.add("原始文本").setOnMenuItemClickListener {
                AlertDialog.Builder(this@ChatRecordReaderActivity).setTitle("导出原文")
                    .setView(android.widget.ScrollView(this@ChatRecordReaderActivity).apply { addView(ui.text(doc.rawText, 14f).apply { setTextIsSelectable(true) }) })
                    .setPositiveButton("关闭", null).show(); true
            }
            if (model.owner.isNotBlank() && model.owner == AuthManager.userId(this@ChatRecordReaderActivity)) menu.add("撤回分享").setOnMenuItemClickListener {
                AlertDialog.Builder(this@ChatRecordReaderActivity).setMessage("撤回后群成员将不能再读取此记录。已保存到其他位置的内容不会删除。")
                    .setNegativeButton("取消", null).setPositiveButton("撤回") { _, _ -> model.revoke() }.show(); true
            }
        }.show()
    }
    private inner class RecordAdapter : RecyclerView.Adapter<RecordHolder>() {
        override fun getItemCount() = rows.size
        override fun onCreateViewHolder(parent: android.view.ViewGroup, type: Int) = RecordHolder(ui.column().apply { layoutParams = RecyclerView.LayoutParams(-1, -2) })
        override fun onBindViewHolder(holder: RecordHolder, position: Int) {
            rowView.bind(holder.body, rows[position])
        }
    }
    private class RecordHolder(val body: LinearLayout) : RecyclerView.ViewHolder(body)
    private fun openMedia(row: RecordRow) {
        model.file(row) { result -> if (!isFinishing && !isDestroyed) result.onSuccess { file ->
            ChatRecordMedia.open(this, row, file) { video = it }
        }.onFailure { android.widget.Toast.makeText(this, it.message ?: "附件读取失败", android.widget.Toast.LENGTH_LONG).show() } }
    }
    companion object {
        fun open(context: Context, group: String, record: String) = context.startActivity(Intent(context, ChatRecordReaderActivity::class.java).putExtra("group", group).putExtra("record", record))
        fun preview(context: Context, draft: String) = context.startActivity(Intent(context, ChatRecordReaderActivity::class.java).putExtra("draft", draft))
    }
}
