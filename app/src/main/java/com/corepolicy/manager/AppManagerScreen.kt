package com.corepolicy.manager

import android.content.pm.PackageManager
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.core.graphics.drawable.toBitmap
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette
import kotlinx.coroutines.launch

enum class AppProfile(val label: String) {
    DEFAULT("Default"),
    BALANCED("Balanced"),
    PERFORMANCE("Performance"),
    BATTERY_SAVER("Battery Saver"),
    CUSTOM("Custom")
}

private enum class AppSortMode { NAME, MANAGED_FIRST }

enum class PolicySyncState { IDLE, PENDING, APPLIED, ERROR }
enum class PreloadMode { OFF, LIGHT, AGGRESSIVE }
enum class ProcessRuleMode { DEFAULT, STRICT, RELAXED }
enum class BatteryPolicyMode { DEFAULT, RESTRICTED, UNRESTRICTED }

data class AppPolicy(
    val packageName: String,
    val appName: String,
    val enabled: Boolean,
    val inheritGlobalDefaults: Boolean,
    val profile: AppProfile,
    val preloadEnabled: Boolean,
    val processControlEnabled: Boolean,
    val batteryPolicyEnabled: Boolean,
    val preloadMode: PreloadMode,
    val processRuleMode: ProcessRuleMode,
    val batteryPolicyMode: BatteryPolicyMode,
    val syncState: PolicySyncState,
    val lastAppliedAt: Long?
)

data class AppEntry(
    val packageName: String,
    val appName: String,
    val icon: ImageBitmap?
)

private fun AppPolicy.summaryLabel(): String = when {
    !enabled -> "Policy disabled"
    inheritGlobalDefaults -> "Managed with global defaults"
    else -> "Managed with per-app overrides"
}

private fun defaultAppPolicy(app: AppEntry): AppPolicy = AppPolicy(
    packageName = app.packageName,
    appName = app.appName,
    enabled = false,
    inheritGlobalDefaults = true,
    profile = AppProfile.DEFAULT,
    preloadEnabled = false,
    processControlEnabled = false,
    batteryPolicyEnabled = false,
    preloadMode = PreloadMode.OFF,
    processRuleMode = ProcessRuleMode.DEFAULT,
    batteryPolicyMode = BatteryPolicyMode.DEFAULT,
    syncState = PolicySyncState.IDLE,
    lastAppliedAt = null
)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun AppManagerScreen(
    modifier: Modifier = Modifier,
    daemonPolicyService: DaemonPolicyService
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val repository = remember(context) { AppPolicyRepository(context) }
    val appEntries = remember { mutableStateListOf<AppEntry>() }
    val policies = remember { mutableStateMapOf<String, AppPolicy>() }
    val iconCache = remember { mutableStateMapOf<String, ImageBitmap?>() }
    val persistedPolicies by repository.loadPolicies().collectAsState(initial = emptyMap())
    var query by remember { mutableStateOf("") }
    var showManagedOnly by remember { mutableStateOf(false) }
    var sortMode by remember { mutableStateOf(AppSortMode.NAME) }
    var selectedPackage by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        val pm = context.packageManager
        val installed = pm.getInstalledApplications(PackageManager.GET_META_DATA)
            .filter { pm.getLaunchIntentForPackage(it.packageName) != null }
            .sortedBy { pm.getApplicationLabel(it).toString() }
            .map { info ->
                val pkg = info.packageName
                AppEntry(
                    packageName = pkg,
                    appName = pm.getApplicationLabel(info).toString(),
                    icon = iconCache[pkg] ?: runCatching {
                        pm.getApplicationIcon(info).toBitmap(48, 48).asImageBitmap()
                    }.getOrNull().also { iconCache[pkg] = it }
                )
            }
        appEntries.clear()
        appEntries.addAll(installed)
        installed.forEach { app ->
            policies.putIfAbsent(app.packageName, persistedPolicies[app.packageName] ?: defaultAppPolicy(app))
        }
    }

    LaunchedEffect(persistedPolicies) {
        persistedPolicies.forEach { (pkg, policy) -> policies[pkg] = policy }
    }

    val filteredApps = appEntries
        .filter {
            query.isBlank() ||
                it.appName.contains(query, ignoreCase = true) ||
                it.packageName.contains(query, ignoreCase = true)
        }
        .filter { !showManagedOnly || policies[it.packageName]?.enabled == true }
        .let { list ->
            when (sortMode) {
                AppSortMode.NAME -> list.sortedBy { it.appName.lowercase() }
                AppSortMode.MANAGED_FIRST -> list.sortedWith(
                    compareByDescending<AppEntry> { policies[it.packageName]?.enabled == true }
                        .thenBy { it.appName.lowercase() }
                )
            }
        }

    val managedCount = policies.values.count { it.enabled }
    val applyPolicyUpdate: (String, AppPolicy) -> Unit = { pkg, policy ->
        policies[pkg] = policy
        scope.launch {
            val pending = policy.copy(syncState = PolicySyncState.PENDING)
            policies[pkg] = pending
            repository.savePolicy(pending)
            daemonPolicyService.applyAppPolicy(pending)
            val applied = pending.copy(syncState = PolicySyncState.APPLIED, lastAppliedAt = System.currentTimeMillis())
            policies[pkg] = applied
            repository.savePolicy(applied)
        }
    }

    LazyColumn(
        modifier = modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(top = CorePolicyDesign.spacing.sm),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.lg)
    ) {
        item {
            AppManagerHeader(
                managedCount = managedCount,
                installedCount = appEntries.size,
                filteredCount = filteredApps.size
            )
        }
        item {
            AppManagerToolbar(
                filteredCount = filteredApps.size,
                query = query,
                showManagedOnly = showManagedOnly,
                sortMode = sortMode,
                onQueryChange = { query = it },
                onManagedOnlyChanged = { showManagedOnly = it },
                onSortModeChanged = { sortMode = it }
            )
        }
        if (filteredApps.isEmpty()) {
            item {
                EmptyStateCard(
                    title = "No apps match the current view",
                    message = "Adjust search or filters to inspect installed apps and apply targeted policy overrides.",
                    iconRes = com.corepolicy.manager.ui.R.drawable.ic_cpu
                )
            }
        } else {
            items(filteredApps, key = { it.packageName }) { app ->
                val policy = policies[app.packageName] ?: return@items
                AppRow(
                    app = app,
                    policy = policy,
                    onEnabledChanged = { enabled -> applyPolicyUpdate(app.packageName, policy.copy(enabled = enabled)) },
                    onClick = { selectedPackage = app.packageName }
                )
            }
        }
        item { Spacer(modifier = Modifier.height(CorePolicyDesign.spacing.sm)) }
    }

    selectedPackage?.let { pkg ->
        policies[pkg]?.let { policy ->
            ModalBottomSheet(
                onDismissRequest = { selectedPackage = null },
                containerColor = LocalCorePolicyPalette.current.surfaceContainer
            ) {
                AppPolicySheet(
                    policy = policy,
                    onPolicyChanged = { applyPolicyUpdate(pkg, it) },
                    onClose = { selectedPackage = null }
                )
            }
        }
    }
}

@Composable
private fun AppManagerHeader(
    managedCount: Int,
    installedCount: Int,
    filteredCount: Int
) {
    val spacing = CorePolicyDesign.spacing
    Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
        PageHeader(
            eyebrow = "App policies",
            title = "Per-app control",
            subtitle = "Targeted overrides for preload, process rules, battery policy, and profile bias."
        )
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            OverviewInlineBadge("Managed", managedCount.toString(), ChipTone.ACTIVE)
            OverviewInlineBadge("Installed", installedCount.toString(), ChipTone.INFO)
            OverviewInlineBadge("Visible", filteredCount.toString(), ChipTone.NEUTRAL)
        }
    }
}

@Composable
private fun AppManagerToolbar(
    filteredCount: Int,
    query: String,
    showManagedOnly: Boolean,
    sortMode: AppSortMode,
    onQueryChange: (String) -> Unit,
    onManagedOnlyChanged: (Boolean) -> Unit,
    onSortModeChanged: (AppSortMode) -> Unit
) {
    SectionCard {
        SearchBar(query = query, onQueryChange = onQueryChange, placeholder = "Search apps by name or package")
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
            StatusChip(
                text = if (showManagedOnly) "$filteredCount controlled visible" else "$filteredCount visible",
                tone = if (showManagedOnly) ChipTone.ACTIVE else ChipTone.NEUTRAL
            )
        }
        Row(
            modifier = Modifier.horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.sm)
        ) {
            SelectableFilterChip(
                label = "Managed only",
                selected = showManagedOnly,
                onClick = { onManagedOnlyChanged(!showManagedOnly) }
            )
            SelectableFilterChip(
                label = if (sortMode == AppSortMode.NAME) "Sort: name" else "Sort: managed",
                selected = sortMode == AppSortMode.MANAGED_FIRST,
                onClick = {
                    onSortModeChanged(
                        if (sortMode == AppSortMode.NAME) AppSortMode.MANAGED_FIRST else AppSortMode.NAME
                    )
                }
            )
        }
    }
}

@Composable
private fun AppRow(
    app: AppEntry,
    policy: AppPolicy,
    onEnabledChanged: (Boolean) -> Unit,
    onClick: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    SectionCard(onClick = onClick) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalAlignment = Alignment.CenterVertically
        ) {
            AppIcon(app = app)
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                Text(app.appName, style = androidx.compose.material3.MaterialTheme.typography.titleMedium, color = palette.onSurface)
                Text(
                    app.packageName,
                    style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                    color = palette.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
            StatusChip(
                text = when (policy.syncState) {
                    PolicySyncState.PENDING -> "Syncing"
                    PolicySyncState.APPLIED -> "Applied"
                    PolicySyncState.ERROR -> "Error"
                    PolicySyncState.IDLE -> if (policy.enabled) "Managed" else "Idle"
                },
                tone = when (policy.syncState) {
                    PolicySyncState.PENDING -> ChipTone.WARNING
                    PolicySyncState.APPLIED -> ChipTone.SUCCESS
                    PolicySyncState.ERROR -> ChipTone.ERROR
                    PolicySyncState.IDLE -> if (policy.enabled) ChipTone.ACTIVE else ChipTone.NEUTRAL
                }
            )
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(policy.summaryLabel(), style = androidx.compose.material3.MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
            SecondaryButton(text = if (policy.enabled) "Disable" else "Enable", onClick = { onEnabledChanged(!policy.enabled) })
        }
    }
}

@Composable
private fun AppIcon(app: AppEntry) {
    val palette = LocalCorePolicyPalette.current
    Box(
        modifier = Modifier
            .size(42.dp)
            .clip(RoundedCornerShape(CorePolicyDesign.radii.md))
            .background(palette.surfaceRaised),
        contentAlignment = Alignment.Center
    ) {
        if (app.icon != null) {
            Image(bitmap = app.icon, contentDescription = app.appName, modifier = Modifier.size(24.dp))
        } else {
            Text(
                text = app.appName.firstOrNull()?.uppercase() ?: "A",
                style = androidx.compose.material3.MaterialTheme.typography.titleMedium,
                color = palette.onSurface
            )
        }
    }
}

@Composable
private fun AppPolicySheet(
    policy: AppPolicy,
    onPolicyChanged: (AppPolicy) -> Unit,
    onClose: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = spacing.lg, end = spacing.lg, top = spacing.xs, bottom = spacing.xxl),
        verticalArrangement = Arrangement.spacedBy(spacing.lg)
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
            Text(policy.appName, style = androidx.compose.material3.MaterialTheme.typography.headlineSmall, color = palette.onSurface)
            Text(policy.packageName, style = androidx.compose.material3.MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
        }

        SectionCard {
            ModernSwitchRow(
                title = "Manage this app",
                subtitle = "Enable CorePolicy rules for ${policy.appName}",
                checked = policy.enabled,
                onCheckedChange = { onPolicyChanged(policy.copy(enabled = it)) }
            )
            ModernSwitchRow(
                title = "Inherit global defaults",
                subtitle = "Use the active global profile unless a control is overridden below",
                checked = policy.inheritGlobalDefaults,
                onCheckedChange = { onPolicyChanged(policy.copy(inheritGlobalDefaults = it)) }
            )
        }

        SectionHeader("Profile bias", "Choose the base runtime posture for this app")
        SelectorRow(
            options = AppProfile.values().toList(),
            selected = policy.profile,
            labelFor = { it.label },
            onSelect = { onPolicyChanged(policy.copy(profile = it)) }
        )

        SectionCard {
            ModernSwitchRow(
                title = "Preload",
                subtitle = "Warm launch assets before the app is opened",
                checked = policy.preloadEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(preloadEnabled = it)) }
            )
            SelectorRow(
                options = PreloadMode.values().toList(),
                selected = policy.preloadMode,
                labelFor = { it.name.lowercase().replaceFirstChar(Char::uppercase) },
                onSelect = { onPolicyChanged(policy.copy(preloadMode = it)) }
            )
            ModernSwitchRow(
                title = "Process control",
                subtitle = "Foreground and background handling rules",
                checked = policy.processControlEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(processControlEnabled = it)) }
            )
            SelectorRow(
                options = ProcessRuleMode.values().toList(),
                selected = policy.processRuleMode,
                labelFor = { it.name.lowercase().replaceFirstChar(Char::uppercase) },
                onSelect = { onPolicyChanged(policy.copy(processRuleMode = it)) }
            )
            ModernSwitchRow(
                title = "Battery policy",
                subtitle = "Power restriction mode for this package",
                checked = policy.batteryPolicyEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(batteryPolicyEnabled = it)) }
            )
            SelectorRow(
                options = BatteryPolicyMode.values().toList(),
                selected = policy.batteryPolicyMode,
                labelFor = { it.name.lowercase().replace('_', ' ').replaceFirstChar(Char::uppercase) },
                onSelect = { onPolicyChanged(policy.copy(batteryPolicyMode = it)) }
            )
        }

        SectionCard {
            StatusChip(
                text = "Sync ${policy.syncState.name.lowercase().replaceFirstChar(Char::uppercase)}",
                tone = when (policy.syncState) {
                    PolicySyncState.APPLIED -> ChipTone.SUCCESS
                    PolicySyncState.PENDING -> ChipTone.WARNING
                    PolicySyncState.ERROR -> ChipTone.ERROR
                    PolicySyncState.IDLE -> ChipTone.NEUTRAL
                }
            )
            policy.lastAppliedAt?.let {
                Text(
                    text = "Last applied ${formatRelativeTime(it)}",
                    style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                    color = palette.onSurfaceVariant
                )
            }
            Text(policy.summaryLabel(), style = androidx.compose.material3.MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
        }

        HorizontalDivider(color = palette.divider, thickness = 1.dp)
        Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm)) {
            PrimaryButton(text = "Done", onClick = onClose, modifier = Modifier.weight(1f))
            SecondaryButton(
                text = "Reset",
                onClick = {
                    onPolicyChanged(
                        policy.copy(
                            enabled = false,
                            inheritGlobalDefaults = true,
                            profile = AppProfile.DEFAULT,
                            preloadEnabled = false,
                            processControlEnabled = false,
                            batteryPolicyEnabled = false,
                            preloadMode = PreloadMode.OFF,
                            processRuleMode = ProcessRuleMode.DEFAULT,
                            batteryPolicyMode = BatteryPolicyMode.DEFAULT,
                            syncState = PolicySyncState.PENDING
                        )
                    )
                },
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
private fun <T> SelectorRow(
    options: List<T>,
    selected: T,
    labelFor: (T) -> String,
    onSelect: (T) -> Unit
) {
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.sm),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.sm)
    ) {
        options.forEach { option ->
            SelectableFilterChip(
                label = labelFor(option),
                selected = option == selected,
                onClick = { onSelect(option) }
            )
        }
    }
}
