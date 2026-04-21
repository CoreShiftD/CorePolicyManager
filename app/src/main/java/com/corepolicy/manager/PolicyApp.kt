package com.corepolicy.manager

import android.os.Build
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.components.DynamicMetric
import com.corepolicy.manager.ui.components.InsightItem
import com.corepolicy.manager.ui.components.InsightTone
import com.corepolicy.manager.ui.components.MetricState
import com.corepolicy.manager.ui.components.MetricType
import com.corepolicy.manager.ui.components.SystemProfile
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.util.Locale

private fun normalizeKernelVersion(raw: String?): String {
    if (raw.isNullOrBlank()) return "Unknown"
    val normalized = Regex("""\d+\.\d+(?:\.\d+)?""").find(raw)?.value
    return normalized ?: raw.substringBefore('-')
}

private fun normalizeArchitecture(rawAbi: String?): String {
    val abi = rawAbi.orEmpty().lowercase(Locale.US)
    return when {
        "arm64" in abi -> "ARM64"
        "armeabi" in abi || "armv7" in abi -> "ARM32"
        "x86_64" in abi -> "X86_64"
        abi == "x86" -> "X86"
        abi.isBlank() -> "Unknown"
        else -> rawAbi!!.uppercase(Locale.US).replace('-', '_')
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PolicyApp(profileDataStore: ProfileDataStore) {
    val context = LocalContext.current
    val configuration = LocalConfiguration.current
    val palette = LocalCorePolicyPalette.current
    val coroutineScope = rememberCoroutineScope()
    val daemonService = remember { MockDaemonPolicyService() }
    val appPolicyRepository = remember(context) { AppPolicyRepository(context) }

    val daemonStatus by daemonService.observeOverview().collectAsState(
        initial = DaemonOverviewStatus(
            state = DaemonState.RUNNING,
            activeProfile = SystemProfile.BALANCED,
            enabledModules = 3,
            lastAction = "Boot sync complete",
            disconnected = false,
            uptimeMs = 0L,
            lastSyncTimestampMs = System.currentTimeMillis(),
            warningCount = 0,
            errorCount = 0,
            restartInProgress = false
        )
    )
    val modules by daemonService.observeModules().collectAsState(initial = emptyList())
    val logs by daemonService.observeLogs().collectAsState(initial = emptyList())
    val savedPolicies by appPolicyRepository.loadPolicies().collectAsState(initial = emptyMap())
    val selectedProfile by profileDataStore.profileFlow.collectAsState(initial = SystemProfile.BALANCED)

    var selectedSection by remember { mutableStateOf(AppSection.OVERVIEW) }
    var showProfileSheet by remember { mutableStateOf(false) }
    val sheetState = rememberModalBottomSheetState()

    val tick by produceState(initialValue = 0L) {
        while (true) {
            value = System.currentTimeMillis()
            delay(1500)
        }
    }

    val metrics = remember(tick) { buildLiveMetrics(tick) }
    val insights = remember(tick) { buildLiveInsights(tick) }

    val architecture = normalizeArchitecture(Build.SUPPORTED_ABIS.firstOrNull())
    val androidVersion = Build.VERSION.RELEASE ?: "Unknown"
    val kernelVersion = normalizeKernelVersion(System.getProperty("os.version"))
    val useNavRail = configuration.screenWidthDp >= 840

    Scaffold(
        containerColor = palette.background,
        bottomBar = {
            if (!useNavRail) {
                NavigationShell(
                    selectedSection = selectedSection,
                    onSectionSelected = { selectedSection = it },
                    layout = NavigationShellLayout.BOTTOM_BAR
                )
            }
        }
    ) { innerPadding ->
        val screenPadding = PaddingValues(
            start = CorePolicyDimens.screenHorizontal,
            end = CorePolicyDimens.screenHorizontal,
            top = CorePolicyDimens.screenTop,
            bottom = 16.dp
        )
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            if (useNavRail) {
                NavigationShell(
                    selectedSection = selectedSection,
                    onSectionSelected = { selectedSection = it },
                    layout = NavigationShellLayout.NAV_RAIL
                )
            }
            AnimatedContent(
                targetState = selectedSection,
                transitionSpec = {
                    val dir = if (targetState.ordinal > initialState.ordinal) 1 else -1
                    (slideInHorizontally(tween(260)) { it / 8 * dir } + fadeIn(tween(200))) togetherWith
                        (slideOutHorizontally(tween(200)) { -it / 8 * dir } + fadeOut(tween(160)))
                },
                modifier = Modifier
                    .fillMaxHeight()
                    .weight(1f),
                label = "sectionCrossfade"
            ) { section ->
                when (section) {
                    AppSection.OVERVIEW -> OverviewScreen(
                        metrics = metrics,
                        insights = insights,
                        systemInfo = Triple("mt6789", architecture, kernelVersion),
                        runtimeInfo = Triple("8 GB", "schedutil", androidVersion),
                        selectedProfile = selectedProfile,
                        daemonStatus = daemonStatus,
                        managedAppsCount = savedPolicies.size,
                        onProfileClick = { showProfileSheet = true },
                        onRestartDaemon = { coroutineScope.launch { daemonService.restartDaemon() } },
                        onOpenLogs = { selectedSection = AppSection.LOGS },
                        onManageModules = { selectedSection = AppSection.MODULES },
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(screenPadding)
                    )

                    AppSection.MODULES -> ModulesScreen(
                        modules = modules,
                        onToggle = { id, enabled -> coroutineScope.launch { daemonService.setModuleEnabled(id, enabled) } },
                        onOpenLogs = { selectedSection = AppSection.LOGS },
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(screenPadding)
                    )

                    AppSection.APP_MANAGER -> AppManagerScreen(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(screenPadding),
                        daemonPolicyService = daemonService
                    )

                    AppSection.PROFILES -> ProfilesScreen(
                        selectedProfile = selectedProfile,
                        onSelect = { profile ->
                            coroutineScope.launch {
                                profileDataStore.saveProfile(profile)
                                daemonService.applyProfile(profile)
                            }
                        },
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(screenPadding)
                    )

                    AppSection.LOGS -> LogsScreen(
                        logs = logs,
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(screenPadding)
                    )
                }
            }
        }
    }

    if (showProfileSheet) {
        ModalBottomSheet(
            onDismissRequest = { showProfileSheet = false },
            sheetState = sheetState,
            containerColor = palette.surfaceContainer
        ) {
            ProfilePickerSheet(
                selectedProfile = selectedProfile,
                onSelect = { profile ->
                    coroutineScope.launch {
                        profileDataStore.saveProfile(profile)
                        daemonService.applyProfile(profile)
                    }
                    showProfileSheet = false
                }
            )
        }
    }
}

@Composable
private fun ProfilePickerSheet(
    selectedProfile: SystemProfile,
    onSelect: (SystemProfile) -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Text("Choose a profile", style = MaterialTheme.typography.titleLarge, color = palette.onSurface)
        Text(
            "Tune daemon behavior by workload target.",
            style = MaterialTheme.typography.bodyMedium,
            color = palette.onSurfaceVariant
        )
        Spacer(Modifier.height(4.dp))
        SystemProfile.values().forEach { profile ->
            val selected = profile == selectedProfile
            val summary = when (profile) {
                SystemProfile.PERFORMANCE -> "Max speed · CPU biased high · aggressive preload"
                SystemProfile.BALANCED -> "Adaptive scaling · safe thermals"
                SystemProfile.EFFICIENCY -> "Battery-first · low background load"
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(CorePolicyDimens.cardRadiusTight))
                    .background(if (selected) palette.primaryContainer else palette.surfaceContainerHigh)
                    .clickable { onSelect(profile) }
                    .padding(horizontal = 14.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Box(
                    modifier = Modifier
                        .size(36.dp)
                        .clip(CircleShape)
                        .background(
                            if (selected) palette.onPrimaryContainer.copy(alpha = 0.18f)
                            else palette.surfaceContainerHighest
                        ),
                    contentAlignment = Alignment.Center
                ) {
                    Image(
                        painter = painterResource(id = profile.iconRes),
                        contentDescription = profile.title,
                        colorFilter = ColorFilter.tint(
                            if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant
                        ),
                        modifier = Modifier.size(18.dp)
                    )
                }
                Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        profile.title,
                        style = MaterialTheme.typography.titleMedium,
                        color = if (selected) palette.onPrimaryContainer else palette.onSurface
                    )
                    Text(
                        summary,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (selected) palette.onPrimaryContainer.copy(alpha = 0.85f) else palette.onSurfaceVariant
                    )
                }
            }
        }
        Spacer(Modifier.height(8.dp))
    }
}

/* -------------------------------------------------------------------------- */
/*  Live metric builders — kept pure & outside the composable for clarity.    */
/* -------------------------------------------------------------------------- */

private fun buildLiveMetrics(tick: Long): List<DynamicMetric> {
    val cpuLoad = 25 + ((tick / 1000) % 65).toInt()
    val activeCores = 2 + ((tick / 1300) % 7).toInt()
    val ramUsedGb = 2.4f + (((tick / 1200) % 20).toInt() / 10f)
    val ramPercent = ((ramUsedGb / 8.0f) * 100).toInt()
    val batteryLevel = 45 + ((tick / 2200) % 52).toInt()
    val isCharging = ((tick / 5000) % 2L) == 0L
    val chargingWatts = 10 + ((tick / 900) % 9).toInt()
    val thermalCelsius = 38 + ((tick / 2500) % 28).toInt()

    val cpuTrend = trend((((tick / 4000) % 9).toInt() - 4), "%")
    val ramTrend = trend((((tick / 2600) % 9).toInt() - 4), "%")
    val batteryTrend = trend(if (isCharging) 4 else -4, "%")
    val thermalTrend = trend((((tick / 3400) % 7).toInt() - 3), "°C")

    val cpuState = when {
        cpuLoad >= 75 -> MetricState.CRITICAL
        cpuLoad >= 50 -> MetricState.WARNING
        else -> MetricState.CALM
    }
    val ramState = when {
        ramPercent >= 75 -> MetricState.CRITICAL
        ramPercent >= 50 -> MetricState.WARNING
        else -> MetricState.CALM
    }
    val batteryState = when {
        isCharging -> MetricState.CALM
        batteryLevel < 20 -> MetricState.CRITICAL
        batteryLevel < 40 -> MetricState.WARNING
        else -> MetricState.NEUTRAL
    }
    val thermalStateLabel = when {
        thermalCelsius < 38 -> "Low"
        thermalCelsius < 48 -> "Moderate"
        thermalCelsius < 58 -> "High"
        else -> "Critical"
    }
    val thermalState = when {
        thermalCelsius < 38 -> MetricState.CALM
        thermalCelsius < 48 -> MetricState.WARNING
        thermalCelsius < 58 -> MetricState.HIGH
        else -> MetricState.CRITICAL
    }

    return listOf(
        DynamicMetric("CPU Load", "$cpuLoad%", "$activeCores cores active", cpuTrend, cpuLoad / 100f, MetricType.CAPACITY, cpuState, R.drawable.ic_cpu),
        DynamicMetric("Thermal", "$thermalCelsius°C", thermalStateLabel, thermalTrend, 0f, MetricType.STATE, thermalState, R.drawable.ic_schedule),
        DynamicMetric("RAM", "$ramPercent%", String.format(Locale.US, "%.1f / 8 GB", ramUsedGb), ramTrend, (ramPercent / 100f), MetricType.CAPACITY, ramState, R.drawable.ic_memory),
        DynamicMetric("Battery", "$batteryLevel%", if (isCharging) "Charging · ${chargingWatts}W" else "Discharging", batteryTrend, (batteryLevel / 100f), MetricType.CAPACITY, batteryState, R.drawable.ic_efficiency)
    )
}

private fun buildLiveInsights(tick: Long): List<InsightItem> {
    val cpuLoad = 25 + ((tick / 1000) % 65).toInt()
    val activeCores = 2 + ((tick / 1300) % 7).toInt()
    val batteryLevel = 45 + ((tick / 2200) % 52).toInt()
    val isCharging = ((tick / 5000) % 2L) == 0L
    val chargingWatts = 10 + ((tick / 900) % 9).toInt()
    val thermalCelsius = 38 + ((tick / 2500) % 28).toInt()
    val thermalStateLabel = when {
        thermalCelsius < 38 -> "Low"
        thermalCelsius < 48 -> "Moderate"
        thermalCelsius < 58 -> "High"
        else -> "Critical"
    }
    return listOf(
        InsightItem(
            R.drawable.ic_cpu,
            if (cpuLoad >= 50) "CPU elevated at $cpuLoad% across $activeCores active cores"
            else "CPU stable at $cpuLoad% across $activeCores active cores",
            if (cpuLoad >= 50) "Under load" else "Stable",
            if (cpuLoad >= 50) InsightTone.WARNING else InsightTone.POSITIVE
        ),
        InsightItem(
            R.drawable.ic_schedule,
            if (thermalCelsius >= 48) "Thermals elevated at $thermalCelsius°C"
            else "Thermals steady at $thermalCelsius°C",
            thermalStateLabel,
            if (thermalCelsius >= 58) InsightTone.CRITICAL
            else if (thermalCelsius >= 48) InsightTone.WARNING
            else InsightTone.POSITIVE
        ),
        InsightItem(
            R.drawable.ic_efficiency,
            if (isCharging) "Charging at ${chargingWatts}W · battery at $batteryLevel%"
            else "Battery at $batteryLevel% · discharging",
            if (isCharging) "Charging" else "On battery",
            if (isCharging) InsightTone.POSITIVE else InsightTone.NEUTRAL
        )
    )
}

private fun trend(delta: Int, unit: String): String {
    val abs = kotlin.math.abs(delta)
    return if (delta == 0) "—" else String.format(
        Locale.US,
        "%s%d%s · 5m",
        if (delta > 0) "↑" else "↓",
        abs,
        unit
    )
}

@Suppress("unused")
private val _keepFontWeight = FontWeight.SemiBold
