package com.elon.app

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.SystemClock
import android.text.TextUtils
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.inputmethod.EditorInfo
import android.widget.BaseAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ListView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.elon.app.chatgptweb.ChatGptWebOfficialFallbackIntent

internal class WebChatLibraryBrowser(
    private val activity: AppCompatActivity,
    private val host: View,
    private val consumerPort: () -> WebChatConsumerPort?,
) {
    private var dialog: Dialog? = null
    private var owner: WebChatConsumerPort? = null
    private var page: WebChatLibrarySnapshot? = null
    private var directory = ""
    private var trail = emptyList<WebChatLibraryBreadcrumb>()
    private var query = ""
    private var requestId: String? = null
    private var poll: Runnable? = null
    private var busy = false
    private var failed = false
    private var title: TextView? = null
    private var status: TextView? = null
    private var search: EditText? = null
    private var more: Button? = null
    private var adapter: Entries? = null
    private var detail: AlertDialog? = null
    private val downloads = WebChatFileDownloadDialog(activity, host, consumerPort)
    private val mutations = WebChatLibraryMutationDialog(activity, host, consumerPort) {
        if (dialog?.isShowing == true) load("refresh")
    }

    fun show(): Boolean {
        if (activity.isFinishing || activity.isDestroyed) return false
        if (dialog?.isShowing == true) return true
        owner = consumerPort() ?: return false
        page = null
        directory = ""
        trail = emptyList()
        query = ""
        val window = Dialog(activity).apply {
            requestWindowFeature(Window.FEATURE_NO_TITLE)
            window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        }
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(color(R.color.elon_bg_app))
            isFocusableInTouchMode = true
            setPadding(dp(12), dp(16), dp(12), dp(12))
            contentDescription = "web-chat-library-browser"
            addView(header())
            addView(searchRow())
            status = text(13f, secondary = true).also {
                it.setPadding(dp(8), dp(8), dp(8), dp(8))
                it.contentDescription = "web-chat-library-status"
                it.accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
                addView(it)
            }
            val entries = Entries().also { adapter = it }
            addView(ListView(activity).apply {
                adapter = entries
                dividerHeight = 0
                contentDescription = "web-chat-library-list"
                setOnItemClickListener { _, _, position, _ ->
                    page?.items?.getOrNull(position)?.let { item ->
                        if (item.kind == "directory") navigate(item.handle) else showFile(item)
                    }
                }
            }, LinearLayout.LayoutParams(-1, 0, 1f))
            more = Button(activity).also {
                it.text = "加载更多"
                it.isAllCaps = false
                it.contentDescription = "web-chat-library-more"
                it.setOnClickListener { load("next") }
                addView(it, LinearLayout.LayoutParams(-1, dp(48)))
            }
            addView(Button(activity).apply {
                text = "官网文件库"
                isAllCaps = false
                contentDescription = "web-chat-library-official"
                setOnClickListener { openOfficial() }
            }, LinearLayout.LayoutParams(-1, dp(48)))
        }
        window.setContentView(root)
        root.requestFocus()
        window.setOnKeyListener { _, key, event ->
            if (key == KeyEvent.KEYCODE_BACK && event.action == KeyEvent.ACTION_UP && directory.isNotEmpty()) {
                back(); true
            } else false
        }
        window.setOnDismissListener {
            stopRead()
            detail?.dismiss()
            detail = null
            downloads.dismiss()
            mutations.dismiss()
            dialog = null
            owner = null
            page = null
            adapter = null
            title = null
            status = null
            search = null
            more = null
        }
        dialog = window
        window.show()
        window.window?.setSoftInputMode(android.view.WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE or
            android.view.WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN)
        window.window?.setLayout(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        load("open")
        return true
    }

    fun dismiss() { dialog?.dismiss() }

    private fun header() = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        addView(icon(R.drawable.ic_project_space_chevron_right, "返回", "web-chat-library-back") { back() }
            .apply { rotation = 180f }, LinearLayout.LayoutParams(dp(48), dp(48)))
        title = text(20f).also {
            it.text = "文件库"
            it.maxLines = 1
            it.ellipsize = TextUtils.TruncateAt.END
            addView(it, LinearLayout.LayoutParams(0, dp(48), 1f))
        }
        addView(icon(R.drawable.ic_side_menu_refresh, "刷新", "web-chat-library-refresh") { load("refresh") },
            LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun searchRow() = LinearLayout(activity).apply {
        gravity = Gravity.CENTER_VERTICAL
        search = EditText(activity).also {
            it.hint = "搜索文件"
            it.setSingleLine(true)
            it.setTextColor(color(R.color.elon_text_primary))
            it.setHintTextColor(color(R.color.elon_text_secondary))
            it.imeOptions = EditorInfo.IME_ACTION_SEARCH
            it.filters = arrayOf(android.text.InputFilter.LengthFilter(200))
            it.contentDescription = "web-chat-library-query"
            it.setOnEditorActionListener { _, action, _ ->
                if (action == EditorInfo.IME_ACTION_SEARCH) { searchNow(); true } else false
            }
            addView(it, LinearLayout.LayoutParams(0, dp(52), 1f))
        }
        addView(icon(R.drawable.ic_search_simple, "搜索", "web-chat-library-search") { searchNow() },
            LinearLayout.LayoutParams(dp(48), dp(48)))
    }

    private fun searchNow() {
        query = search?.text?.toString()?.trim().orEmpty()
        page = null
        load("open")
    }

    private fun navigate(handle: String) {
        trail = if (handle.isEmpty()) emptyList() else {
            val existing = trail.indexOfFirst { it.handle == handle }
            if (existing >= 0) trail.take(existing + 1)
            else trail + WebChatLibraryBreadcrumb(handle, page?.items?.firstOrNull { it.handle == handle }?.name.orEmpty())
        }
        directory = handle
        query = ""
        search?.setText("")
        page = null
        load("open")
    }

    private fun back() {
        if (directory.isEmpty()) dismiss()
        else navigate(trail.dropLast(1).lastOrNull()?.handle.orEmpty())
    }

    private fun stopRead() {
        poll?.let(host::removeCallbacks)
        poll = null
        requestId?.let { owner?.cancelLibraryFiles(it) }
        requestId = null
        busy = false
    }

    private fun load(operation: String) {
        stopRead()
        val port = owner ?: return
        if (consumerPort() !== port || dialog?.isShowing != true) { dismiss(); return }
        busy = true
        failed = false
        render()
        val started = SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (poll !== this || dialog?.isShowing != true) return
                if (consumerPort() !== port) { dismiss(); return }
                if (SystemClock.elapsedRealtime() - started >= 16_000) { finish(false); return }
                if (requestId == null) {
                    val result = port.requestLibraryFiles(directory, query, operation)
                    if (!result.accepted || result.requestId == null) {
                        if (result.error in setOf("bridge_not_ready", "adapter_generation_not_ready")) {
                            host.postDelayed(this, 500); return
                        }
                        finish(false); return
                    }
                    requestId = result.requestId
                }
                val snapshot = port.libraryFiles()?.takeIf {
                    it.requestId == requestId && it.directoryHandle == directory && it.query == query
                }
                if (snapshot != null && snapshot !== page) { page = snapshot; trail = snapshot.breadcrumbs; render() }
                val command = port.state().commandRequests.firstOrNull { it.id == requestId }
                when (command?.status) {
                    WebChatConsumerCommandStatus.SUCCEEDED -> finish(snapshot != null)
                    WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT -> finish(false)
                    else -> host.postDelayed(this, 300)
                }
            }
        }
        poll = task
        host.post(task)
    }

    private fun finish(success: Boolean) {
        if (!success) requestId?.let { owner?.cancelLibraryFiles(it) }
        poll?.let(host::removeCallbacks)
        poll = null
        requestId = null
        busy = false
        failed = !success
        render()
    }

    private fun render() {
        title?.text = trail.lastOrNull()?.name ?: "文件库"
        status?.text = WebChatLibraryPresentation.status(page, busy, failed)
        adapter?.notifyDataSetChanged()
        more?.visibility = if (page?.hasMore == true) View.VISIBLE else View.GONE
        more?.isEnabled = !busy
    }

    private fun showFile(file: WebChatLibraryEntry) {
        val port = owner ?: return
        detail?.dismiss()
        val actions = buildList {
            if (file.downloadHandle.isNotBlank()) add("下载")
            if (file.canRename) add("重命名")
            if (file.canTrash) add("移到最近删除")
            add("官网文件库")
        }
        detail = AlertDialog.Builder(activity).setTitle(file.name).setNegativeButton("关闭", null)
            .setItems(actions.toTypedArray()) { _, index ->
                if (consumerPort() !== port) return@setItems
                when (actions[index]) {
                    "重命名" -> mutations.show(port, file, "rename")
                    "移到最近删除" -> mutations.show(port, file, "trash")
                    "官网文件库" -> openOfficial()
                    else -> {
                        val result = port.downloadLibraryFile(file.handle, file.downloadHandle)
                        if (result.accepted && result.requestId != null) downloads.show(port, result.requestId)
                        else Toast.makeText(activity, "文件列表已变化，请刷新后重试", Toast.LENGTH_SHORT).show()
                    }
                }
            }.show()
    }

    private fun openOfficial() {
        dismiss()
        activity.startActivity(ChatGptWebOfficialFallbackIntent.create(activity, "https://chatgpt.com/library"))
    }

    private inner class Entries : BaseAdapter() {
        override fun getCount() = page?.items?.size ?: 0
        override fun getItem(position: Int) = page!!.items[position]
        override fun getItemId(position: Int) = position.toLong()
        override fun getView(position: Int, recycled: View?, parent: ViewGroup): View {
            val row = recycled as? LinearLayout ?: LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                minimumHeight = dp(68)
                setPadding(dp(12), dp(10), dp(12), dp(10))
                addView(text(16f).apply { maxLines = 2; ellipsize = TextUtils.TruncateAt.END })
                addView(text(12f, secondary = true).apply { maxLines = 1; ellipsize = TextUtils.TruncateAt.END })
            }
            val item = getItem(position)
            (row.getChildAt(0) as TextView).text = item.name
            (row.getChildAt(1) as TextView).text = WebChatLibraryPresentation.subtitle(item)
            row.contentDescription = "web-chat-library-entry:${item.handle}"
            return row
        }
    }

    private fun icon(drawable: Int, label: String, selector: String, action: () -> Unit) = ImageButton(activity).apply {
        setImageResource(drawable)
        scaleType = ImageView.ScaleType.CENTER_INSIDE
        setColorFilter(color(R.color.elon_icon_primary))
        setBackgroundColor(Color.TRANSPARENT)
        setPadding(dp(12), dp(12), dp(12), dp(12))
        contentDescription = selector
        androidx.appcompat.widget.TooltipCompat.setTooltipText(this, label)
        setOnClickListener { action() }
    }

    private fun text(size: Float, secondary: Boolean = false) = TextView(activity).apply {
        textSize = size
        gravity = Gravity.CENTER_VERTICAL
        setTextColor(color(if (secondary) R.color.elon_text_secondary else R.color.elon_text_primary))
    }
    private fun color(id: Int) = ContextCompat.getColor(activity, id)
    private fun dp(value: Int) = (activity.resources.displayMetrics.density * value).toInt()
}
