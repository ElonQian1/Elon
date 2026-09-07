package com.elon.app

internal fun updatePendingAttachmentUploadChoice(
    attachments: MutableList<PendingAttachment>,
    expected: PendingAttachment,
    uploadCopy: Boolean,
): Boolean {
    // The menu may outlive removal, editing, or submission of its original file.
    val index = attachments.indexOfFirst { it === expected }
    if (index < 0 || expected.chatGptUploadCopy == uploadCopy) return false
    attachments[index] = expected.copy(chatGptUploadCopy = uploadCopy)
    return true
}
