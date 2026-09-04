package com.elon.app

import android.content.ComponentName
import android.content.Intent
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

/** Public, credential-free launch only. A future account handoff needs a separate protocol. */
internal fun openOfficialQuantApp(activity: AppCompatActivity): Boolean {
    val manager = activity.packageManager
    val installed = readInstalledPackageInfo(manager, OfficialQuantApkPolicy.PACKAGE_NAME) ?: return false
    val trusted = OfficialQuantApkPolicy.accepts(
        installed.packageName,
        currentPackageSignerSha256(installed),
        installed.projectApkVersionCode(),
    )
    if (!trusted) {
        Toast.makeText(
            activity,
            "无法确认官方量化应用，请从一龙项目广场安装或更新官方版本。",
            Toast.LENGTH_LONG,
        ).show()
        return false
    }
    val component = ComponentName(OfficialQuantApkPolicy.PACKAGE_NAME, OfficialQuantApkPolicy.ACTIVITY_NAME)
    val target = runCatching {
        @Suppress("DEPRECATION")
        manager.getActivityInfo(component, 0)
    }.getOrNull()
    if (target == null || !target.enabled || !target.exported || !target.applicationInfo.enabled ||
        target.packageName != component.packageName || target.name != component.className ||
        target.targetActivity != null
    ) {
        Toast.makeText(activity, "官方量化入口不可用，请更新应用后重试。", Toast.LENGTH_LONG).show()
        return false
    }
    // Do not reuse a Launcher Intent or add tokens, URIs, balances, or account identifiers.
    val intent = Intent(Intent.ACTION_MAIN).apply {
        addCategory(Intent.CATEGORY_LAUNCHER)
        setComponent(component)
        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    return runCatching {
        activity.startActivity(intent)
        true
    }.getOrElse {
        Toast.makeText(activity, "无法打开官方量化应用，请稍后重试。", Toast.LENGTH_LONG).show()
        false
    }
}
