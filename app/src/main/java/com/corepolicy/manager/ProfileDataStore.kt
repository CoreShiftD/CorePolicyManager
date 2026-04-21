package com.corepolicy.manager

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.corepolicy.manager.ui.components.SystemProfile
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "settings")

class ProfileDataStore(private val context: Context) {

    private val PROFILE_KEY = stringPreferencesKey("system_profile")

    val profileFlow: Flow<SystemProfile> = context.dataStore.data.map { preferences ->
        val profileName = preferences[PROFILE_KEY] ?: SystemProfile.BALANCED.name
        try {
            SystemProfile.valueOf(profileName)
        } catch (e: Exception) {
            SystemProfile.BALANCED
        }
    }

    suspend fun saveProfile(profile: SystemProfile) {
        context.dataStore.edit { preferences ->
            preferences[PROFILE_KEY] = profile.name
        }
    }
}
