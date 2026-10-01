package com.elon.app

import android.content.Context
import android.graphics.BitmapFactory
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.Base64
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

internal class GroupMemberViews(val context: Context) {
    val colors = MobileColors(context)
    fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
    fun label(value: String, muted: Boolean = false) = TextView(context).apply {
        text = value; textSize = if (muted) 13f else 16f; setTextColor(if (muted) colors.muted else colors.text)
    }
    fun button(value: String, action: () -> Unit) = Button(context).apply {
        text = value; minHeight = dp(48); setTextColor(colors.primary); isAllCaps = false; setOnClickListener { action() }
    }
    fun column() = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(colors.surface) }
    fun row() = LinearLayout(context).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
    fun avatar(person: RosterPerson, size: Int = 40): View = FrameLayout(context).apply {
        layoutParams = LinearLayout.LayoutParams(dp(size), dp(size))
        background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(this@GroupMemberViews.colors.elevated) }
        addView(label(person.name.take(1)).apply { gravity = Gravity.CENTER }, FrameLayout.LayoutParams(-1, -1))
        person.avatar?.takeIf { it.startsWith("data:image/") }?.let { data ->
            runCatching { val bytes = Base64.decode(data.substringAfter(','), Base64.DEFAULT); BitmapFactory.decodeByteArray(bytes, 0, bytes.size) }.getOrNull()?.let { bitmap ->
                addView(ImageView(context).apply { setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply { isCircular = true }); importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }, FrameLayout.LayoutParams(-1, -1))
            }
        }
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    fun grid(group: AppGroup, open: () -> Unit): View {
        val outer = column().apply { setPadding(dp(16), dp(12), dp(16), dp(12)) }
        val columns = if (context.resources.configuration.screenWidthDp >= 400 && context.resources.configuration.fontScale <= 1.2f) 5 else 4
        group.members.take(columns * 3).chunked(columns).forEach { people ->
            val row = row()
            people.forEach { member ->
                val tile = column().apply {
                    gravity = Gravity.CENTER; setPadding(dp(4), dp(8), dp(4), dp(8)); minimumHeight = dp(88)
                    isClickable = true; isFocusable = true; contentDescription = "${member.displayName}，查看群成员"
                    addView(avatar(RosterPerson(member.id, member.displayName, member.avatarDataUrl, "member", ""), 44))
                    addView(label(member.displayName, true).apply { maxLines = 1; ellipsize = android.text.TextUtils.TruncateAt.END; gravity = Gravity.CENTER })
                    setOnClickListener { open() }
                }
                row.addView(tile, LinearLayout.LayoutParams(0, -2, 1f))
            }
            repeat(columns - people.size) { row.addView(View(context), LinearLayout.LayoutParams(0, 1, 1f)) }
            outer.addView(row)
        }
        outer.addView(button("查看全部成员（${group.memberCount}）", open))
        return outer
    }
    private data class PersonViews(val check: CheckBox, val avatar: FrameLayout, val name: TextView, val detail: TextView, var person: RosterPerson? = null)
    fun personRow(person: RosterPerson, self: String, checked: Boolean?, recycled: View? = null, action: () -> Unit): View {
        val result = (recycled as? LinearLayout)?.takeIf { it.tag is PersonViews } ?: row().apply {
            minimumHeight = dp(64); setPadding(dp(12), dp(8), dp(12), dp(8))
            val check = CheckBox(context).apply { isClickable = false; isFocusable = false; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }; addView(check)
            val icon = FrameLayout(context); addView(icon, LinearLayout.LayoutParams(dp(40), dp(40)))
            val name = label(""); val detail = label("", true)
            addView(column().apply {
                setPadding(dp(12), 0, 0, 0)
                addView(name); addView(detail)
            }, LinearLayout.LayoutParams(0, -2, 1f))
            tag = PersonViews(check, icon, name, detail); isClickable = true; isFocusable = true
        }
        return result.apply {
            val holder = tag as PersonViews
            holder.check.visibility = if (checked == null) View.GONE else View.VISIBLE; holder.check.isChecked = checked == true
            holder.name.text = person.name + if (person.id == self) "（我）" else ""
            holder.detail.text = if (person.role == "member") "入群 ${person.joinedAt.take(10)}" else person.roleLabel
            if (holder.person != person) { holder.avatar.removeAllViews(); holder.avatar.addView(avatar(person)); holder.person = person }
            setOnClickListener { action() }
            contentDescription = "${person.name}，${person.roleLabel}" + (checked?.let { if (it) "，已选择" else "，未选择" } ?: "")
        }
    }
}
