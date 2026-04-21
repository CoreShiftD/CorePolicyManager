package com.corepolicy.manager

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import org.json.JSONArray
import org.json.JSONObject

private val Context.appPolicyDataStore by preferencesDataStore(name = "app_policies")

class AppPolicyRepository(private val context: Context) {
    private val policiesKey = stringPreferencesKey("policies_json")

    fun loadPolicies(): Flow<Map<String, AppPolicy>> =
        context.appPolicyDataStore.data.map { preferences ->
            decodePolicies(preferences[policiesKey])
        }

    suspend fun savePolicy(policy: AppPolicy) {
        context.appPolicyDataStore.edit { preferences ->
            val current = decodePolicies(preferences[policiesKey]).toMutableMap()
            current[policy.packageName] = policy
            preferences[policiesKey] = encodePolicies(current)
        }
    }

    private fun encodePolicies(map: Map<String, AppPolicy>): String {
        val array = JSONArray()
        map.values.forEach { policy ->
            array.put(
                JSONObject()
                    .put("packageName", policy.packageName)
                    .put("appName", policy.appName)
                    .put("enabled", policy.enabled)
                    .put("inheritGlobalDefaults", policy.inheritGlobalDefaults)
                    .put("profile", policy.profile.name)
                    .put("preloadEnabled", policy.preloadEnabled)
                    .put("processControlEnabled", policy.processControlEnabled)
                    .put("batteryPolicyEnabled", policy.batteryPolicyEnabled)
                    .put("preloadMode", policy.preloadMode.name)
                    .put("processRuleMode", policy.processRuleMode.name)
                    .put("batteryPolicyMode", policy.batteryPolicyMode.name)
                    .put("syncState", policy.syncState.name)
                    .put("lastAppliedAt", policy.lastAppliedAt ?: JSONObject.NULL)
            )
        }
        return array.toString()
    }

    private fun decodePolicies(raw: String?): Map<String, AppPolicy> {
        if (raw.isNullOrBlank()) return emptyMap()
        return runCatching {
            val array = JSONArray(raw)
            buildMap {
                repeat(array.length()) { index ->
                    val item = array.optJSONObject(index) ?: return@repeat
                    val packageName = item.optString("packageName")
                    if (packageName.isBlank()) return@repeat
                    put(
                        packageName,
                        AppPolicy(
                            packageName = packageName,
                            appName = item.optString("appName", packageName),
                            enabled = item.optBoolean("enabled", false),
                            inheritGlobalDefaults = item.optBoolean("inheritGlobalDefaults", true),
                            profile = runCatching { AppProfile.valueOf(item.optString("profile", AppProfile.DEFAULT.name)) }.getOrDefault(AppProfile.DEFAULT),
                            preloadEnabled = item.optBoolean("preloadEnabled", false),
                            processControlEnabled = item.optBoolean("processControlEnabled", false),
                            batteryPolicyEnabled = item.optBoolean("batteryPolicyEnabled", false),
                            preloadMode = runCatching { PreloadMode.valueOf(item.optString("preloadMode", PreloadMode.OFF.name)) }.getOrDefault(PreloadMode.OFF),
                            processRuleMode = runCatching { ProcessRuleMode.valueOf(item.optString("processRuleMode", ProcessRuleMode.DEFAULT.name)) }.getOrDefault(ProcessRuleMode.DEFAULT),
                            batteryPolicyMode = runCatching { BatteryPolicyMode.valueOf(item.optString("batteryPolicyMode", BatteryPolicyMode.DEFAULT.name)) }.getOrDefault(BatteryPolicyMode.DEFAULT),
                            syncState = runCatching { PolicySyncState.valueOf(item.optString("syncState", PolicySyncState.IDLE.name)) }.getOrDefault(PolicySyncState.IDLE),
                            lastAppliedAt = if (item.isNull("lastAppliedAt")) null else item.optLong("lastAppliedAt")
                        )
                    )
                }
            }
        }.getOrDefault(emptyMap())
    }
}
