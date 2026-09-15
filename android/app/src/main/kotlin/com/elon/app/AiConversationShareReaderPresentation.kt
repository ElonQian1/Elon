package com.elon.app

import android.view.View
import android.widget.TextView
import androidx.core.content.ContextCompat
import java.io.File

internal object AiConversationShareReaderPresentation {
    fun messages(snapshot: AiConversationShareSnapshot, pendingImages: Boolean = false): List<ChatMessage> =
        snapshot.messages.flatMapIndexed { index, source ->
            val message = source.copyForSharing()
            val user = message.role == "user"
            val row = message.copy(
                role = if (user) "user" else "friend",
                id = message.id ?: "${snapshot.card.id}:$index",
                senderLabel = message.senderLabel?.takeIf(String::isNotBlank)
                    ?: if (user) snapshot.card.senderName else snapshot.card.provider,
                sendStatus = null, apkUrl = null, modelUsed = null, nodeId = null,
                projectPostCard = null, evidenceWorking = false, canResolveSuggestion = false,
                webChatMessage = (message.webChatMessage ?: WebChatProductionMessage(
                    snapshot.card.provider, message.id ?: "$index", emptySet(), !user,
                )).let { metadata ->
                    metadata.copy(actions = emptySet(), contentParts = metadata.contentParts.map { part ->
                        part.copy(assetHandle = if (pendingImages) part.assetHandle else null, imageOriginal = null,
                            previewPending = pendingImages && part.previewPending,
                            imageSource = localImageSource(part.imageSource))
                    })
                },
                attachments = message.attachments?.map { attachment ->
                    attachment.copy(url = null, localPath = localImageSource(attachment.localPath))
                },
            )
            if (index in snapshot.gaps) listOf(ChatMessage("ai-conversation-share-gap", "",
                id = "gap:${row.id}", createdAtMs = 0), row) else listOf(row)
        }

    fun bind(
        holder: ChatAdapter.VH,
        message: ChatMessage,
        onOpen: ((ChatMessage, WebChatProductionContentPart) -> Unit)?,
    ) {
        if (message.role == "ai-conversation-share-gap") {
            holder.text.setText(R.string.ai_conversation_share_gap)
            return
        }
        val recalled = message.isRecalled()
        val images = if (recalled) emptyList() else message.attachments.orEmpty().filter {
            it.isImage() && localImageSource(it.localPath) != null
        }
        bindChatAttachmentViews(holder.attachmentList, images, message.role == "user")
        images.forEachIndexed { index, attachment ->
            holder.attachmentList?.getChildAt(index)?.setOnClickListener {
                onOpen?.invoke(message, WebChatProductionContentPart("image",
                    attachment.displayName.orEmpty(), imageSource = attachment.localPath,
                    imageWidth = attachment.imageWidth, imageHeight = attachment.imageHeight))
            }
        }
        message.attachments.orEmpty().filterNot { it in images }.takeIf { !recalled }?.forEach { attachment ->
            holder.attachmentList?.apply {
                visibility = View.VISIBLE
                addView(TextView(context).apply {
                    text = attachment.transcription ?: attachment.displayName ?: attachment.fileName
                        ?: context.getString(R.string.ai_conversation_share_attachment)
                    textSize = 13f
                    setTextColor(ContextCompat.getColor(context, R.color.elon_text_secondary))
                })
            }
        }
        applyChatProjectBubbleStyle(holder.bubble, message.role, false)
        val richText = message.copy(role = "friend", webChatMessage = (message.webChatMessage
            ?: WebChatProductionMessage("snapshot", message.id.orEmpty(), emptySet())).copy(renderMarkdown = true))
        if (recalled || !WebChatProductionRichContentBinder.bindMessageText(holder.text, richText)) {
            holder.text.text = if (recalled) message.copy(role = "friend").recallNoticeText() else message.content
        }
        holder.text.setTextColor(ContextCompat.getColor(holder.text.context,
            if (message.role == "user") R.color.elon_button_primary_text else R.color.elon_text_primary))
        holder.text.visibility = if (message.content.isBlank() && !recalled) View.GONE else View.VISIBLE
        AiConversationShareReaderLinks.bind(holder.text)
        holder.text.setOnClickListener(null)
        holder.text.setOnLongClickListener(null)
        WebChatProductionRichContentBinder.bindParts(holder.webChatPartList,
            if (recalled) message.copy(webChatMessage = null) else message, onOpen)
        // Shared question bubbles belong to the sharer, never the currently signed-in viewer.
        bindSenderAvatar(holder.userAvatar ?: holder.friendAvatar, message)
        listOf(holder.status, holder.selectionCheck, holder.modelAttribution, holder.pauseButton,
            holder.apkActionBar, holder.evidenceSummary, holder.evidenceDetails, holder.evidenceLastEntry,
            holder.finalReplyLabel, holder.itemView.findViewById<View>(R.id.webChatMessageActionBar))
            .forEach { it?.visibility = View.GONE }
        holder.itemView.setOnClickListener(null)
        holder.itemView.setOnLongClickListener(null)
        holder.bubble?.setOnClickListener(null)
        holder.bubble?.setOnLongClickListener(null)
    }

    fun localImageSource(source: String?): String? = source?.takeIf {
        !it.contains("://") && File(it).isAbsolute && File(it).isFile
    }
}
