package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

data class ChatProjectPostCard(
    val title: String,
    val body: String,
    val authorName: String,
    val authorAvatarDataUrl: String? = null,
    val timeText: String,
    val topic: String,
    val imageSource: String?,
    val shareCount: String = "1",
    val commentCount: String = "0",
    val likeCount: String = "16"
)

internal fun bindChatProjectPostCardView(
    container: LinearLayout?,
    text: TextView,
    message: ChatMessage
): Boolean {
    val cardData = message.projectPostCard ?: return false
    text.text = ""
    text.visibility = View.GONE
    container ?: return true
    container.removeAllViews()
    container.visibility = View.VISIBLE
    container.addView(buildChatProjectPostCard(container.context, cardData))
    return true
}

private fun buildChatProjectPostCard(context: Context, post: ChatProjectPostCard): View {
    return LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(context.postDp(18), context.postDp(18), context.postDp(18), context.postDp(14))
        background = roundedPostBackground(context, "#181B20", POST_CARD_RADIUS_DP)
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        )

        addView(postHeader(context, post))
        addView(postTitle(context, post.title))
        val bodyText = post.body.ifBlank { post.title }
        if (bodyText.isNotBlank()) addView(postBody(context, bodyText))
        post.imageSource?.let { addView(postImage(context, it)) }
        addView(postMetrics(context, post))
    }
}

private fun postHeader(context: Context, post: ChatProjectPostCard): LinearLayout {
    return LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        addView(postAvatar(context, post.authorName, post.authorAvatarDataUrl), LinearLayout.LayoutParams(
            context.postDp(48),
            context.postDp(48)
        ).apply {
            marginEnd = context.postDp(12)
        })
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(context).apply {
                text = post.authorName
                textSize = 18f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(context).apply {
                text = post.timeText
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                setPadding(0, context.postDp(4), 0, 0)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        addView(TextView(context).apply {
            text = post.topic
            textSize = 17f
            setTextColor(Color.parseColor("#A6AFBD"))
            gravity = Gravity.CENTER_VERTICAL
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
        }, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            marginStart = context.postDp(10)
        })
    }
}

private fun postAvatar(context: Context, authorName: String, avatarDataUrl: String?): View {
    val bitmap = UserProfileStore.decodeAvatar(avatarDataUrl)
    if (bitmap != null) {
        return TextView(context).apply {
            background = RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                cornerRadius = context.postDp(24).toFloat()
                setAntiAlias(true)
            }
        }
    }
    return TextView(context).apply {
        text = UserProfileStore.avatarInitial(authorName.ifBlank { "成员" })
        gravity = Gravity.CENTER
        includeFontPadding = false
        textSize = 18f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor("#101010"))
        background = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(Color.parseColor("#D8D8D8"))
        }
    }
}

private fun postTitle(context: Context, titleText: String): TextView {
    return TextView(context).apply {
        text = titleText
        textSize = 19f
        setTextColor(Color.parseColor("#F2F5FA"))
        setLineSpacing(context.postDp(4).toFloat(), 1f)
        setPadding(0, context.postDp(20), 0, 0)
    }
}

private fun postBody(context: Context, bodyText: String): TextView {
    return TextView(context).apply {
        text = bodyText
        textSize = 17f
        setTextColor(Color.parseColor("#A6AFBD"))
        setLineSpacing(context.postDp(4).toFloat(), 1f)
        setPadding(0, context.postDp(10), 0, 0)
    }
}

private fun postImage(context: Context, source: String): ImageView {
    return ImageView(context).apply {
        tag = source
        contentDescription = "帖子图片"
        scaleType = ImageView.ScaleType.CENTER_CROP
        setImageResource(android.R.drawable.ic_menu_gallery)
        background = roundedPostBackground(context, "#22262C", 6)
        setOnClickListener {
            val attachment = if (source.startsWith("http://", true) || source.startsWith("https://", true)) {
                ChatAttachment(kind = "image", displayName = "帖子图片", mimeType = "image/*", url = source)
            } else {
                ChatAttachment(kind = "image", displayName = "帖子图片", mimeType = "image/*", localPath = source)
            }
            ChatImageViewer.show(context, attachment)
        }
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            context.postDp(150)
        ).apply {
            topMargin = context.postDp(16)
        }
        ChatImagePreviewLoader.cached(source)?.let {
            setImageBitmap(it)
        } ?: ChatImagePreviewLoader.load(context, source) { bitmap ->
            post {
                if (tag == source) setImageBitmap(bitmap)
            }
        }
    }
}

private fun postMetrics(context: Context, post: ChatProjectPostCard): LinearLayout {
    return LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(context.postDp(18), context.postDp(18), context.postDp(18), 0)
        addView(postMetric(context, "↗", post.shareCount), metricParams())
        addView(postMetric(context, "◌", post.commentCount), metricParams())
        addView(postMetric(context, "♡", post.likeCount), metricParams())
    }
}

private fun postMetric(context: Context, icon: String, value: String): TextView {
    return TextView(context).apply {
        text = "$icon $value"
        textSize = 16f
        gravity = Gravity.CENTER
        includeFontPadding = false
        setTextColor(Color.parseColor("#A6AFBD"))
    }
}

private fun metricParams(): LinearLayout.LayoutParams {
    return LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
}

private fun roundedPostBackground(context: Context, colorHex: String, radiusDp: Int): GradientDrawable {
    return GradientDrawable().apply {
        setColor(Color.parseColor(colorHex))
        cornerRadius = context.postDp(radiusDp).toFloat()
    }
}

private fun Context.postDp(value: Int): Int {
    return (value * resources.displayMetrics.density + 0.5f).toInt()
}

private const val POST_CARD_RADIUS_DP = 18
