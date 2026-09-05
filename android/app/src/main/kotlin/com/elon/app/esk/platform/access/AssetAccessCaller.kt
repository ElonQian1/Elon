package com.elon.app.esk.platform.access

import android.app.Activity
import android.content.pm.PackageManager
import com.elon.app.OfficialQuantApkPolicy
import com.elon.app.currentPackageSignerSha256
import com.elon.app.projectApkVersionCode
import com.elon.app.readInstalledPackageInfo

internal fun Activity.hasOfficialAssetAccessCaller(): Boolean = runCatching {
    val caller = callingActivity ?: return false
    if (packageName != "com.elon.app" || callingPackage != "com.elon.quant" ||
        caller.packageName != callingPackage ||
        caller.className != "com.elon.quant.assets.access.AssetAccessActivity") return false
    val installed = readInstalledPackageInfo(packageManager, caller.packageName) ?: return false
    @Suppress("DEPRECATION")
    val activity = packageManager.getActivityInfo(caller, PackageManager.MATCH_DISABLED_COMPONENTS)
    val allowed = setOf(PackageManager.COMPONENT_ENABLED_STATE_DEFAULT, PackageManager.COMPONENT_ENABLED_STATE_ENABLED)
    activity.enabled && activity.applicationInfo.enabled && activity.targetActivity == null &&
        activity.name == caller.className && activity.packageName == caller.packageName &&
        packageManager.getComponentEnabledSetting(caller) in allowed &&
        packageManager.getApplicationEnabledSetting(caller.packageName) in allowed &&
        OfficialQuantApkPolicy.accepts(installed.packageName, currentPackageSignerSha256(installed),
            installed.projectApkVersionCode())
}.getOrDefault(false)
