package com.corepolicy.manager.core.designsystem.component

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
fun AppScaffold(
    topBar: @Composable () -> Unit = {},
    bottomBar: @Composable () -> Unit = {},
    content: @Composable (PaddingValues) -> Unit,
) {
    Scaffold(
        topBar = topBar,
        bottomBar = bottomBar,
    ) { paddingValues ->
        Surface(
            modifier = Modifier,
        ) {
            content(paddingValues)
        }
    }
}
