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
    private val attachPickedFile: (String, Uri, String?) -> Unit
) {
    private lateinit var cameraAttachmentLauncher: ActivityResultLauncher<Uri>
    private lateinit var photoAttachmentLauncher: ActivityResultLauncher<PickVisualMediaRequest>
    private lateinit var documentAttachmentLauncher: ActivityResultLauncher<Array<String>>
    private var pendingCameraUri: Uri? = null
    private var pendingCameraName: String? = null

    fun setupAttachmentLaunchers() {
        cameraAttachmentLauncher = activity.registerForActivityResult(ActivityResultContracts.TakePicture()) { success ->
            val uri = pendingCameraUri
            val name = pendingCameraName
            pendingCameraUri = null
            pendingCameraName = null
            if (success && uri != null) {
                attachPickedFile("相机照片", uri, name)
            } else {
                Toast.makeText(activity, "已取消拍摄", Toast.LENGTH_SHORT).show()
            }
        }
        photoAttachmentLauncher = activity.registerForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
            if (uri != null) {
                attachPickedFile("相册图片", uri, null)
            } else {
                Toast.makeText(activity, "已取消选择相册", Toast.LENGTH_SHORT).show()
            }
        }
        documentAttachmentLauncher = activity.registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri != null) {
                runCatching {
                    activity.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                attachPickedFile("文档", uri, null)
            } else {
                Toast.makeText(activity, "已取消选择文档", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun openCameraAttachment() {
        if (activeConversation().ended) return
        val attachmentDir = File(activity.cacheDir, "attachments").apply { mkdirs() }
        val fileName = "camera_${System.currentTimeMillis()}.jpg"
        val file = File(attachmentDir, fileName)
        val uri = FileProvider.getUriForFile(activity, "com.elon.app.fileprovider", file)
        pendingCameraUri = uri
        pendingCameraName = fileName
        runCatching {
            cameraAttachmentLauncher.launch(uri)
        }.onFailure {
            pendingCameraUri = null
            pendingCameraName = null
            Toast.makeText(activity, "无法打开相机", Toast.LENGTH_SHORT).show()
        }
    }

    fun openPhotoAttachment() {
        if (activeConversation().ended) return
        runCatching {
            photoAttachmentLauncher.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly)
            )
        }.onFailure {
            Toast.makeText(activity, "无法打开相册", Toast.LENGTH_SHORT).show()
        }
    }

    fun openDocumentAttachment() {
        if (activeConversation().ended) return
        runCatching {
            documentAttachmentLauncher.launch(arrayOf("*/*"))
        }.onFailure {
            Toast.makeText(activity, "无法打开文档选择器", Toast.LENGTH_SHORT).show()
        }
    }
}
