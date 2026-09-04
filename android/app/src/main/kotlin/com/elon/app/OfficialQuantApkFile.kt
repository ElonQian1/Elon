package com.elon.app

import java.io.File

/** A new private file per attempt prevents parallel downloads from replacing verified bytes. */
internal fun createOfficialQuantApkFile(privateCacheDirectory: File): File {
    val directory = File(privateCacheDirectory, "official-quant-apk")
    check(directory.isDirectory || directory.mkdirs()) { "无法准备官方量化安装目录" }
    return File.createTempFile("quant-", ".apk", directory)
}
