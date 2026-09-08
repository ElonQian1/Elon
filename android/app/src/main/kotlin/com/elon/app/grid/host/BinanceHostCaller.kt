package com.elon.app.grid.host

import android.app.Activity
import android.content.Context
import android.os.Binder
import com.elon.app.OfficialQuantApkPolicy
import com.elon.app.currentPackageSignerSha256
import com.elon.app.projectApkVersionCode
import com.elon.app.readInstalledPackageInfo

internal object BinanceHostCaller {
    const val ACTIVITY = "com.elon.quant.grids.host.HostedGridActivity"
    fun trusted(context: Context, uid: Int): Boolean = runCatching {
        val names = context.packageManager.getPackagesForUid(uid)?.toSet() ?: return false
        if (names != setOf(OfficialQuantApkPolicy.PACKAGE_NAME)) return false
        val info = readInstalledPackageInfo(context.packageManager, names.single()) ?: return false
        OfficialQuantApkPolicy.accepts(info.packageName, currentPackageSignerSha256(info), info.projectApkVersionCode()) &&
            info.applicationInfo?.enabled == true
    }.getOrDefault(false)
    fun activity(activity: Activity): Boolean = runCatching {
        val caller = activity.callingActivity ?: return false
        require(activity.packageName == "com.elon.app" && activity.callingPackage == OfficialQuantApkPolicy.PACKAGE_NAME)
        require(caller.packageName == activity.callingPackage && caller.className == ACTIVITY)
        val info = activity.packageManager.getApplicationInfo(caller.packageName, 0)
        trusted(activity, info.uid)
    }.getOrDefault(false)
    fun ipc(context: Context) = trusted(context, Binder.getCallingUid())
}
