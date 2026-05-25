package com.elon.app

import java.util.Locale

internal fun looksLikeDevelopmentRequest(text: String): Boolean {
    val lower = text.lowercase(Locale.CHINA)
    val directWords = listOf(
        "app", "apk", "android", "应用", "功能", "页面", "界面", "按钮", "代码", "开发",
        "修改", "添加", "新增", "生成", "做一个", "做个", "编译", "打包", "安装", "发布",
        "登录", "注册", "首页", "设置", "接口", "后端", "服务端", "数据库", "继续", "项目"
    )
    if (directWords.any { lower.contains(it) }) return true

    val actionWords = listOf(
        "改", "改成", "修改", "调整", "优化", "美化", "添加", "新增", "增加", "加上",
        "删掉", "删除", "去掉", "替换", "做成", "变成", "接入", "修复", "处理"
    )
    val uiWords = listOf(
        "点击", "屏幕", "中间", "文字", "字体", "动画", "闪烁", "按钮", "菜单",
        "页面", "界面", "弹窗", "提示", "显示", "隐藏", "颜色", "图标", "布局",
        "输入框", "底部", "顶部", "气泡", "回复", "折叠"
    )
    return actionWords.any { lower.contains(it) } && uiWords.any { lower.contains(it) }
}

internal fun looksLikeResumeCommand(normalized: String): Boolean {
    if (normalized in setOf(
            "继续",
            "继续吧",
            "继续开发",
            "继续做",
            "继续完成",
            "重试",
            "再试一次",
            "重新开始",
            "再来一次"
        )
    ) {
        return true
    }
    return (normalized.contains("继续") || normalized.contains("重试") || normalized.contains("再试")) &&
        (normalized.contains("上一次") ||
            normalized.contains("未完成") ||
            normalized.contains("当前项目的开发") ||
            normalized.contains("当前进度"))
}

internal fun looksLikeApkDeliveryRequest(text: String): Boolean {
    val lower = text.lowercase(Locale.CHINA)
    val asksForApk = lower.contains("apk") || lower.contains("安装包") || lower.contains("下载包")
    val asksForDelivery = listOf("地址", "链接", "下载", "发给我", "给我", "做好", "做完", "完成")
        .any { lower.contains(it) }
    return asksForApk && asksForDelivery
}

internal fun looksLikeDirectImageRequest(text: String): Boolean {
    val lower = text.lowercase(Locale.CHINA)
    val appWords = listOf(
        "app", "apk", "android", "应用", "功能", "页面", "界面", "按钮", "代码", "开发",
        "修改", "添加", "新增", "编译", "打包", "安装", "发布", "登录", "注册", "首页",
        "设置", "接口", "后端", "服务端", "数据库", "项目"
    )
    if (appWords.any { lower.contains(it) }) return false

    val imageWords = listOf("文生图", "生图", "生成图", "图像", "图片", "壁纸", "照片", "头像", "插画", "海报", "卡通", "山水画")
    val intentWords = listOf("文生图", "生图", "生成", "画", "绘制", "做一张", "来一张", "出一张", "创作")
    return imageWords.any { lower.contains(it) } && intentWords.any { lower.contains(it) }
}
