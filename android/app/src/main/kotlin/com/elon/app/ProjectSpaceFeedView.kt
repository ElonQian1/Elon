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
    private val openAnnouncementEditor: (ProjectChannel, String) -> Unit,
    private val openProjectDocuments: () -> Unit,
    private val openProjectResources: () -> Unit,
    private val openProjectMembers: () -> Unit,
    private val projectApkActionLabel: () -> String,
    private val downloadProjectApk: () -> Unit
) {
    private val metricPrefs = activity.getSharedPreferences(POST_METRIC_PREFS, Context.MODE_PRIVATE)

    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>,
        loading: Boolean
    ) {
        val posts = feedPosts(space, messagesByChannel)
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#000000"))
            setPadding(0, dp(PROJECT_SPACE_CONTENT_TOP_DP), 0, dp(40))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }

        root.addView(projectHero(space, posts.size))
        root.addView(projectQuickActions())
        root.addView(projectPreviewStrip())
        root.addView(projectPinnedBar(space, messagesByChannel, posts.size))
        root.addView(projectFeedPanel(posts, loading))
        container.addView(root)
    }

    private fun projectHero(space: ProjectSpace, postCount: Int): LinearLayout {
        val owner = space.members.firstOrNull { it.role.equals("owner", ignoreCase = true) }
            ?: space.members.firstOrNull()
        val ownerName = owner?.account?.takeIf { it.isNotBlank() } ?: "项目成员"
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(38), 0, dp(38), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(58)
            )

            addView(projectIcon(space.project), LinearLayout.LayoutParams(dp(45), dp(45)).apply {
                marginEnd = dp(18)
            })

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    text = space.project.name.ifBlank { "项目名称" }
                    textSize = 17f
                    setTextColor(Color.parseColor("#B8B8B8"))
                    setTypeface(typeface, Typeface.NORMAL)
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = "创建者： $ownerName"
                    textSize = 15f
                    setTextColor(Color.parseColor("#777777"))
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    setPadding(0, dp(10), 0, 0)
                })
                addView(TextView(activity).apply {
                    text = "成员 ${space.project.memberCount.coerceAtLeast(1)}    帖子 $postCount"
                    textSize = 15f
                    setTextColor(Color.parseColor("#777777"))
                    includeFontPadding = false
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    setPadding(0, dp(8), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))

            addView(TextView(activity).apply {
                text = "加入"
                textSize = 16f
                setTypeface(typeface, Typeface.BOLD)
                includeFontPadding = false
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#000000"))
                background = roundedBackground("#D9D9D9", 16)
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { openProjectMembers() }
                contentDescription = "查看项目成员"
            }, LinearLayout.LayoutParams(dp(68), dp(30)).apply {
                marginStart = dp(18)
            })
        }
    }

    private fun projectIcon(project: ProjectSpaceSummary): View {
        val bitmap = UserProfileStore.decodeAvatar(project.iconDataUrl.cleanProjectSpaceDisplayName())
        if (bitmap != null) {
            return ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(6).toFloat()
                    setAntiAlias(true)
                })
            }
        }
        return View(activity).apply {
            background = roundedBackground("#FFFFFF", 6)
        }
    }

    private fun projectQuickActions(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(34), 0, dp(34), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            ).apply {
                topMargin = dp(24)
            }
            addView(quickAction(
                iconRes = R.drawable.ic_project_space_folder,
                label = "项目文档",
                description = "打开项目文档",
                onClick = openProjectDocuments
            ), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(quickAction(
                iconRes = R.drawable.ic_project_space_folder,
                label = "项目资源",
                description = "查看项目资源",
                onClick = openProjectResources
            ), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
            addView(quickAction(
                iconRes = R.drawable.ic_project_space_download,
                label = projectApkActionLabel(),
                description = "下载项目 APK",
                onClick = downloadProjectApk
            ), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        }
    }

    private fun quickAction(
        iconRes: Int,
        label: String,
        description: String,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            isClickable = true
            foreground = selectableForeground()
            contentDescription = description
            setOnClickListener { onClick() }
            addView(ImageView(activity).apply {
                setImageResource(iconRes)
                setColorFilter(Color.parseColor("#D9D9D9"))
            }, LinearLayout.LayoutParams(dp(32), dp(32)).apply {
                marginEnd = dp(8)
            })
            addView(TextView(activity).apply {
                text = label
                textSize = 15f
                includeFontPadding = false
                setTextColor(Color.parseColor("#B8B8B8"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
        }
    }

    private fun projectPreviewStrip(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(18), 0, dp(18), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(167)
            ).apply {
                topMargin = dp(18)
            }
            repeat(3) { index ->
                addView(View(activity).apply {
                    background = roundedBackground("#7B7B7B", 9)
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f).apply {
                    if (index < 2) marginEnd = dp(16)
                })
            }
        }
    }

    private fun projectPinnedBar(
        space: ProjectSpace,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>,
        postCount: Int
    ): LinearLayout {
        val announcement = space.channels.firstOrNull { it.kind == "announcements" }
        val pinnedText = latestAnnouncementText(announcement, messagesByChannel)
            .replace(Regex("""\s+"""), " ")
            .take(16)
            .ifBlank { "创建者自定义标题最多显示16个字" }
        val editable = announcement != null && canEditProjectAnnouncement(space.project.role)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(14), 0, dp(16), 0)
            background = roundedBackground("#7D7D7D", 14)
            if (editable) {
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener {
                    announcement?.let { openAnnouncementEditor(it, latestAnnouncementText(it, messagesByChannel)) }
                }
                contentDescription = "编辑置顶公告"
            }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(43)
            ).apply {
                setMargins(dp(20), dp(26), dp(20), 0)
            }

            addView(TextView(activity).apply {
                text = "置顶"
                textSize = 15f
                includeFontPadding = false
                setTextColor(Color.parseColor("#D9D9D9"))
            })
            addView(TextView(activity).apply {
                text = pinnedText
                textSize = 15f
                includeFontPadding = false
                setTextColor(Color.parseColor("#D9D9D9"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                marginStart = dp(14)
                marginEnd = dp(12)
            })
            addView(TextView(activity).apply {
                text = "${postCount.coerceAtLeast(0)}贴"
                textSize = 13f
                includeFontPadding = false
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#D9D9D9"))
                background = roundedBackground("#252525", 9)
            }, LinearLayout.LayoutParams(dp(44), dp(18)))
        }
    }

    private fun latestAnnouncementText(
        announcement: ProjectChannel?,
        messagesByChannel: Map<String, List<ProjectChannelMessage>>
    ): String {
        val latest = announcement?.let { channel ->
            messagesByChannel[channel.id]
                .orEmpty()
                .maxByOrNull { parseChatMessageCreatedAt(it.createdAt) ?: 0L }
        }
        val textValue = cleanAnnouncementText(latest?.content)
            ?: cleanAnnouncementText(announcement?.lastMessage)
            ?: "不得发布与主题内容不相关的帖子。"
        return parseProjectSpacePostText(textValue).detailText
            .takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
            ?: "不得发布与主题内容不相关的帖子。"
    }

    private fun cleanAnnouncementText(value: String?): String? {
        return value?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun projectFeedPanel(posts: List<ProjectSpaceFeedPost>, loading: Boolean): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(15), 0, dp(86))
            minimumHeight = dp(460)
            background = roundedBackground(
                colorHex = "#000000",
                topStartDp = 20,
                topEndDp = 20,
                bottomEndDp = 0,
                bottomStartDp = 0
            )
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(21)
            }

            addView(topicChips())
            when {
                posts.isNotEmpty() -> posts.forEachIndexed { index, post ->
                    val card = postCard(post)
                    val params = card.layoutParams as LinearLayout.LayoutParams
                    params.topMargin = if (index == 0) dp(14) else dp(10)
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
            setPadding(dp(34), 0, dp(24), 0)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(24)
            )
            topics.forEach { topic ->
                addView(TextView(activity).apply {
                    text = topic
                    textSize = 14f
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setTextColor(Color.parseColor("#D9D9D9"))
                    background = roundedBackground("#777777", 4)
                    setPadding(dp(7), 0, dp(7), 0)
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    dp(18)
                ).apply {
                    marginEnd = if (topic == "问题反馈") 0 else dp(29)
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

    private fun postCard(post: ProjectSpaceFeedPost): LinearLayout {
        val postText = parseProjectSpacePostText(post.message.content)
        val sender = post.message.senderName.cleanProjectSpaceDisplayName() ?: "项目成员"
        val timeText = parseChatMessageCreatedAt(post.message.createdAt)
            ?.let { formatChatTimelineLabel(it) }
            ?: "刚刚"
        val bodyText = postBodyWithoutImages(postText.body).ifBlank { postText.title }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(20), dp(24), dp(15))
            background = roundedBackground("#1F2023", 15)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPost(post.channel, post.message) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                setMargins(dp(10), dp(14), dp(10), 0)
            }

            addView(postHeader(sender, post.message.senderAvatarDataUrl, timeText, projectSpaceTopicLabel(post.channel)))
            addView(TextView(activity).apply {
                text = postText.title
                textSize = 16f
                setTextColor(Color.parseColor("#D9D9D9"))
                setLineSpacing(dp(4).toFloat(), 1f)
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(22), 0, 0)
            })
            addView(TextView(activity).apply {
                text = bodyText
                textSize = 16f
                setTextColor(Color.parseColor("#8E8E8E"))
                setLineSpacing(dp(4).toFloat(), 1f)
                maxLines = 3
                ellipsize = TextUtils.TruncateAt.END
                setPadding(0, dp(12), 0, 0)
            })
            extractProjectPostImageSource(postText.body)?.let { source ->
                addView(postImagePreview(source))
            }
            addView(postMetrics(post, postText))
        }
    }

    private fun postHeader(sender: String, avatarDataUrl: String?, timeText: String, topic: String): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(avatar(sender, avatarDataUrl), LinearLayout.LayoutParams(dp(51), dp(51)).apply {
                marginEnd = dp(18)
            })
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    text = sender
                    textSize = 17f
                    includeFontPadding = false
                    setTextColor(Color.parseColor("#D9D9D9"))
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                })
                addView(TextView(activity).apply {
                    text = "回复于$timeText"
                    textSize = 13f
                    includeFontPadding = false
                    setTextColor(Color.parseColor("#AFAFAF"))
                    setPadding(0, dp(9), 0, 0)
                })
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                text = topic
                textSize = 14f
                includeFontPadding = false
                setTextColor(Color.parseColor("#D9D9D9"))
                gravity = Gravity.CENTER
                maxLines = 1
                background = roundedBackground("#000000", 6)
                setPadding(dp(8), 0, dp(8), 0)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                dp(20)
            ))
        }
    }

    private fun avatar(sender: String, avatarDataUrl: String?): View {
        val bitmap = UserProfileStore.decodeAvatar(avatarDataUrl.cleanProjectSpaceDisplayName())
        if (bitmap != null) {
            return ImageView(activity).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageDrawable(RoundedBitmapDrawableFactory.create(resources, bitmap).apply {
                    cornerRadius = dp(26).toFloat()
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
            background = roundedBackground("#D8D8D8", 26)
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
                dp(111)
            ).apply {
                topMargin = dp(16)
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
            setPadding(dp(52), dp(21), dp(52), 0)
            addView(metricButton(
                iconRes = R.drawable.ic_project_post_share,
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
                iconRes = R.drawable.ic_project_post_chat,
                value = post.replyCount.coerceAtLeast(0).toString(),
                description = "查看${post.replyCount.coerceAtLeast(0)}条讨论",
                onClick = { openPost(post.channel, post.message) }
            ), metricParams())
            addView(metricButton(
                iconRes = if (liked) R.drawable.ic_project_post_like_filled else R.drawable.ic_project_post_like,
                value = likeCount.toString(),
                description = if (liked) "取消点赞" else "点赞",
                selected = liked,
                onClick = { views ->
                    val nextLiked = !metricPrefs.getBoolean("$key:liked", false)
                    metricPrefs.edit().putBoolean("$key:liked", nextLiked).apply()
                    updateMetricButton(
                        views = views,
                        iconRes = if (nextLiked) R.drawable.ic_project_post_like_filled else R.drawable.ic_project_post_like,
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
            textSize = 16f
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
            minimumHeight = dp(34)
            setPadding(dp(4), dp(5), dp(4), dp(5))
            addView(icon, LinearLayout.LayoutParams(dp(24), dp(24)).apply {
                marginEnd = dp(7)
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
        return Color.parseColor(if (selected) "#58BE6A" else "#AFAFAF")
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
                    setTextColor(Color.parseColor("#D9D9D9"))
                    background = roundedBackground("#212121", 24)
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

    private fun roundedBackground(
        colorHex: String,
        topStartDp: Int,
        topEndDp: Int,
        bottomEndDp: Int,
        bottomStartDp: Int
    ): GradientDrawable {
        val topStart = dp(topStartDp).toFloat()
        val topEnd = dp(topEndDp).toFloat()
        val bottomEnd = dp(bottomEndDp).toFloat()
        val bottomStart = dp(bottomStartDp).toFloat()
        return GradientDrawable().apply {
            setColor(Color.parseColor(colorHex))
            cornerRadii = floatArrayOf(
                topStart, topStart,
                topEnd, topEnd,
                bottomEnd, bottomEnd,
                bottomStart, bottomStart
            )
        }
    }

    private companion object {
        const val MAX_FEED_POSTS = 40
        const val MAX_IMAGE_PREVIEW_BYTES = 5 * 1024 * 1024
        const val POST_METRIC_PREFS = "project_post_metrics"
        const val PROJECT_SPACE_CONTENT_TOP_DP = 42
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
