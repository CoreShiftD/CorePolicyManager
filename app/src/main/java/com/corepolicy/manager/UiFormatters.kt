package com.corepolicy.manager

import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.concurrent.TimeUnit

fun formatRelativeTime(timestampMs: Long, nowMs: Long = System.currentTimeMillis()): String {
    val delta = (nowMs - timestampMs).coerceAtLeast(0L)
    val min = TimeUnit.MILLISECONDS.toMinutes(delta)
    return when {
        min < 1 -> "just now"
        min < 60 -> "${min}m ago"
        min < 24 * 60 -> "${TimeUnit.MINUTES.toHours(min)}h ago"
        else -> "${TimeUnit.MINUTES.toDays(min)}d ago"
    }
}

fun formatDaemonStateLabel(state: DaemonState, disconnected: Boolean): String = when {
    disconnected -> "Offline"
    state == DaemonState.RUNNING -> "Running"
    state == DaemonState.STOPPED -> "Stopped"
    else -> "Degraded"
}

fun formatModuleHealthLabel(health: ModuleHealth): String = when (health) {
    ModuleHealth.HEALTHY -> "Healthy"
    ModuleHealth.DEGRADED -> "Degraded"
    ModuleHealth.DISABLED -> "Disabled"
    ModuleHealth.CONFLICT -> "Conflict"
}

fun formatDateHeading(timestampMs: Long): String =
    SimpleDateFormat("EEE, MMM d", Locale.US).format(Date(timestampMs))

fun formatDuration(durationMs: Long): String {
    val totalMinutes = TimeUnit.MILLISECONDS.toMinutes(durationMs.coerceAtLeast(0L))
    val days = totalMinutes / (24 * 60)
    val hours = (totalMinutes % (24 * 60)) / 60
    val minutes = totalMinutes % 60
    return when {
        days > 0 -> "${days}d ${hours}h"
        hours > 0 -> "${hours}h ${minutes}m"
        else -> "${minutes}m"
    }
}
