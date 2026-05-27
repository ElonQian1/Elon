package com.elon.app

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import org.json.JSONObject
import kotlin.math.abs

data class ChatProjectShare(
    val id: String,
    val name: String,
    val description: String?,
    val ownerAccount: String?,
    val memberCount: Int,
    val joinMode: String,
    val latestLog: String?,
    val source: String
)

internal fun AppProject.toChatProjectShare(): ChatProjectShare {
    val isJoint = isJointDevelopmentProject()
    return ChatProjectShare(
        id = if (isJoint) projectSpaceId() else id,
        name = title,
        description = subtitle.takeIf { it.isNotBlank() },
        ownerAccount = null,
        memberCount = conversations.size.coerceAtLeast(1),
        joinMode = "open",
        latestLog = events.firstOrNull()?.trim()?.takeIf { it.isNotBlank() },
        source = if (isJoint) "store" else "local"
    )
}

internal fun StoreProject.toChatProjectShare(): ChatProjectShare {
    return ChatProjectShare(
        id = id,
        name = name,
        description = description,
        ownerAccount = ownerAccount.takeIf { it.isNotBlank() && it != "?" },
        memberCount = memberCount,
        joinMode = joinMode,
        latestLog = lastTaskStatus?.takeIf { it.isNotBlank() },
        source = "store"
    )
}

internal fun ChatProjectShare.toMessageText(): String {
    val json = JSONObject()
        .put("id", id)
        .put("name", name)
        .put("description", description ?: "")
        .put("owner_account", ownerAccount ?: "")
        .put("member_count", memberCount)
        .put("join_mode", joinMode)
        .put("latest_log", latestLog ?: "")
        .put("source", source)
    return "$PROJECT_SHARE_MARKER\n$json"
}

internal fun parseChatProjectShareMessage(content: String): ChatProjectShare? {
    val trimmed = content.trim()
    if (!trimmed.startsWith(PROJECT_SHARE_MARKER)) return null
    val jsonText = trimmed.removePrefix(PROJECT_SHARE_MARKER).trim()
    if (jsonText.isBlank()) return null
    return runCatching {
        val json = JSONObject(jsonText)
        ChatProjectShare(
            id = json.optString("id").trim(),
            name = json.optString("name").trim(),
            description = json.optString("description").trim().takeIf { it.isNotBlank() },
            ownerAccount = json.optString("owner_account").trim().takeIf { it.isNotBlank() },
            memberCount = json.optInt("member_count", 1).coerceAtLeast(1),
            joinMode = json.optString("join_mode", "open").ifBlank { "open" },
            latestLog = json.optString("latest_log").trim().takeIf { it.isNotBlank() },
            source = json.optString("source", "store").ifBlank { "store" }
        ).takeIf { it.id.isNotBlank() && it.name.isNotBlank() }
    }.getOrNull()
}

internal fun bindChatProjectShareView(
    container: LinearLayout?,
    text: TextView,
    message: ChatMessage,
    onProjectShareAction: ((ChatProjectShare) -> Unit)?
): Boolean {
    val share = parseChatProjectShareMessage(message.content) ?: return false
    text.text = ""
    text.visibility = View.GONE
    container ?: return true
    container.removeAllViews()
    container.visibility = View.VISIBLE
    val card = buildChatProjectShareCard(
        parent = container,
        share = share,
        role = message.role,
        onProjectShareAction = onProjectShareAction
    )
    container.addView(card)
    container.addView(projectShareInitiatorView(container, card, share, message))
    return true
}

internal fun applyChatProjectBubbleStyle(
    bubble: LinearLayout?,
    role: String,
    projectCardBound: Boolean
) {
    bubble ?: return
    val context = bubble.context
    if (projectCardBound) {
        bubble.background = ColorDrawable(Color.TRANSPARENT)
        bubble.setPadding(0, 0, 0, 0)
        return
    }
    bubble.setBackgroundResource(
        if (role == "user") R.drawable.bg_bubble_user else R.drawable.bg_bubble_ai
    )
    bubble.setPadding(context.projectDp(12), context.projectDp(9), context.projectDp(12), context.projectDp(9))
}

internal fun projectCardPaletteFor(key: String): IntArray {
    val hash = key.fold(0) { acc, c -> acc * 31 + c.code }
    return PROJECT_CARD_PALETTES[abs(hash) % PROJECT_CARD_PALETTES.size]
}

private fun buildChatProjectShareCard(
    parent: LinearLayout,
    share: ChatProjectShare,
    role: String,
    onProjectShareAction: ((ChatProjectShare) -> Unit)?
): View {
    val context = parent.context
    val width = context.resources.displayMetrics.widthPixels
        .coerceAtMost(context.projectDp(280)) - context.projectDp(18)
    val palette = projectCardPaletteFor(share.id)

    val card = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        background = GradientDrawable().apply {
            cornerRadius = context.projectDp(8).toFloat()
            setColor(Color.parseColor("#1E1E1E"))
        }
        clipToOutline = true
        layoutParams = LinearLayout.LayoutParams(width.coerceAtLeast(context.projectDp(210)), LinearLayout.LayoutParams.WRAP_CONTENT)
    }

    val banner = FrameLayout(context).apply {
        background = GradientDrawable(GradientDrawable.Orientation.TL_BR, palette).apply {
            cornerRadii = floatArrayOf(
                context.projectDp(8).toFloat(), context.projectDp(8).toFloat(),
                context.projectDp(8).toFloat(), context.projectDp(8).toFloat(),
                0f, 0f, 0f, 0f
            )
        }
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, context.projectDp(58))
    }
    banner.addView(TextView(context).apply {
        text = share.name.firstOrNull()?.uppercaseChar()?.toString() ?: "P"
        gravity = Gravity.CENTER
        includeFontPadding = false
        setTextColor(Color.WHITE)
        textSize = 20f
        background = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor("#55000000"))
        }
    }, FrameLayout.LayoutParams(context.projectDp(42), context.projectDp(42)).apply {
        gravity = Gravity.START or Gravity.CENTER_VERTICAL
        leftMargin = context.projectDp(14)
    })
    card.addView(banner)

    val body = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(context.projectDp(14), context.projectDp(11), context.projectDp(14), context.projectDp(14))
    }
    body.addView(TextView(context).apply {
        text = share.name
        textSize = 16f
        setTextColor(Color.parseColor("#F0F0F0"))
        setTypeface(typeface, android.graphics.Typeface.BOLD)
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
    })
    body.addView(TextView(context).apply {
        text = projectShareMetaText(share)
        textSize = 12f
        setTextColor(Color.parseColor("#8C8C8C"))
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
            topMargin = context.projectDp(5)
        }
    })

    val desc = share.description ?: share.latestLog
    if (!desc.isNullOrBlank()) {
        body.addView(TextView(context).apply {
            text = desc
            textSize = 13f
            setTextColor(Color.parseColor("#A0A0A0"))
            maxLines = 3
            ellipsize = TextUtils.TruncateAt.END
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
                topMargin = context.projectDp(8)
            }
        })
    }

    if (onProjectShareAction != null) {
        body.addView(TextView(context).apply {
            text = projectShareActionLabel(share, role)
            textSize = 14.5f
            gravity = Gravity.CENTER
            includeFontPadding = false
            setTextColor(Color.WHITE)
            background = GradientDrawable().apply {
                cornerRadius = context.projectDp(6).toFloat()
                setColor(Color.parseColor("#3BA55D"))
            }
            isClickable = true
            setOnClickListener { onProjectShareAction(share) }
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, context.projectDp(38)).apply {
                topMargin = context.projectDp(12)
            }
        })
    }
    card.addView(body)
    return card
}

private fun projectShareMetaText(share: ChatProjectShare): String {
    val owner = share.ownerAccount?.takeIf { it.isNotBlank() }
    return buildString {
        append("●  ${share.memberCount} 位成员")
        if (owner != null) append("  ·  创建者: $owner")
    }
}

private fun projectShareActionLabel(share: ChatProjectShare, role: String): String {
    return when {
        share.joinMode == "open" || role == "user" || share.source == "local" -> "加入项目"
        else -> "申请加入"
    }
}

private fun projectShareInitiatorView(
    parent: LinearLayout,
    card: View,
    share: ChatProjectShare,
    message: ChatMessage
): TextView {
    val context = parent.context
    val cardWidth = (card.layoutParams as? LinearLayout.LayoutParams)?.width
        ?.takeIf { it > 0 }
        ?: LinearLayout.LayoutParams.WRAP_CONTENT
    return TextView(context).apply {
        text = "发起者：${projectShareInitiatorName(share, message)}"
        textSize = 11.5f
        includeFontPadding = false
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        gravity = Gravity.CENTER
        setTextColor(Color.parseColor("#777777"))
        layoutParams = LinearLayout.LayoutParams(cardWidth, LinearLayout.LayoutParams.WRAP_CONTENT).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            topMargin = context.projectDp(7)
        }
    }
}

private fun projectShareInitiatorName(share: ChatProjectShare, message: ChatMessage): String {
    val sender = message.senderLabel?.trim()?.takeIf { it.isNotBlank() }
    val owner = share.ownerAccount?.trim()?.takeIf { it.isNotBlank() }
    return when {
        sender != null -> sender
        message.role == "user" -> "我"
        owner != null -> owner
        else -> "未知"
    }
}

private fun android.content.Context.projectDp(value: Int): Int {
    return (value * resources.displayMetrics.density + 0.5f).toInt()
}

private val PROJECT_CARD_PALETTES = arrayOf(
    intArrayOf(0xFF3B4F8A.toInt(), 0xFF2A3A73.toInt()),
    intArrayOf(0xFF5A3070.toInt(), 0xFF3E1F5A.toInt()),
    intArrayOf(0xFF2D6A4A.toInt(), 0xFF1B4A33.toInt()),
    intArrayOf(0xFF7A3535.toInt(), 0xFF5A2020.toInt()),
    intArrayOf(0xFF5A4A1A.toInt(), 0xFF3A3010.toInt()),
    intArrayOf(0xFF1A5A6A.toInt(), 0xFF0F3A4A.toInt()),
    intArrayOf(0xFF6A3A1A.toInt(), 0xFF4A260F.toInt()),
    intArrayOf(0xFF2A4A6A.toInt(), 0xFF1A3050.toInt())
)

private const val PROJECT_SHARE_MARKER = "【一龙项目卡片】"
