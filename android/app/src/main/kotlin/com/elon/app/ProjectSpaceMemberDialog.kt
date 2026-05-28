package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal object ProjectSpaceMemberDialog {
    fun show(
        activity: AppCompatActivity,
        projectTitle: String,
        members: List<ProjectMember>,
        dp: (Int) -> Int
    ) {
        val content = ScrollView(activity).apply {
            isFillViewport = false
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(18), dp(8), dp(18), dp(8))
                if (members.isEmpty()) {
                    addView(emptyRow(activity, dp))
                } else {
                    members.forEach { member ->
                        addView(memberRow(activity, member, dp))
                    }
                }
            })
        }

        AlertDialog.Builder(activity)
            .setTitle("${projectTitle.ifBlank { "项目" }}成员")
            .setView(content)
            .setPositiveButton("关闭", null)
            .show()
    }

    private fun memberRow(
        activity: AppCompatActivity,
        member: ProjectMember,
        dp: (Int) -> Int
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(10), 0, dp(10))
            addView(avatar(activity, member.account, dp))
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(12), 0, 0, 0)
                addView(TextView(activity).apply {
                    text = member.account.ifBlank { "成员" }
                    textSize = 16f
                    setTypeface(typeface, Typeface.BOLD)
                    setTextColor(Color.parseColor("#E6E6E6"))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = memberSubtitle(member)
                    textSize = 12f
                    setTextColor(Color.parseColor("#9A9A9A"))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        }
    }

    private fun avatar(activity: AppCompatActivity, account: String, dp: (Int) -> Int): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(40), dp(40))
            gravity = Gravity.CENTER
            text = account.trim().take(1).ifBlank { "成" }.uppercase()
            textSize = 15f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.WHITE)
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor("#3A3A3A"))
            }
        }
    }

    private fun emptyRow(activity: AppCompatActivity, dp: (Int) -> Int): TextView {
        return TextView(activity).apply {
            text = "暂无成员"
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#9A9A9A"))
            setPadding(dp(20), dp(42), dp(20), dp(42))
        }
    }

    private fun memberSubtitle(member: ProjectMember): String {
        val role = when (member.role) {
            "owner" -> "所有者"
            "editor" -> "协作者"
            "member" -> "成员"
            else -> member.role.ifBlank { "成员" }
        }
        return member.joinedAt.takeIf { it.isNotBlank() }?.let { "$role · 加入于 $it" } ?: role
    }
}
