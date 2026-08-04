package com.elon.app

import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import kotlin.concurrent.thread

internal class ProjectSpaceFeedView(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val openPost: (ProjectChannel, ProjectChannelMessage) -> Unit,
    private val openPostComposer: () -> Unit,
    private val openProjectDescription: (ProjectSpace) -> Unit,
    private val openProjectMembers: () -> Unit,
    private val joinProject: () -> Unit,
    private val openProjectDocuments: () -> Unit,
    private val openProjectResources: () -> Unit,
    private val projectApkActionLabel: () -> String,
    private val downloadProjectApk: () -> Unit,
    private val replaceProjectPreviewImage: (ProjectSpace, Int) -> Unit
) {
    private val metricPrefs = activity.getSharedPreferences(POST_METRIC_PREFS, Context.MODE_PRIVATE)
    private val playStoreHeader = ProjectSpacePlayStoreHeaderView(
        activity = activity,
        dp = dp,
        selectableForeground = selectableForeground,
        openProjectMembers = openProjectMembers,
        joinProject = joinProject,
        openProjectDocuments = openProjectDocuments,
        openProjectResources = openProjectResources,
        projectApkActionLabel = projectApkActionLabel,
        downloadProjectApk = downloadProjectApk,
        replaceProjectPreviewImage = replaceProjectPreviewImage
    )

    @Suppress("UNUSED_PARAMETER")
    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>,
        loading: Boolean
    ) {
        val posts = feedPosts(space, messagesByChannel)
        container.addView(playStoreHeader.render(space, posts.size, projectPreviewImages(space)))
        container.addView(projectStoreContent(space, posts, loading))
    }

    private fun projectStoreContent(
        space: ProjectSpace,
        posts: List<ProjectSpaceFeedPost>,
        loading: Boolean
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor(PROJECT_SPACE_STORE_BG))
            setPadding(0, 0, 0, dp(28))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )

            addView(projectAboutSection(space))
            addView(projectFeedPanel(posts, loading))
        }
    }

    private fun projectAboutSection(space: ProjectSpace): LinearLayout {
        val editable = canEditProjectDescription(space.project.role)
        val description = space.project.description
            ?.trim()
            ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
        val bodyText = description ?: if (editable) "添加项目简介" else "暂无项目简介"
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            isClickable = true
            foreground = selectableForeground()
            contentDescription = if (editable) "编辑项目简介" else "查看项目简介"
            setOnClickListener { openProjectDescription(space) }

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    text = "关于此应用"
                    textSize = 20f
                    includeFontPadding = false
                    setTypeface(typeface, Typeface.BOLD)
                    setTextColor(Color.parseColor(PROJECT_SPACE_STORE_TEXT))
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                addView(aboutArrowButton(), LinearLayout.LayoutParams(dp(48), dp(48)))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            ))

            addView(TextView(activity).apply {
                text = bodyText
                textSize = 16f
                includeFontPadding = true
                setTextColor(Color.parseColor(if (description == null) "#777777" else PROJECT_SPACE_STORE_MUTED))
                setLineSpacing(dp(3).toFloat(), 1f)
                maxLines = 4
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(8)
            })
        }
    }

    private fun aboutArrowButton(): FrameLayout {
        return FrameLayout(activity).apply {
            addView(FrameLayout(activity).apply {
            background = roundedBackground("#172231", 12)
                addView(ImageView(activity).apply {
                    setImageResource(R.drawable.ic_project_space_chevron_right)
                    scaleType = ImageView.ScaleType.CENTER
                }, FrameLayout.LayoutParams(dp(18), dp(18), Gravity.CENTER))
            }, FrameLayout.LayoutParams(dp(36), dp(36), Gravity.CENTER))
        }
    }

    private fun projectFeedPanel(posts: List<ProjectSpaceFeedPost>, loading: Boolean): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, dp(86))
            minimumHeight = dp(460)
            setBackgroundColor(Color.parseColor(PROJECT_SPACE_STORE_BG))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(24)
            }

            addView(topicChips(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(28)
            ))
            when {
                posts.isNotEmpty() -> posts.forEachIndexed { index, post ->
                    val card = postCard(post)
                    val params = card.layoutParams as LinearLayout.LayoutParams
                    params.topMargin = if (index == 0) dp(18) else dp(10)
                    addView(card, params)
                }
                loading -> addView(emptyState("正在加载帖子...", showButton = false))
                else -> addView(emptyState("还没有帖子，点击+好发布内容", showButton = true))
            }
        }
    }

    private fun topicChips(): LinearLayout {
        val topics = listOf("需求", "讨论", "意见", "问题反馈")
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(24), 0, dp(24), 0)
            topics.forEachIndexed { index, topic ->
                addView(TextView(activity).apply {
                    text = topic
                    textSize = 12f
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setTextColor(Color.parseColor(PROJECT_SPACE_STORE_MUTED))
                    background = roundedStrokeBackground(PROJECT_SPACE_STORE_BG, 6, PROJECT_SPACE_STORE_DIVIDER, 1)
                    setPadding(dp(7), 0, dp(7), 0)
                }, LinearLayout.LayoutParams(
                    0,
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    1f
                ).apply {
                    if (index < topics.lastIndex) marginEnd = dp(13)
                })
            }
        }
    }

    private fun feedPosts(
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>
    ): List<ProjectSpaceFeedPost> {
        val channelsById = space.channels
            .filter { it.isProjectSpaceFeedChannel() }
            .associateBy { it.id }
        return channelsById.values.flatMap { channel ->
            val channelMessages = messagesByChannel[channel.id].orEmpty()
            val replyCounts = projectSpaceReplyCountsByPost(channelMessages)
            channelMessages
                .filter { it.isProjectSpaceFeedPost() }
                .map { ProjectSpaceFeedPost(channel, it, replyCounts[it.id] ?: 0) }
        }.sortedByDescending { parseChatMessageCreatedAt(it.message.createdAt) ?: 0L }
            .take(MAX_FEED_POSTS)
    }

    private fun projectPreviewImages(space: ProjectSpace): List<String?> {
        val manual = space.galleryImages.take(PROJECT_PREVIEW_SLOT_COUNT)
        val manualSet = manual.mapNotNull { it.cleanProjectSpaceDisplayName() }.toSet()
        val automatic = space.landingPreviewImages
            .mapNotNull { it.cleanProjectSpaceDisplayName() }
            .filterNot { it in manualSet }
            .distinct()
        return (0 until PROJECT_PREVIEW_SLOT_COUNT).map { index ->
            manual.getOrNull(index).cleanProjectSpaceDisplayName()
                ?: automatic.getOrNull(index)
        }
    }

    private fun postCard(post: ProjectSpaceFeedPost): LinearLayout {
        val postText = parseProjectSpacePostText(post.message.content)
        val sender = post.message.senderName.cleanProjectSpaceDisplayName() ?: "项目成员"
        val timeText = parseChatMessageCreatedAt(post.message.createdAt)
            ?.let { formatChatTimelineLabel(it) }
            ?: "刚刚"
        val bodyText = postBodyWithoutImages(postText.body).ifBlank { postText.title }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(25), 0, dp(14))
            background = roundedStrokeBackground(PROJECT_SPACE_STORE_BG, 14, PROJECT_SPACE_STORE_DIVIDER, 1)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPost(post.channel, post.message) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(dp(10), dp(10), dp(10), 0)
            }

            addView(
                postHeader(sender, post.message.senderAvatarDataUrl, timeText, projectSpaceTopicLabel(post.channel)),
                LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    leftMargin = dp(23)
                    rightMargin = dp(28)
                }
            )
            addView(TextView(activity).apply {
                text = postText.title
                textSize = 15f
                setTextColor(activity.elonColor(R.color.elon_text_primary))
                setTypeface(typeface, Typeface.BOLD)
                setLineSpacing(dp(4).toFloat(), 1f)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(14)
                topMargin = dp(20)
                rightMargin = dp(28)
            })
            addView(TextView(activity).apply {
                text = bodyText
                textSize = 15f
                setTextColor(Color.parseColor("#8E8E8E"))
                setLineSpacing(dp(3).toFloat(), 1f)
                maxLines = 3
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(14)
                topMargin = dp(6)
                rightMargin = dp(28)
            })
            extractProjectPostImageSource(postText.body)?.let { source ->
                addView(postImagePreview(source))
            }
            addView(postMetrics(post, postText), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun postHeader(sender: String, avatarDataUrl: String?, timeText: String, topic: String): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.TOP
            addView(avatar(sender, avatarDataUrl), LinearLayout.LayoutParams(dp(34), dp(34)).apply {
                marginEnd = dp(5)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.START
                addView(TextView(activity).apply {
                    text = sender
                    textSize = 15f
                    includeFontPadding = false
                    gravity = Gravity.START
                    setTextColor(activity.elonColor(R.color.elon_text_primary))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = "回复于$timeText"
                    textSize = 12f
                    includeFontPadding = false
                    gravity = Gravity.START
                    setTextColor(Color.parseColor("#777777"))
                    setPadding(0, dp(2), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                text = topic
                textSize = 14f
                includeFontPadding = false
                setTextColor(activity.elonColor(R.color.elon_text_primary))
                gravity = Gravity.CENTER
                maxLines = 1
                background = roundedBackground("#0B1017", 6)
                setPadding(dp(8), 0, dp(8), 0)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                dp(19)
            ))
        }
    }

    private fun avatar(sender: String, avatarDataUrl: String?): View {
        val bitmap = UserProfileStore.decodeAvatar(avatarDataUrl.cleanProjectSpaceDisplayName())
        if (bitmap != null) {
            return ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    setCircular(true)
                    setAntiAlias(true)
                })
            }
        }
        return TextView(activity).apply {
            text = sender.firstOrNull()?.toString() ?: "成"
            gravity = Gravity.CENTER
            includeFontPadding = false
            textSize = 17f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#101010"))
            background = roundedBackground("#7AA7FF", 17)
        }
    }

    private fun postImagePreview(source: String): ImageView {
        val image = ImageView(activity).apply {
            scaleType = ImageView.ScaleType.CENTER_CROP
            setBackgroundColor(Color.parseColor("#22262C"))
            setImageResource(android.R.drawable.ic_menu_gallery)
            tag = source
            layoutParams = LinearLayout.LayoutParams(
                dp(234),
                dp(112)
            ).apply {
                leftMargin = dp(14)
                topMargin = dp(11)
            }
        }
        thread(name = "project-post-image-preview") {
            val bitmap = runCatching {
                val bytes = ChatImageDiskCache.readBytes(activity, source, MAX_IMAGE_PREVIEW_BYTES)
                BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
            }.getOrNull()
            if (bitmap != null) {
                activity.runOnUiThread {
                    if (image.tag == source) image.setImageBitmap(bitmap)
                }
            }
        }
        return image
    }

    private fun postMetrics(post: ProjectSpaceFeedPost, postText: ProjectSpacePostText): LinearLayout {
        val key = post.metricKey()
        val shareCount = metricPrefs.getInt("$key:shares", 0)
        val liked = metricPrefs.getBoolean("$key:liked", false)
        val likeCount = if (liked) 1 else 0
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(21), 0, 0)
            addView(metricButton(
                iconRes = R.drawable.ic_project_space_post_share,
                value = shareCount.toString(),
                description = "分享帖子",
                onClick = { views ->
                    sharePost(post, postText) {
                        val nextCount = metricPrefs.getInt("$key:shares", 0) + 1
                        metricPrefs.edit().putInt("$key:shares", nextCount).apply()
                        views.value.text = nextCount.toString()
                    }
                }
            ), metricParams())
            addView(metricButton(
                iconRes = R.drawable.ic_project_space_post_comment,
                value = post.replyCount.coerceAtLeast(0).toString(),
                description = "查看${post.replyCount.coerceAtLeast(0)}条讨论",
                onClick = { openPost(post.channel, post.message) }
            ), metricParams())
            addView(metricButton(
                iconRes = R.drawable.ic_project_space_post_like,
                value = likeCount.toString(),
                description = if (liked) "取消点赞" else "点赞",
                selected = liked,
                onClick = { views ->
                    val nextLiked = !metricPrefs.getBoolean("$key:liked", false)
                    metricPrefs.edit().putBoolean("$key:liked", nextLiked).apply()
                    updateMetricButton(
                        views = views,
                        iconRes = R.drawable.ic_project_space_post_like,
                        value = if (nextLiked) "1" else "0",
                        selected = nextLiked,
                        description = if (nextLiked) "取消点赞" else "点赞"
                    )
                }
            ), metricParams())
        }
    }

    private fun metricButton(
        iconRes: Int,
        value: String,
        description: String,
        selected: Boolean = false,
        onClick: (MetricButtonViews) -> Unit
    ): LinearLayout {
        val color = metricColor(selected)
        val icon = ImageView(activity).apply {
            setImageResource(iconRes)
            setColorFilter(color)
        }
        val valueText = TextView(activity).apply {
            text = value
            textSize = 18f
            includeFontPadding = false
            setTextColor(color)
            gravity = Gravity.CENTER
        }
        val views = MetricButtonViews(icon, valueText)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            isClickable = true
            foreground = selectableForeground()
            contentDescription = description
            minimumHeight = dp(40)
            setPadding(dp(4), dp(5), dp(4), dp(5))
            addView(icon, LinearLayout.LayoutParams(dp(22), dp(22)).apply {
                marginEnd = dp(8)
            })
            addView(valueText, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
            setOnClickListener { onClick(views) }
        }
    }

    private fun updateMetricButton(
        views: MetricButtonViews,
        iconRes: Int,
        value: String,
        selected: Boolean,
        description: String
    ) {
        val color = metricColor(selected)
        views.icon.setImageResource(iconRes)
        views.icon.setColorFilter(color)
        views.value.text = value
        views.value.setTextColor(color)
        (views.icon.parent as? View)?.contentDescription = description
    }

    private fun metricColor(selected: Boolean): Int {
        return activity.elonColor(if (selected) R.color.elon_button_primary_bg else R.color.elon_text_primary)
    }

    private fun sharePost(
        post: ProjectSpaceFeedPost,
        postText: ProjectSpacePostText,
        onShared: () -> Unit
    ) {
        val topic = projectSpaceTopicLabel(post.channel)
        val shareText = buildString {
            append("【").append(postText.title).append("】")
            postBodyWithoutImages(postText.body).trim().takeIf { it.isNotBlank() }?.let { body ->
                append("\n\n").append(body)
            }
            append("\n\n来自项目话题：").append(topic)
        }
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, postText.title)
            putExtra(Intent.EXTRA_TEXT, shareText)
        }
        runCatching {
            activity.startActivity(Intent.createChooser(intent, "分享帖子"))
        }.onSuccess {
            onShared()
        }.onFailure { error ->
            Toast.makeText(activity, error.message ?: "无法打开系统分享", Toast.LENGTH_SHORT).show()
        }
    }

    private fun metricParams(): LinearLayout.LayoutParams {
        return LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
    }

    private fun emptyState(textValue: String, showButton: Boolean): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding(dp(20), dp(72), dp(20), dp(104))
            addView(TextView(activity).apply {
                text = textValue
                textSize = 15f
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#AFAFAF"))
            })
            if (showButton) {
                addView(TextView(activity).apply {
                    text = "+"
                    textSize = 24f
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setTextColor(activity.elonColor(R.color.elon_text_primary))
                    background = roundedBackground("#172231", 24)
                    isClickable = true
                    foreground = selectableForeground()
                    setOnClickListener { openPostComposer() }
                    contentDescription = "发布帖子"
                }, LinearLayout.LayoutParams(dp(48), dp(48)).apply {
                    topMargin = dp(28)
                })
            }
        }
    }

    private fun roundedBackground(colorHex: String, radiusDp: Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
        }
    }

    private fun roundedStrokeBackground(
        colorHex: String,
        radiusDp: Int,
        strokeColorHex: String,
        strokeWidthDp: Int
    ): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadius = dp(radiusDp).toFloat()
            setStroke(dp(strokeWidthDp), Color.parseColor(strokeColorHex))
        }
    }

    private companion object {
        const val MAX_FEED_POSTS = 40
        const val PROJECT_PREVIEW_SLOT_COUNT = 4
        const val MAX_IMAGE_PREVIEW_BYTES = 5 * 1024 * 1024
        const val POST_METRIC_PREFS = "project_post_metrics"
        const val PROJECT_SPACE_STORE_BG = "#0B1017"
        const val PROJECT_SPACE_STORE_TEXT = "#F3F8FB"
        const val PROJECT_SPACE_STORE_MUTED = "#ADCDDCE4"
        const val PROJECT_SPACE_STORE_DIVIDER = "#3397AECC"
    }

    private data class MetricButtonViews(
        val icon: ImageView,
        val value: TextView
    )
}

internal data class ProjectSpaceFeedPost(
    val channel: ProjectChannel,
    val message: ProjectChannelMessage,
    val replyCount: Int
) {
    fun metricKey(): String {
        val messageId = message.id.trim()
        if (messageId.isNotBlank()) return "post:$messageId"
        return "post:${channel.id}:${message.createdAt}:${message.content.hashCode()}"
    }
}

private fun extractProjectPostImageSource(text: String): String? {
    val markdown = Regex("""!\[[^]]*]\(([^)]+)\)""").find(text)
        ?.groupValues
        ?.getOrNull(1)
        ?.trim()
    if (!markdown.isNullOrBlank()) return markdown
    return Regex(
        """https?://\S+\.(?:png|jpe?g|webp|gif)(?:\?\S*)?""",
        RegexOption.IGNORE_CASE
    ).find(text)?.value
}

private fun postBodyWithoutImages(text: String): String {
    return text
        .replace(Regex("""!\[[^]]*]\(([^)]+)\)"""), "")
        .replace(
            Regex("""https?://\S+\.(?:png|jpe?g|webp|gif)(?:\?\S*)?""", RegexOption.IGNORE_CASE),
            ""
        )
        .lines()
        .joinToString("\n") { it.trimEnd() }
        .trim()
}
