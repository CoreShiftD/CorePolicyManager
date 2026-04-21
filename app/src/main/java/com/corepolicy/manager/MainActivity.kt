package com.corepolicy.manager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.corepolicy.manager.ui.theme.CorePolicyTheme

class MainActivity : ComponentActivity() {
    private lateinit var profileDataStore: ProfileDataStore

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        profileDataStore = ProfileDataStore(this)

        setContent {
            CorePolicyTheme {
                PolicyApp(profileDataStore)
            }
        }
    }
}
