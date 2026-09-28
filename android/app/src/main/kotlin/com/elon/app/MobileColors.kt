package com.elon.app

import android.content.Context

/** Resolved Material roles for programmatic Views. Recreate with the themed context. */
internal class MobileColors(context: Context) {
    val surface = context.elonColor(R.color.mobile_surface)
    val container = context.elonColor(R.color.mobile_surface_container)
    val elevated = context.elonColor(R.color.mobile_surface_container_high)
    val text = context.elonColor(R.color.mobile_on_surface)
    val muted = context.elonColor(R.color.mobile_on_surface_variant)
    val primary = context.elonColor(R.color.mobile_primary)
    val onPrimary = context.elonColor(R.color.mobile_on_primary)
    val primaryContainer = context.elonColor(R.color.mobile_primary_container)
    val onPrimaryContainer = context.elonColor(R.color.mobile_on_primary_container)
    val outline = context.elonColor(R.color.mobile_outline)
    val divider = context.elonColor(R.color.mobile_outline_variant)
    val error = context.elonColor(R.color.mobile_error)
    val errorContainer = context.elonColor(R.color.mobile_error_container)
    val success = context.elonColor(R.color.mobile_success)
    val successContainer = context.elonColor(R.color.mobile_success_container)
    val warning = context.elonColor(R.color.mobile_warning)
    val warningContainer = context.elonColor(R.color.mobile_warning_container)
}
