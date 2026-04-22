plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.corepolicy.manager.feature.overview"
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

    buildFeatures {
        compose = true
    }
}

kotlin {
    jvmToolchain(libs.versions.java.get().toInt())
}

dependencies {
    implementation(project(":core:domain"))
    implementation(project(":core:designsystem"))

    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)

    debugImplementation(libs.androidx.compose.ui.tooling)
}
