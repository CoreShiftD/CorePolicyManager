package com.corepolicy.manager.app.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue

class AppNavigationState(
    private val destinationState: MutableState<AppDestination>,
) {
    var currentDestination by destinationState
        private set

    fun navigateTo(destination: AppDestination) {
        currentDestination = destination
    }
}

@Composable
fun rememberAppNavigationState(): AppNavigationState {
    val saver = Saver<MutableState<AppDestination>, String>(
        save = { it.value.name },
        restore = { androidx.compose.runtime.mutableStateOf(AppDestination.valueOf(it)) },
    )
    val destination = rememberSaveable(saver = saver) {
        androidx.compose.runtime.mutableStateOf(AppDestination.Overview)
    }
    return AppNavigationState(destinationState = destination)
}
