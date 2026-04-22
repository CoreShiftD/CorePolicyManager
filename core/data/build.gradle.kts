plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.corepolicy.manager.core.data"
    compileSdk = libs.versions.compileSdk.get().toInt()

    defaultConfig {
        minSdk = libs.versions.minSdk.get().toInt()
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        val javaVersion = JavaVersion.toVersion(libs.versions.java.get())
        sourceCompatibility = javaVersion
        targetCompatibility = javaVersion
    }
}

kotlin {
    jvmToolchain(libs.versions.java.get().toInt())
}

dependencies {
    api(project(":core:domain"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.datastore.preferences)
    implementation(libs.kotlinx.coroutines.core)
}
