package com.corepolicy.manager

import android.content.pm.PackageManager
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.graphics.drawable.toBitmap
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

private data class AppManagerContentState(
    val appEntries: List<AppEntry>,
    val filteredApps: List<AppEntry>,
    val managedCount: Int,
    val query: String,
    val showManagedOnly: Boolean,
    val sortMode: AppSortMode
)

private fun AppPolicy.summaryLabel(): String =
    if (inheritGlobalDefaults) "Inheriting global defaults" else "Custom policy overrides active"

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

@Composable
@OptIn(ExperimentalMaterial3Api::class)
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
                    icon = iconCache[pkg] ?: runCatching { pm.getApplicationIcon(info).toBitmap(48, 48).asImageBitmap() }.getOrNull().also {
                        iconCache[pkg] = it
                    }
                )
            }
        appEntries.clear()
        appEntries.addAll(installed)
        installed.forEach { app ->
            val persisted = persistedPolicies[app.packageName]
            policies.putIfAbsent(
                app.packageName,
                persisted ?: defaultAppPolicy(app)
            )
        }
    }
    LaunchedEffect(persistedPolicies) {
        persistedPolicies.forEach { (pkg, policy) -> policies[pkg] = policy }
    }

    val filteredApps = appEntries.filter {
        query.isBlank() || it.appName.contains(query, ignoreCase = true) || it.packageName.contains(query, ignoreCase = true)
    }.filter { !showManagedOnly || (policies[it.packageName]?.enabled == true) }
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
            val applied = pending.copy(
                syncState = PolicySyncState.APPLIED,
                lastAppliedAt = System.currentTimeMillis()
            )
            policies[pkg] = applied
            repository.savePolicy(applied)
        }
    }
    val contentState = AppManagerContentState(
        appEntries = appEntries,
        filteredApps = filteredApps,
        managedCount = managedCount,
        query = query,
        showManagedOnly = showManagedOnly,
        sortMode = sortMode
    )

    AppManagerContent(
        modifier = modifier,
        state = contentState,
        policyFor = { pkg -> policies[pkg] },
        onQueryChange = { query = it },
        onManagedOnlyChanged = { showManagedOnly = it },
        onSortModeChanged = { sortMode = it },
        onEnabledChanged = { pkg, enabled ->
            policies[pkg]?.let { policy ->
                applyPolicyUpdate(pkg, policy.copy(enabled = enabled))
            }
        },
        onAppSelected = { selectedPackage = it }
    )

    selectedPackage?.let { pkg ->
        val policy = policies[pkg]
        if (policy != null) {
            AppPolicyBottomSheet(
                policy = policy,
                onPolicyChanged = { applyPolicyUpdate(pkg, it) },
                onClose = { selectedPackage = null }
            )
        }
    }
}

@Composable
private fun AppManagerContent(
    state: AppManagerContentState,
    policyFor: (String) -> AppPolicy?,
    onQueryChange: (String) -> Unit,
    onManagedOnlyChanged: (Boolean) -> Unit,
    onSortModeChanged: (AppSortMode) -> Unit,
    onEnabledChanged: (String, Boolean) -> Unit,
    onAppSelected: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    LazyColumn(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        item {
            AppManagerHeader(
                managedCount = state.managedCount,
                installedCount = state.appEntries.size,
                filteredCount = state.filteredApps.size
            )
        }
        item {
            AppManagerToolbar(
                filteredCount = state.filteredApps.size,
                query = state.query,
                showManagedOnly = state.showManagedOnly,
                sortMode = state.sortMode,
                onQueryChange = onQueryChange,
                onManagedOnlyChanged = onManagedOnlyChanged,
                onSortModeChanged = onSortModeChanged
            )
        }
        if (state.filteredApps.isEmpty()) {
            item {
                EmptyStateCard(
                    title = "No apps match the current view",
                    message = "Adjust search or filters to inspect installed apps and apply targeted policy overrides.",
                    iconRes = com.corepolicy.manager.ui.R.drawable.ic_cpu
                )
            }
        } else {
            items(state.filteredApps, key = { it.packageName }) { app ->
                val policy = policyFor(app.packageName) ?: return@items
                AppRow(
                    app = app,
                    policy = policy,
                    onEnabledChanged = { enabled ->
                        onEnabledChanged(app.packageName, enabled)
                    },
                    onClick = { onAppSelected(app.packageName) }
                )
            }
        }
        item { Spacer(modifier = Modifier.height(6.dp)) }
    }
}

@Composable
private fun AppManagerHeader(
    managedCount: Int,
    installedCount: Int,
    filteredCount: Int
) {
    Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
        PageHeader(
            eyebrow = "Application Policy",
            title = "App Manager",
            subtitle = "Manage targeted policy overrides without changing the global daemon profile."
        )
        SectionCard {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OverviewInlineBadge(
                    label = "Managed",
                    value = managedCount.toString(),
                    tone = ChipTone.ACTIVE,
                    modifier = Modifier.weight(1f)
                )
                OverviewInlineBadge(
                    label = "Installed",
                    value = installedCount.toString(),
                    tone = ChipTone.INFO,
                    modifier = Modifier.weight(1f)
                )
                OverviewInlineBadge(
                    label = "Visible",
                    value = filteredCount.toString(),
                    tone = ChipTone.NEUTRAL,
                    modifier = Modifier.weight(1f)
                )
            }
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
        SectionHeader(
            title = "App controls",
            subtitle = "$filteredCount visible apps",
            trailing = {
                StatusChip(
                    text = if (showManagedOnly) "Managed view" else "All apps",
                    tone = if (showManagedOnly) ChipTone.ACTIVE else ChipTone.NEUTRAL
                )
            }
        )
        SearchBar(
            query = query,
            onQueryChange = onQueryChange,
            placeholder = "Search apps by name or package"
        )
        AppManagerFilterRow(
            showManagedOnly = showManagedOnly,
            sortMode = sortMode,
            onManagedOnlyChanged = onManagedOnlyChanged,
            onSortModeChanged = onSortModeChanged
        )
    }
}

@Composable
@OptIn(ExperimentalMaterial3Api::class)
private fun AppPolicyBottomSheet(
    policy: AppPolicy,
    onPolicyChanged: (AppPolicy) -> Unit,
    onClose: () -> Unit
) {
    ModalBottomSheet(onDismissRequest = onClose) {
        AppPolicySheet(
            policy = policy,
            onPolicyChanged = onPolicyChanged,
            onClose = onClose
        )
    }
}

@Composable
private fun AppRow(
    app: AppEntry,
    policy: AppPolicy,
    onEnabledChanged: (Boolean) -> Unit,
    onClick: () -> Unit
) {
    ControlCard(onClick = onClick) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            AppIdentityBlock(app = app, modifier = Modifier.weight(1f))
            AppRowControls(policy = policy, onEnabledChanged = onEnabledChanged)
        }
        AppPolicyStateRow(policy = policy)
    }
}

@Composable
private fun AppIdentityBlock(
    app: AppEntry,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Box(
            modifier = Modifier
                .size(44.dp)
                .clip(RoundedCornerShape(CorePolicyDimens.cardRadiusTight))
                .background(palette.surfaceContainerHigh),
            contentAlignment = Alignment.Center
        ) {
            if (app.icon != null) {
                Image(
                    bitmap = app.icon,
                    contentDescription = app.appName,
                    modifier = Modifier.size(30.dp)
                )
            } else {
                androidx.compose.material3.Text(
                    app.appName.firstOrNull()?.uppercase() ?: "A",
                    style = androidx.compose.material3.MaterialTheme.typography.titleMedium,
                    color = palette.onSurface
                )
            }
        }
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            androidx.compose.material3.Text(
                app.appName,
                style = androidx.compose.material3.MaterialTheme.typography.titleSmall,
                color = palette.onSurface,
                maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis
            )
            androidx.compose.material3.Text(
                app.packageName,
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                color = palette.onSurfaceVariant,
                maxLines = 1,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun AppRowControls(
    policy: AppPolicy,
    onEnabledChanged: (Boolean) -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        horizontalAlignment = Alignment.End,
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            if (policy.enabled) {
                StatusChip("Managed", ChipTone.ACTIVE, leadingDot = true)
            }
            if (policy.syncState == PolicySyncState.PENDING) {
                StatusChip("Syncing", ChipTone.WARNING)
            }
        }
        Switch(
            checked = policy.enabled,
            onCheckedChange = onEnabledChanged,
            colors = androidx.compose.material3.SwitchDefaults.colors(
                checkedThumbColor = palette.onPrimaryContainer,
                checkedTrackColor = palette.primary,
                uncheckedThumbColor = palette.onSurfaceVariant,
                uncheckedTrackColor = palette.surfaceContainerHigh,
                uncheckedBorderColor = palette.divider
            )
        )
    }
}

@Composable
private fun AppPolicyStateRow(policy: AppPolicy) {
    val palette = LocalCorePolicyPalette.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        androidx.compose.material3.Text(
            text = policy.summaryLabel(),
            style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
            color = palette.onSurfaceVariant
        )
        androidx.compose.material3.Text(
            text = "Profile · ${policy.profile.label}",
            style = androidx.compose.material3.MaterialTheme.typography.labelMedium,
            color = palette.onSurfaceVariant
        )
    }
}

@Composable
private fun AppPolicySheet(
    policy: AppPolicy,
    onPolicyChanged: (AppPolicy) -> Unit,
    onClose: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 20.dp, bottom = 24.dp, top = 4.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            androidx.compose.material3.Text(
                policy.appName,
                style = androidx.compose.material3.MaterialTheme.typography.titleLarge,
                color = palette.onSurface
            )
            androidx.compose.material3.Text(
                policy.packageName,
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                color = palette.onSurfaceVariant
            )
        }

        SectionCard {
            ModernSwitchRow(
                title = "Manage this app",
                subtitle = "Apply CorePolicy rules to ${policy.appName}",
                checked = policy.enabled,
                onCheckedChange = { onPolicyChanged(policy.copy(enabled = it)) }
            )
            ModernSwitchRow(
                title = "Inherit global defaults",
                subtitle = "Falls back to profile-wide settings",
                checked = policy.inheritGlobalDefaults,
                onCheckedChange = { onPolicyChanged(policy.copy(inheritGlobalDefaults = it)) }
            )
        }

        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            androidx.compose.material3.Text(
                "Profile",
                style = androidx.compose.material3.MaterialTheme.typography.labelLarge,
                color = palette.onSurfaceVariant
            )
            ProfileChipRow(
                selectedProfile = policy.profile,
                onProfileSelected = { onPolicyChanged(policy.copy(profile = it)) }
            )
        }

        SectionCard {
            ModernSwitchRow(
                title = "Preload",
                subtitle = "App warmup and launch cache",
                checked = policy.preloadEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(preloadEnabled = it)) }
            )
            ModernSwitchRow(
                title = "Process control",
                subtitle = "Foreground / background rules",
                checked = policy.processControlEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(processControlEnabled = it)) }
            )
            ModernSwitchRow(
                title = "Battery policy",
                subtitle = "Per-app power restrictions",
                checked = policy.batteryPolicyEnabled,
                onCheckedChange = { onPolicyChanged(policy.copy(batteryPolicyEnabled = it)) }
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            androidx.compose.material3.Text(
                text = if (!policy.enabled && policy.inheritGlobalDefaults && policy.profile == AppProfile.DEFAULT &&
                    !policy.preloadEnabled && !policy.processControlEnabled && !policy.batteryPolicyEnabled
                ) "Uses global defaults" else policy.summaryLabel(),
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                color = palette.onSurfaceVariant
            )
            StatusChip(
                text = "Sync · ${policy.syncState.name.lowercase().replaceFirstChar { it.uppercase() }}",
                tone = when (policy.syncState) {
                    PolicySyncState.APPLIED -> ChipTone.SUCCESS
                    PolicySyncState.PENDING -> ChipTone.WARNING
                    PolicySyncState.ERROR -> ChipTone.ERROR
                    PolicySyncState.IDLE -> ChipTone.NEUTRAL
                }
            )
        }

        AppPolicyActionArea(
            onDone = onClose,
            onReset = {
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
            }
        )
        Spacer(modifier = Modifier.height(8.dp))
    }
}

@Composable
private fun ProfileChipRow(
    selectedProfile: AppProfile,
    onProfileSelected: (AppProfile) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        AppProfile.values().forEach { profile ->
            val selected = profile == selectedProfile
            StatusChip(
                text = profile.label,
                tone = if (selected) ChipTone.ACTIVE else ChipTone.NEUTRAL,
                modifier = Modifier.clickable { onProfileSelected(profile) }
            )
        }
    }
}

@Composable
private fun AppManagerFilterRow(
    showManagedOnly: Boolean,
    sortMode: AppSortMode,
    onManagedOnlyChanged: (Boolean) -> Unit,
    onSortModeChanged: (AppSortMode) -> Unit
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        SelectableFilterChip(
            label = "Managed only",
            selected = showManagedOnly,
            onClick = { onManagedOnlyChanged(!showManagedOnly) }
        )
        SelectableFilterChip(
            label = if (sortMode == AppSortMode.NAME) "Sort: Name" else "Sort: Managed",
            selected = sortMode == AppSortMode.MANAGED_FIRST,
            onClick = {
                onSortModeChanged(if (sortMode == AppSortMode.NAME) AppSortMode.MANAGED_FIRST else AppSortMode.NAME)
            }
        )
    }
}

@Composable
private fun AppPolicyActionArea(
    onDone: () -> Unit,
    onReset: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    HorizontalDivider(color = palette.divider, thickness = 1.dp)
    Spacer(modifier = Modifier.height(4.dp))
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        // Primary action
        Box(
            modifier = Modifier
                .clip(RoundedCornerShape(CorePolicyDimens.chipRadius))
                .background(palette.primary)
                .clickable(onClick = onDone)
                .padding(horizontal = 24.dp, vertical = 13.dp)
        ) {
            androidx.compose.material3.Text(
                "Done",
                style = androidx.compose.material3.MaterialTheme.typography.labelLarge
                    .copy(fontWeight = FontWeight.SemiBold),
                color = palette.onPrimary
            )
        }
        // Secondary action
        Box(
            modifier = Modifier
                .clip(RoundedCornerShape(CorePolicyDimens.chipRadius))
                .background(palette.surfaceContainerHigh)
                .border(1.dp, palette.divider, RoundedCornerShape(CorePolicyDimens.chipRadius))
                .clickable(onClick = onReset)
                .padding(horizontal = 24.dp, vertical = 13.dp)
        ) {
            androidx.compose.material3.Text(
                "Reset to defaults",
                style = androidx.compose.material3.MaterialTheme.typography.labelLarge,
                color = palette.onSurfaceVariant
            )
        }
    }
}
