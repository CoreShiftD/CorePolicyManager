package com.corepolicy.manager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val application = application as CorePolicyApplication
        setContent {
            CorePolicyApp(container = application.container)
        }
    }
}
