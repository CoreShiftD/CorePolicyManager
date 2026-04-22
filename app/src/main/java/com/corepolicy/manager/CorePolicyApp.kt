package com.corepolicy.manager

import androidx.compose.runtime.Composable
import com.corepolicy.manager.foundation.VisualFoundationScreen
import com.corepolicy.manager.core.designsystem.theme.AppTheme

@Composable
fun CorePolicyApp() {
    AppTheme {
        VisualFoundationScreen()
    }
}
