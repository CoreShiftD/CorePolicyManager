plugins {
    alias(libs.plugins.kotlin.jvm)
}

kotlin {
    jvmToolchain(libs.versions.java.get().toInt())
}
