package com.elon.app

import android.content.Context
import android.net.Uri
import androidx.core.content.FileProvider
import java.io.File
import java.lang.reflect.Proxy
import org.junit.Assert.assertTrue
import org.robolectric.shadows.ShadowContentResolver

/** AndroidX hardcodes '/' for filesystem descendants; the Windows JVM uses '\\'. */
internal fun installWindowsExportFileProvider(context: Context) {
    if (File.separatorChar != '\\') return
    val authority = "${context.packageName}.fileprovider"
    val provider = ShadowContentResolver.getProvider(Uri.parse("content://$authority/")) as FileProvider
    var configured = false
    context.resources.getXml(R.xml.file_paths).use { xml ->
        while (xml.eventType != org.xmlpull.v1.XmlPullParser.END_DOCUMENT) {
            if (xml.eventType == org.xmlpull.v1.XmlPullParser.START_TAG && xml.name == "cache-path" &&
                xml.getAttributeValue(null, "name") == "message_exports" &&
                xml.getAttributeValue(null, "path") == "message_exports/") configured = true
            xml.next()
        }
    }
    assertTrue("production FileProvider export root missing", configured)
    val root = File(context.cacheDir, "message_exports").canonicalFile
    val strategyClass = Class.forName("androidx.core.content.FileProvider\$PathStrategy")
    val strategy = Proxy.newProxyInstance(strategyClass.classLoader, arrayOf(strategyClass)) { _, method, args ->
        when (method.name) {
            "getUriForFile" -> {
                val file = (args!![0] as File).canonicalFile
                require(file.parentFile == root)
                Uri.Builder().scheme("content").authority(authority).appendPath("message_exports").appendPath(file.name).build()
            }
            "getFileForUri" -> {
                val uri = args!![0] as Uri
                require(uri.authority == authority && uri.pathSegments.size == 2 && uri.pathSegments.first() == "message_exports")
                File(root, uri.lastPathSegment!!).canonicalFile.also { require(it.parentFile == root) }
            }
            else -> error("Unexpected FileProvider strategy method")
        }
    }
    FileProvider::class.java.getDeclaredField("sCache").apply { isAccessible = true }.let {
        @Suppress("UNCHECKED_CAST")
        (it.get(null) as MutableMap<String, Any>)[authority] = strategy
    }
    FileProvider::class.java.declaredFields.first { it.type == strategyClass }.apply {
        isAccessible = true; set(provider, strategy)
    }
}
