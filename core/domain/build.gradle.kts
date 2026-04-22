plugins {
    alias(libs.plugins.kotlin.jvm)
}

kotlin {
    jvmToolchain(libs.versions.java.get().toInt())
}

dependencies {
    api(project(":core:model"))
    api(libs.kotlinx.coroutines.core)
}
