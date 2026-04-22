package com.corepolicy.manager.core.data

import android.content.Context
import com.corepolicy.manager.core.data.bridge.LocalDaemonTransport
import com.corepolicy.manager.core.data.bridge.RustJniBridge
import com.corepolicy.manager.core.data.bridge.ShellCommandExecutor
import com.corepolicy.manager.core.data.repository.DefaultCorePolicyRepository
import com.corepolicy.manager.core.domain.CorePolicyRepository
import com.corepolicy.manager.core.domain.usecase.ObserveDaemonStateUseCase
import com.corepolicy.manager.core.domain.usecase.ObserveOverviewUseCase
import com.corepolicy.manager.core.domain.usecase.ObservePoliciesUseCase
import com.corepolicy.manager.core.domain.usecase.ObserveSettingsUseCase
import com.corepolicy.manager.core.domain.usecase.SetCompactDensityUseCase
import com.corepolicy.manager.core.domain.usecase.SetConfirmBeforePolicyApplyUseCase
import com.corepolicy.manager.core.domain.usecase.SetDynamicColorUseCase
import com.corepolicy.manager.core.domain.usecase.UpdatePolicyModeUseCase

class CorePolicyContainer(context: Context) {
    private val appContext = context.applicationContext

    private val commandExecutor = ShellCommandExecutor()
    private val daemonTransport = LocalDaemonTransport()
    private val rustBridge = RustJniBridge()

    val repository: CorePolicyRepository = DefaultCorePolicyRepository(
        context = appContext,
        commandExecutor = commandExecutor,
        daemonTransport = daemonTransport,
        rustBridge = rustBridge,
    )

    val observeOverview = ObserveOverviewUseCase(repository)
    val observeDaemonState = ObserveDaemonStateUseCase(repository)
    val observePolicies = ObservePoliciesUseCase(repository)
    val observeSettings = ObserveSettingsUseCase(repository)
    val updatePolicyMode = UpdatePolicyModeUseCase(repository)
    val setDynamicColor = SetDynamicColorUseCase(repository)
    val setCompactDensity = SetCompactDensityUseCase(repository)
    val setConfirmBeforePolicyApply = SetConfirmBeforePolicyApplyUseCase(repository)
}
