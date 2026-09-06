package com.elon.app

import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import java.io.File

internal class MainAttachmentPickerActions(
    private val activity: AppCompatActivity,
    private val activeConversation: () -> AppConversation,
    private val attachPickedFile: (String, Uri, String?) -> List<PendingAttachment>,
    private val attachPickedImages: (String, List<Uri>, List<String?>) -> List<PendingAttachment>,
    private val preparationPort: () -> WebChatAttachmentPreparationPort? = { null },
) {
    private lateinit var cameraAttachmentLauncher: ActivityResultLauncher<Uri>
    private lateinit var photoAttachmentLauncher: ActivityResultLauncher<PickVisualMediaRequest>
    private lateinit var documentAttachmentLauncher: ActivityResultLauncher<Array<String>>
    private var pendingCameraUri: Uri? = null
    private var pendingCameraName: String? = null
    private var selection: WebChatAttachmentSelection? = null
    private var pickerOpen = false

    private fun beginSelection(kind: WebChatAttachmentSelectionKind): Boolean {
        if (pickerOpen) return false
        pickerOpen = true
        selection = runCatching { preparationPort()?.begin(kind) }.getOrNull()
        return true
    }

    private fun finishSelection(files: List<PendingAttachment> = emptyList()) {
        val current = selection
        selection = null
        pickerOpen = false
        if (files.size == 1) current?.selected(files) else current?.cancel()
    }

    fun setupAttachmentLaunchers() {
        cameraAttachmentLauncher = activity.registerForActivityResult(ActivityResultContracts.TakePicture()) { success ->
            val uri = pendingCameraUri
            val name = pendingCameraName
            pendingCameraUri = null
            pendingCameraName = null
            if (success && uri != null) {
                finishSelection(attachPickedImages("相机照片", listOf(uri), listOf(name)))
            } else {
                finishSelection()
                Toast.makeText(activity, "已取消拍摄", Toast.LENGTH_SHORT).show()
            }
        }
        photoAttachmentLauncher = activity.registerForActivityResult(
            ActivityResultContracts.PickMultipleVisualMedia(MAX_PENDING_ATTACHMENTS)
        ) { uris ->
            if (uris.isNotEmpty()) {
                val files = attachPickedImages("相册图片", uris, emptyList())
                finishSelection(if (uris.size == 1) files else emptyList())
            } else {
                finishSelection()
                Toast.makeText(activity, "已取消选择相册", Toast.LENGTH_SHORT).show()
            }
        }
        documentAttachmentLauncher = activity.registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri != null) {
                runCatching {
                    activity.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                finishSelection(attachPickedFile("文档", uri, null))
            } else {
                finishSelection()
                Toast.makeText(activity, "已取消选择文档", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun openCameraAttachment() {
        if (activeConversation().ended) return
        if (!beginSelection(WebChatAttachmentSelectionKind.IMAGE)) return
        runCatching {
            val attachmentDir = File(activity.cacheDir, "attachments").apply { mkdirs() }
            val fileName = "camera_${System.currentTimeMillis()}.jpg"
            val file = File(attachmentDir, fileName)
            val uri = FileProvider.getUriForFile(activity, "com.elon.app.fileprovider", file)
            pendingCameraUri = uri
            pendingCameraName = fileName
            cameraAttachmentLauncher.launch(uri)
        }.onFailure {
            finishSelection()
            pendingCameraUri = null
            pendingCameraName = null
            Toast.makeText(activity, "无法打开相机", Toast.LENGTH_SHORT).show()
        }
    }

    fun openPhotoAttachment() {
        if (activeConversation().ended) return
        if (!beginSelection(WebChatAttachmentSelectionKind.IMAGE)) return
        runCatching {
            photoAttachmentLauncher.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)
            )
        }.onFailure {
            finishSelection()
            Toast.makeText(activity, "无法打开相册", Toast.LENGTH_SHORT).show()
        }
    }

    fun openDocumentAttachment() {
        if (activeConversation().ended) return
        if (!beginSelection(WebChatAttachmentSelectionKind.DOCUMENT)) return
        runCatching {
            documentAttachmentLauncher.launch(arrayOf("*/*"))
        }.onFailure {
            finishSelection()
            Toast.makeText(activity, "无法打开文档选择器", Toast.LENGTH_SHORT).show()
        }
    }
}
