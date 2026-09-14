package com.elon.app

import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal class GroupMentionPicker(
    private val activity: AppCompatActivity,
    private val onSelected: (List<GroupMentionTarget>) -> Unit,
    private val onDismissed: () -> Unit,
) {
    private val dialog = BottomSheetDialog(activity)
    private var members = emptyList<GroupMentionTarget>()
    private var visible = emptyList<GroupMentionTarget>()
    private val selected = linkedMapOf<String, GroupMentionTarget>()
    private var multiple = false
    private val root = LinearLayout(activity).apply {
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16), dp(8), dp(16), dp(16))
        setBackgroundColor(color(R.color.elon_bg_app))
    }
    private val search = EditText(activity).apply {
        hint = "搜索群友或群 AI"
        contentDescription = hint
        isSingleLine = true
        textSize = 16f
        setTextColor(color(R.color.elon_text_primary))
        setHintTextColor(color(R.color.elon_text_tertiary))
        setPadding(dp(12), 0, dp(12), 0)
        background = rounded(R.color.elon_surface_search)
    }
    private val mode = button("多选") { multiple = !multiple; selected.clear(); render() }
    private val confirm = button("完成") {
        if (selected.isNotEmpty()) { onSelected(selected.values.toList()); dismiss() }
    }
    private val status = label("正在加载群成员…", 14f).apply { gravity = Gravity.CENTER; minHeight = dp(64) }
    private val retry = button("重新加载") {}
    private val adapter = MembersAdapter()
    private val list = RecyclerView(activity).apply {
        layoutManager = LinearLayoutManager(activity)
        adapter = this@GroupMentionPicker.adapter
    }

    init {
        root.addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(button("取消") { dismiss() }, LinearLayout.LayoutParams(dp(64), dp(48)))
            addView(label("选择提醒的人", 18f).apply {
                gravity = Gravity.CENTER; setTypeface(typeface, Typeface.BOLD)
            }, LinearLayout.LayoutParams(0, dp(48), 1f))
            addView(mode, LinearLayout.LayoutParams(dp(64), dp(48)))
        })
        root.addView(search, LinearLayout.LayoutParams(-1, dp(48)).apply { topMargin = dp(8); bottomMargin = dp(8) })
        root.addView(status, LinearLayout.LayoutParams(-1, -2))
        root.addView(retry, LinearLayout.LayoutParams(-1, dp(48)))
        root.addView(list, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(confirm, LinearLayout.LayoutParams(-1, dp(48)))
        search.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) { render() }
            override fun afterTextChanged(s: Editable?) = Unit
        })
        dialog.setContentView(root)
        dialog.window?.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
        dialog.setOnShowListener {
            dialog.findViewById<FrameLayout>(com.google.android.material.R.id.design_bottom_sheet)?.let {
                it.layoutParams.height = (activity.resources.displayMetrics.heightPixels * 0.78).toInt()
                BottomSheetBehavior.from(it).apply { state = BottomSheetBehavior.STATE_EXPANDED; skipCollapsed = true }
            }
        }
        dialog.setOnDismissListener { onDismissed() }
        showLoading()
    }

    fun show() = dialog.show()
    fun dismiss() = dialog.dismiss()

    fun showLoading() {
        members = emptyList(); visible = emptyList(); adapter.notifyDataSetChanged()
        search.isEnabled = false; mode.isEnabled = false
        status.visibility = View.VISIBLE; status.text = "正在加载群成员…"
        retry.visibility = View.GONE; confirm.visibility = View.GONE
    }

    fun showError(onRetry: () -> Unit) {
        status.text = "群成员加载失败，请重试"
        retry.visibility = View.VISIBLE
        retry.setOnClickListener { onRetry() }
    }

    fun showMembers(items: List<GroupMentionTarget>) {
        members = items; search.isEnabled = true; mode.isEnabled = true
        retry.visibility = View.GONE
        render()
    }

    private fun render() {
        visible = filterGroupMentions(members, search.text.toString())
        mode.text = if (multiple) "单选" else "多选"
        confirm.text = "完成（${selected.size}）"
        confirm.isEnabled = selected.isNotEmpty()
        confirm.visibility = if (multiple) View.VISIBLE else View.GONE
        status.text = if (members.isEmpty()) "暂无可选择的群成员" else "没有匹配的群友或群 AI"
        status.visibility = if (visible.isEmpty()) View.VISIBLE else View.GONE
        adapter.notifyDataSetChanged()
    }

    private inner class MemberHolder(val row: LinearLayout, val avatar: TextView, val name: TextView, val detail: TextView, val check: TextView) : RecyclerView.ViewHolder(row)

    private inner class MembersAdapter : RecyclerView.Adapter<MemberHolder>() {
        override fun getItemCount() = visible.size
        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): MemberHolder {
            val avatar = label("", 18f).apply { gravity = Gravity.CENTER }
            val name = label("", 16f).apply { maxLines = 2 }
            val detail = label("", 12f).apply { setTextColor(color(R.color.elon_text_secondary)) }
            val check = label("", 20f).apply { gravity = Gravity.CENTER }
            val row = LinearLayout(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                minimumHeight = dp(72)
                setPadding(dp(4), dp(8), dp(4), dp(8))
                isFocusable = true
                addView(avatar, LinearLayout.LayoutParams(dp(44), dp(44)))
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    addView(name); addView(detail)
                }, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = dp(12) })
                addView(check, LinearLayout.LayoutParams(dp(40), dp(48)))
            }
            return MemberHolder(row, avatar, name, detail, check)
        }

        override fun onBindViewHolder(holder: MemberHolder, position: Int) {
            val item = visible[position]
            holder.name.text = item.name
            holder.detail.text = if (item.isAi) "群 AI · 一龙助手" else "群友"
            holder.avatar.text = if (item.isAi) "EL" else item.name.take(1)
            holder.avatar.background = rounded(R.color.elon_surface_card)
            UserProfileStore.decodeAvatar(item.avatar)?.let { bitmap ->
                holder.avatar.text = ""
                holder.avatar.background = RoundedBitmapDrawableFactory.create(activity.resources, bitmap).apply { cornerRadius = dp(8).toFloat() }
            }
            holder.check.text = if (selected.containsKey(item.id)) "✓" else "○"
            holder.check.visibility = if (multiple) View.VISIBLE else View.GONE
            holder.row.contentDescription = "${item.name}，${if (item.isAi) "群 AI" else "群友"}${if (selected.containsKey(item.id)) "，已选择" else ""}"
            holder.row.setOnClickListener {
                if (multiple) {
                    if (selected.remove(item.id) == null) selected[item.id] = item
                    render()
                } else { onSelected(listOf(item)); dismiss() }
            }
        }
    }

    private fun label(value: String, size: Float) = TextView(activity).apply { text = value; textSize = size; setTextColor(color(R.color.elon_text_primary)) }
    private fun button(value: String, action: () -> Unit) = Button(activity).apply { text = value; isAllCaps = false; minWidth = 0; minimumWidth = 0; setPadding(0, 0, 0, 0); setOnClickListener { action() } }
    private fun rounded(id: Int) = GradientDrawable().apply { setColor(color(id)); cornerRadius = dp(8).toFloat() }
    private fun color(id: Int) = ContextCompat.getColor(activity, id)
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
}
