# AI Project Context

Generated: 2026-04-24T07:03:05

## File Tree

```text
.github/workflows/build-release.yml
app/build.gradle.kts
app/proguard-rules.pro
app/src/main/AndroidManifest.xml
app/src/main/kotlin/com/corepolicy/manager/CorePolicyApplication.kt
app/src/main/kotlin/com/corepolicy/manager/MainActivity.kt
app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml
app/src/main/res/mipmap-anydpi-v26/ic_launcher_round.xml
app/src/main/res/values/themes.xml
build.gradle.kts
core/common/build.gradle.kts
core/common/src/main/AndroidManifest.xml
core/data/build.gradle.kts
core/data/consumer-rules.pro
core/data/src/main/AndroidManifest.xml
core/database/build.gradle.kts
core/database/src/main/AndroidManifest.xml
core/designsystem/build.gradle.kts
core/designsystem/consumer-rules.pro
core/designsystem/src/main/AndroidManifest.xml
core/designsystem/src/main/kotlin/com/corepolicy/manager/core/designsystem/Theme.kt
core/model/build.gradle.kts
core/model/src/main/AndroidManifest.xml
core/network/build.gradle.kts
core/network/src/main/AndroidManifest.xml
docs/android-build.md
docs/runtime-notes.md
feature/home/build.gradle.kts
feature/home/src/main/AndroidManifest.xml
feature/settings/build.gradle.kts
feature/settings/consumer-rules.pro
feature/settings/src/main/AndroidManifest.xml
gradle/libs.versions.toml
gradle/wrapper/gradle-wrapper.properties
gradle.properties
rust/.cargo/config.toml
rust/Cargo.lock
rust/Cargo.toml
rust/src/arena.rs
rust/src/core/core_state.rs
rust/src/core/io.rs
rust/src/core/lifecycle.rs
rust/src/core/mod.rs
rust/src/core/policy.rs
rust/src/core/process.rs
rust/src/core/reducer.rs
rust/src/core/replay.rs
rust/src/core/result.rs
rust/src/core/scheduler.rs
rust/src/core/state_view.rs
rust/src/core/validation.rs
rust/src/core/verify.rs
rust/src/high_level/addon.rs
rust/src/high_level/addons/mod.rs
rust/src/high_level/addons/preload.rs
rust/src/high_level/android.rs
rust/src/high_level/api.rs
rust/src/high_level/bridge.rs
rust/src/high_level/capability.rs
rust/src/high_level/identity.rs
rust/src/high_level/mod.rs
rust/src/lib.rs
rust/src/low_level/io/buffer.rs
rust/src/low_level/io/drain.rs
rust/src/low_level/io/mod.rs
rust/src/low_level/io/writer.rs
rust/src/low_level/reactor/mod.rs
rust/src/low_level/spawn/mod.rs
rust/src/low_level/sys/mod.rs
rust/src/low_level.rs
rust/src/main.rs
rust/src/mid_level/ipc.rs
rust/src/mid_level/mod.rs
rust/src/runtime.rs
scripts/build-rust-android.sh
settings.gradle.kts
```

## Files

---

## `.github/workflows/build-release.yml`

```yml
name: Build CorePolicyManager (Stable Release)

on:
  push:
    branches: [ main ]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  build:
    runs-on: ubuntu-latest
    environment: Keystore

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Java 21
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '21'
          cache: gradle

      - name: Setup Android SDK
        uses: android-actions/setup-android@v3

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android, armv7-linux-androideabi

      - name: Cache Rust dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            rust/target
          key: ${{ runner.os }}-rust-${{ hashFiles('rust/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-rust-

      - name: Cache Gradle Wrapper & Caches
        uses: actions/cache@v4
        with:
          path: |
            ~/.gradle/caches
            ~/.gradle/wrapper
            .gradle
          key: ${{ runner.os }}-gradle-${{ hashFiles('**/*.gradle*', '**/gradle-wrapper.properties', 'gradle/libs.versions.toml') }}
          restore-keys: |
            ${{ runner.os }}-gradle-

      - name: Install Android Toolchain Components
        run: |
          yes | sdkmanager --licenses >/dev/null || true
          sdkmanager --version
          sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;28.0.13004108"

      - name: Build Rust Daemon for Android
        run: |
          SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
          NDK="$SDK_ROOT/ndk/28.0.13004108"
          
          if [ ! -d "$NDK" ]; then
            echo "NDK not found at $NDK"
            ls -R "$SDK_ROOT/ndk" || true
            exit 1
          fi
          
          export ANDROID_NDK_HOME="$NDK"
          export ANDROID_NDK_ROOT="$NDK"
          export PATH="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"
          
          echo "Using NDK at: $ANDROID_NDK_HOME"
          "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/clang" --version
          
          ./scripts/build-rust-android.sh

      - name: Restore keystore (optional)
        env:
          RELEASE_KEYSTORE_B64: ${{ secrets.RELEASE_KEYSTORE_B64 }}
          RELEASE_KEYSTORE_PASSWORD: ${{ secrets.RELEASE_KEYSTORE_PASSWORD }}
          RELEASE_KEY_ALIAS: ${{ secrets.RELEASE_KEY_ALIAS }}
          RELEASE_KEY_PASSWORD: ${{ secrets.RELEASE_KEY_PASSWORD }}
        run: |
          if [ -z "$RELEASE_KEYSTORE_B64" ]; then
            echo "No keystore provided, using debug/default signing"
            exit 0
          fi

          if [ -z "$RELEASE_KEYSTORE_PASSWORD" ] || [ -z "$RELEASE_KEY_ALIAS" ] || [ -z "$RELEASE_KEY_PASSWORD" ]; then
            echo "Keystore provided but one or more signing secrets are missing."
            exit 1
          fi

          echo "$RELEASE_KEYSTORE_B64" | base64 --decode > release.keystore

          cat <<EOK > keystore.properties
          storeFile=$PWD/release.keystore
          storePassword=$RELEASE_KEYSTORE_PASSWORD
          keyAlias=$RELEASE_KEY_ALIAS
          keyPassword=$RELEASE_KEY_PASSWORD
          EOK

          echo "RELEASE_STORE_FILE=$PWD/release.keystore" >> "$GITHUB_ENV"

      - name: Build release APK
        run: |
          chmod +x ./gradlew
          ./gradlew :app:assembleRelease --stacktrace --no-daemon

      - name: Verify release signing
        if: ${{ env.RELEASE_STORE_FILE != '' }}
        run: |
          APK=$(find app/build/outputs/apk/release -name "*.apk" | head -n 1)
          if [ -z "$APK" ]; then
            echo "No APK found to verify."
            exit 1
          fi
          $ANDROID_HOME/build-tools/36.0.0/apksigner verify --verbose --print-certs "$APK"

      - name: Collect release APKs and set tag
        run: |
          set -euo pipefail

          mkdir -p out
          find app/build/outputs/apk/release -name "*.apk" -print -exec cp {} out/ \;

          if ! ls out/*.apk >/dev/null 2>&1; then
            echo "No APK files were produced."
            exit 1
          fi

          VERSION=$(sed -n 's/.*versionName\s*=\s*"\([^"]*\)".*/\1/p' app/build.gradle.kts | head -n1)
          if [ -z "$VERSION" ]; then
            VERSION="0.0.0"
          fi

          echo "TAG_NAME=v$VERSION" >> "$GITHUB_ENV"
          echo "Collected APKs:"
          ls -lah out/

      - name: Force update tag
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git tag -f "${{ env.TAG_NAME }}"
          git push origin "refs/tags/${{ env.TAG_NAME }}:refs/tags/${{ env.TAG_NAME }}" --force

      - name: Publish GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ env.TAG_NAME }}
          files: out/*.apk
          generate_release_notes: true
          make_latest: true

      - name: Upload APK artifacts
        uses: actions/upload-artifact@v4
        with:
          name: CorePolicyManager-release-apks
          path: out/*.apk
          retention-days: 7

```

---

## `app/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ksp)
}

android {
    namespace = "com.corepolicy.manager"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()

    defaultConfig {
        applicationId = "com.corepolicy.manager"
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
        targetSdk =
            libs.versions.targetSdk
                .get()
                .toInt()
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnit4Runner"
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
        }
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        val javaVersion = JavaVersion.toVersion(libs.versions.java.get())
        sourceCompatibility = javaVersion
        targetCompatibility = javaVersion
    }

    buildFeatures {
        compose = true
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}

kotlin {
    jvmToolchain(
        libs.versions.java
            .get()
            .toInt(),
    )
}

dependencies {
    implementation(project(":feature:home"))
    implementation(project(":feature:settings"))
    implementation(project(":core:designsystem"))
    implementation(project(":core:common"))

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.navigation.compose)

    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)

    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(libs.androidx.hilt.navigation.compose)

    debugImplementation(libs.androidx.compose.ui.tooling)
}

```

---

## `app/proguard-rules.pro`

```pro

```

---

## `app/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <application
        android:name=".CorePolicyApplication"
        android:allowBackup="true"
        android:icon="@mipmap/ic_launcher"
        android:label="CorePolicy"
        android:roundIcon="@mipmap/ic_launcher_round"
        android:supportsRtl="true"
        android:theme="@style/Theme.CorePolicy">
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>

</manifest>

```

---

## `app/src/main/kotlin/com/corepolicy/manager/CorePolicyApplication.kt`

```kt
package com.corepolicy.manager

import android.app.Application
import dagger.hilt.android.HiltAndroidApp

@HiltAndroidApp
class CorePolicyApplication : Application()

```

---

## `app/src/main/kotlin/com/corepolicy/manager/MainActivity.kt`

```kt
package com.corepolicy.manager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.corepolicy.manager.core.designsystem.CorePolicyTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            CorePolicyTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    Surface(
                        modifier =
                            Modifier
                                .fillMaxSize()
                                .padding(innerPadding),
                    ) {
                        Greeting("CorePolicy")
                    }
                }
            }
        }
    }
}

@Composable
fun Greeting(
    name: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = "Hello $name!",
        modifier = modifier,
    )
}

```

---

## `app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@drawable/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
</adaptive-icon>

```

---

## `app/src/main/res/mipmap-anydpi-v26/ic_launcher_round.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@drawable/ic_launcher_background" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
</adaptive-icon>

```

---

## `app/src/main/res/values/themes.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.CorePolicy" parent="Theme.Material3.DayNight.NoActionBar">
        <item name="android:statusBarColor">@android:color/transparent</item>
        <item name="android:navigationBarColor">@android:color/transparent</item>
    </style>
</resources>

```

---

## `build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.jvm) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.hilt) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.spotless)
}

subprojects {
    apply(plugin = "com.diffplug.spotless")
    spotless {
        kotlin {
            target("**/*.kt")
            targetExclude("**/build/**/*.kt")
            ktlint()
        }
        kotlinGradle {
            target("*.gradle.kts")
            ktlint()
        }
    }
}

```

---

## `core/common/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.corepolicy.manager.core.common"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()

    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }

    compileOptions {
        val javaVersion = JavaVersion.toVersion(libs.versions.java.get())
        sourceCompatibility = javaVersion
        targetCompatibility = javaVersion
    }
}

kotlin {
    jvmToolchain(
        libs.versions.java
            .get()
            .toInt(),
    )
}

dependencies {
    implementation(libs.androidx.core.ktx)
}

```

---

## `core/common/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `core/data/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
}
android {
    namespace = "com.corepolicy.manager.core.data"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }
}
dependencies {
    implementation(project(":core:database"))
    implementation(project(":core:network"))
    implementation(project(":core:model"))
    implementation(project(":core:common"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.kotlinx.coroutines.core)
}

```

---

## `core/data/consumer-rules.pro`

```pro

```

---

## `core/data/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `core/database/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.ksp)
}
android {
    namespace = "com.corepolicy.manager.core.database"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }
}
dependencies {
    implementation(project(":core:model"))
    implementation(project(":core:common"))
    implementation(libs.room.runtime)
    implementation(libs.room.ktx)
    ksp(libs.room.compiler)
}

```

---

## `core/database/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `core/designsystem/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.corepolicy.manager.core.designsystem"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()

    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }

    compileOptions {
        val javaVersion = JavaVersion.toVersion(libs.versions.java.get())
        sourceCompatibility = javaVersion
        targetCompatibility = javaVersion
    }
}

kotlin {
    jvmToolchain(
        libs.versions.java
            .get()
            .toInt(),
    )
}

dependencies {
    implementation(libs.androidx.core.ktx)
}

```

---

## `core/designsystem/consumer-rules.pro`

```pro

```

---

## `core/designsystem/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `core/designsystem/src/main/kotlin/com/corepolicy/manager/core/designsystem/Theme.kt`

```kt
package com.corepolicy.manager.core.designsystem

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

private val DarkColorScheme = darkColorScheme()
private val LightColorScheme = lightColorScheme()

@Composable
fun CorePolicyTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    MaterialTheme(
        colorScheme = colorScheme,
        content = content,
    )
}

```

---

## `core/model/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.corepolicy.manager.core.model"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()

    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }

    compileOptions {
        val javaVersion = JavaVersion.toVersion(libs.versions.java.get())
        sourceCompatibility = javaVersion
        targetCompatibility = javaVersion
    }
}

kotlin {
    jvmToolchain(
        libs.versions.java
            .get()
            .toInt(),
    )
}

dependencies {
    implementation(libs.androidx.core.ktx)
}

```

---

## `core/model/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `core/network/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
}
android {
    namespace = "com.corepolicy.manager.core.network"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }
}
dependencies {
    implementation(project(":core:model"))
    implementation(project(":core:common"))
    implementation(libs.androidx.core.ktx)
}

```

---

## `core/network/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `docs/android-build.md`

```md
# Building Rust Daemon for Android

This project supports cross-compilation for Android ARM targets using a dedicated build script.

## Prerequisites

1.  **Android NDK**: Ensure you have the Android NDK installed.
2.  **Environment Variables**: Set `ANDROID_NDK_HOME` to your NDK path.
    Also, ensure the NDK LLVM binaries are in your `PATH`:
    ```bash
    export PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
    ```

## Build and Package

Use the provided script to build for all supported targets and package them for the Android app:

```bash
./scripts/build-rust-android.sh
```

### Script Behavior
- **`CARGO_TARGET_DIR`**: The script respects your `CARGO_TARGET_DIR` if set. In CI or if unset, it defaults to a repo-local directory: `rust/target`.
- **Packaging**: It creates the required `jniLibs` structure in the `app` module and copies the binaries as `libcoreshift.so`.
- **Permissions**: It applies `chmod 755` to the packaged binaries.

### Supported Targets
- **ARM64 (arm64-v8a)**: `aarch64-linux-android`
- **ARMv7 (armeabi-v7a)**: `armv7-linux-androideabi`

## Output Artifacts

The final packaged payloads are located at:
- `app/src/main/jniLibs/arm64-v8a/libcoreshift.so`
- `app/src/main/jniLibs/armeabi-v7a/libcoreshift.so`

**Important**: These are **executable ELF PIE payloads**, not JNI shared libraries. They are named with `.so` to ensure the Android Package Manager extracts them upon installation.

## CI Configuration
The CI environment automatically installs the required Rust targets and uses the repo-local `rust/target` directory for deterministic builds.

```

---

## `docs/runtime-notes.md`

```md
# Rust Daemon Runtime Notes

## Packaging Strategy
The Rust daemon is packaged as `libcoreshift.so` within the `jniLibs` directory. 
**Note**: This is a packaged **executable binary (ELF PIE)**, not a JNI shared library. 
It is named with a `.so` prefix to ensure the Android Package Manager (PM) extracts it to the application's native library directory upon installation.

## Runtime Execution
Because modern Android versions restrict the execution of binaries directly from the APK or certain storage locations, the Android application must follow these steps to run the daemon:

1.  **Locate the Binary**: Find the extracted `libcoreshift.so` in the app's `nativeLibraryDir`.
2.  **Copy to Internal Storage**: Copy the file from the native library directory to the app's internal files directory (e.g., `/data/data/<package_name>/files/coreshift`).
3.  **Set Permissions**: Apply execute permissions to the copied file:
    ```java
    file.setExecutable(true, true); // Equivalent to chmod 700
    ```
4.  **Execute**: Start the process using `ProcessBuilder` or `Runtime.exec()`.

## Target Architectures
- **arm64-v8a**: `lib/arm64-v8a/libcoreshift.so`
- **armeabi-v7a**: `lib/armeabi-v7a/libcoreshift.so`

```

---

## `feature/home/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ksp)
}
android {
    namespace = "com.corepolicy.manager.feature.home"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }
    buildFeatures { compose = true }
}
dependencies {
    implementation(project(":core:data"))
    implementation(project(":core:designsystem"))
    implementation(project(":core:model"))
    implementation(project(":core:common"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
}

```

---

## `feature/home/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `feature/settings/build.gradle.kts`

```kts
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ksp)
}
android {
    namespace = "com.corepolicy.manager.feature.settings"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    defaultConfig {
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
    }
    buildFeatures { compose = true }
}
dependencies {
    implementation(project(":core:data"))
    implementation(project(":core:designsystem"))
    implementation(project(":core:model"))
    implementation(project(":core:common"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
}

```

---

## `feature/settings/consumer-rules.pro`

```pro

```

---

## `feature/settings/src/main/AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
</manifest>

```

---

## `gradle/libs.versions.toml`

```toml
[versions]
# Core Build Tools
agp = "9.1.1"
kotlin = "2.3.20"
ksp = "2.3.6"
gradle = "9.4.1"

# SDK Versions
compileSdk = "36"
minSdk = "26"
targetSdk = "36"

# Java
java = "25"

# Jetpack Foundation
androidxCore = "1.18.0"
androidxLifecycle = "2.8.7"
androidxActivity = "1.13.0"
androidxNavigation = "2.9.0"
androidxWork = "2.11.2"
androidxDatastore = "1.2.1"
androidxHilt = "1.3.0"

# Compose
composeBom = "2026.03.01"

# Architecture Components
hilt = "2.59.2"
room = "2.8.4"
kotlinxCoroutines = "1.10.2"

# Quality Tools
spotless = "7.0.2"

[libraries]
# AndroidX Core
androidx-core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "androidxCore" }
androidx-lifecycle-runtime-ktx = { group = "androidx.lifecycle", name = "lifecycle-runtime-ktx", version.ref = "androidxLifecycle" }
androidx-activity-compose = { group = "androidx.activity", name = "activity-compose", version.ref = "androidxActivity" }
androidx-navigation-compose = { group = "androidx.navigation", name = "navigation-compose", version.ref = "androidxNavigation" }
androidx-work-runtime-ktx = { group = "androidx.work", name = "work-runtime-ktx", version.ref = "androidxWork" }
androidx-datastore-preferences = { group = "androidx.datastore", name = "datastore-preferences", version.ref = "androidxDatastore" }

# Compose BOM
androidx-compose-bom = { group = "androidx.compose", name = "compose-bom", version.ref = "composeBom" }
androidx-compose-ui = { group = "androidx.compose.ui", name = "ui" }
androidx-compose-ui-graphics = { group = "androidx.compose.ui", name = "ui-graphics" }
androidx-compose-foundation = { group = "androidx.compose.foundation", name = "foundation" }
androidx-compose-material3 = { group = "androidx.compose.material3", name = "material3" }
androidx-compose-material-icons-extended = { group = "androidx.compose.material", name = "material-icons-extended" }
androidx-compose-ui-tooling-preview = { group = "androidx.compose.ui", name = "ui-tooling-preview" }
androidx-compose-ui-tooling = { group = "androidx.compose.ui", name = "ui-tooling" }

# Hilt
hilt-android = { group = "com.google.dagger", name = "hilt-android", version.ref = "hilt" }
hilt-compiler = { group = "com.google.dagger", name = "hilt-compiler", version.ref = "hilt" }
androidx-hilt-navigation-compose = { group = "androidx.hilt", name = "hilt-navigation-compose", version.ref = "androidxHilt" }
androidx-hilt-work = { group = "androidx.hilt", name = "hilt-work", version.ref = "androidxHilt" }

# Room
room-runtime = { group = "androidx.room", name = "room-runtime", version.ref = "room" }
room-ktx = { group = "androidx.room", name = "room-ktx", version.ref = "room" }
room-compiler = { group = "androidx.room", name = "room-compiler", version.ref = "room" }

# Coroutines
kotlinx-coroutines-core = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-core", version.ref = "kotlinxCoroutines" }
kotlinx-coroutines-android = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-android", version.ref = "kotlinxCoroutines" }

# Quality Tools
spotless-gradlePlugin = { group = "com.diffplug.spotless", name = "spotless-plugin-gradle", version.ref = "spotless" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
android-library = { id = "com.android.library", version.ref = "agp" }
kotlin-jvm = { id = "org.jetbrains.kotlin.jvm", version.ref = "kotlin" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
ksp = { id = "com.google.devtools.ksp", version.ref = "ksp" }
hilt = { id = "com.google.dagger.hilt.android", version.ref = "hilt" }
spotless = { id = "com.diffplug.spotless", version.ref = "spotless" }

```

---

## `gradle/wrapper/gradle-wrapper.properties`

```properties
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-9.4.1-bin.zip
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists

```

---

## `gradle.properties`

```properties
org.gradle.jvmargs=-Xmx4096m -Dfile.encoding=UTF-8
android.useAndroidX=true
android.nonTransitiveRClass=true
org.gradle.caching=true
org.gradle.parallel=true
org.gradle.configuration-cache=true
org.gradle.configuration-cache.problems=warn

```

---

## `rust/.cargo/config.toml`

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android28-clang"

[target.armv7-linux-androideabi]
linker = "armv7a-linux-androideabi28-clang"

```

---

## `rust/Cargo.lock`

```lock
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "CoreShift"
version = "0.1.0"
dependencies = [
 "arrayvec",
 "bincode",
 "blake3",
 "bytemuck",
 "lazy_static",
 "libc",
 "serde",
 "serde_json",
 "smallvec",
]

[[package]]
name = "arrayref"
version = "0.3.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "76a2e8124351fda1ef8aaaa3bbd7ebbcb486bbcd4225aca0aa0d84bb2db8fecb"

[[package]]
name = "arrayvec"
version = "0.7.6"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "7c02d123df017efcdfbd739ef81735b36c5ba83ec3c59c80a9d7ecc718f92e50"

[[package]]
name = "bincode"
version = "1.3.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b1f45e9417d87227c7a56d22e471c6206462cba514c7590c09aff4cf6d1ddcad"
dependencies = [
 "serde",
]

[[package]]
name = "blake3"
version = "1.8.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "4d2d5991425dfd0785aed03aedcf0b321d61975c9b5b3689c774a2610ae0b51e"
dependencies = [
 "arrayref",
 "arrayvec",
 "cc",
 "cfg-if",
 "constant_time_eq",
 "cpufeatures",
]

[[package]]
name = "bytemuck"
version = "1.25.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "c8efb64bd706a16a1bdde310ae86b351e4d21550d98d056f22f8a7f7a2183fec"

[[package]]
name = "cc"
version = "1.2.59"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b7a4d3ec6524d28a329fc53654bbadc9bdd7b0431f5d65f1a56ffb28a1ee5283"
dependencies = [
 "find-msvc-tools",
 "shlex",
]

[[package]]
name = "cfg-if"
version = "1.0.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9330f8b2ff13f34540b44e946ef35111825727b38d33286ef986142615121801"

[[package]]
name = "constant_time_eq"
version = "0.4.2"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "3d52eff69cd5e647efe296129160853a42795992097e8af39800e1060caeea9b"

[[package]]
name = "cpufeatures"
version = "0.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8b2a41393f66f16b0823bb79094d54ac5fbd34ab292ddafb9a0456ac9f87d201"
dependencies = [
 "libc",
]

[[package]]
name = "find-msvc-tools"
version = "0.1.9"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "5baebc0774151f905a1a2cc41989300b1e6fbb29aff0ceffa1064fdd3088d582"

[[package]]
name = "itoa"
version = "1.0.18"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682"

[[package]]
name = "lazy_static"
version = "1.5.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "bbd2bcb4c963f2ddae06a2efc7e9f3591312473c50c6685e1f298068316e66fe"

[[package]]
name = "libc"
version = "0.2.184"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "48f5d2a454e16a5ea0f4ced81bd44e4cfc7bd3a507b61887c99fd3538b28e4af"

[[package]]
name = "memchr"
version = "2.8.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "f8ca58f447f06ed17d5fc4043ce1b10dd205e060fb3ce5b979b8ed8e59ff3f79"

[[package]]
name = "proc-macro2"
version = "1.0.106"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8fd00f0bb2e90d81d1044c2b32617f68fcb9fa3bb7640c23e9c748e53fb30934"
dependencies = [
 "unicode-ident",
]

[[package]]
name = "quote"
version = "1.0.45"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41f2619966050689382d2b44f664f4bc593e129785a36d6ee376ddf37259b924"
dependencies = [
 "proc-macro2",
]

[[package]]
name = "serde"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "9a8e94ea7f378bd32cbbd37198a4a91436180c5bb472411e48b5ec2e2124ae9e"
dependencies = [
 "serde_core",
 "serde_derive",
]

[[package]]
name = "serde_core"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "41d385c7d4ca58e59fc732af25c3983b67ac852c1a25000afe1175de458b67ad"
dependencies = [
 "serde_derive",
]

[[package]]
name = "serde_derive"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d540f220d3187173da220f885ab66608367b6574e925011a9353e4badda91d79"
dependencies = [
 "proc-macro2",
 "quote",
 "syn",
]

[[package]]
name = "serde_json"
version = "1.0.149"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "83fc039473c5595ace860d8c4fafa220ff474b3fc6bfdb4293327f1a37e94d86"
dependencies = [
 "itoa",
 "memchr",
 "serde",
 "serde_core",
 "zmij",
]

[[package]]
name = "shlex"
version = "1.3.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0fda2ff0d084019ba4d7c6f371c95d8fd75ce3524c3cb8fb653a3023f6323e64"

[[package]]
name = "smallvec"
version = "1.15.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "67b1b7a3b5fe4f1376887184045fcf45c69e92af734b7aaddc05fb777b6fbd03"

[[package]]
name = "syn"
version = "2.0.117"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e665b8803e7b1d2a727f4023456bbbbe74da67099c585258af0ad9c5013b9b99"
dependencies = [
 "proc-macro2",
 "quote",
 "unicode-ident",
]

[[package]]
name = "unicode-ident"
version = "1.0.24"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "e6e4313cd5fcd3dad5cafa179702e2b244f760991f45397d14d4ebf38247da75"

[[package]]
name = "zmij"
version = "1.0.21"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "b8848ee67ecc8aedbaf3e4122217aff892639231befc6a1b58d29fff4c2cabaa"

```

---

## `rust/Cargo.toml`

```toml
[package]
name = "CoreShift"
version = "0.1.0"
edition = "2024"

[lib]
path = "src/lib.rs"

[features]
default = ["std"]
std = ["blake3/std"]
pure = ["blake3/pure"]

[dependencies]
blake3 = { version = "1.8", default-features = false }
arrayvec = "0.7"
bytemuck = { version = "1.2", default-features = false }
lazy_static = { version = "1.5", default-features = false }
libc = { version = "0.2", default-features = false }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
bincode = "1.3.3"
smallvec = "1.15.1"


```

---

## `rust/src/arena.rs`

```rs
enum Slot<T> {
    Occupied {
        generation: u32,
        data: T,
    },
    Free {
        generation: u32,
        next: Option<usize>,
    },
}

// O(1) storage mapping index and generation.
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free_head: Option<usize>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
        }
    }

    pub fn insert(&mut self, value: T) -> (u32, u32) {
        if let Some(index) = self.free_head {
            if let Slot::Free { generation, next } = self.slots[index] {
                self.free_head = next;
                self.slots[index] = Slot::Occupied {
                    generation,
                    data: value,
                };
                (index as u32, generation)
            } else {
                unreachable!("Free list is corrupted");
            }
        } else {
            let index = self.slots.len();
            let generation = 1;
            self.slots.push(Slot::Occupied {
                generation,
                data: value,
            });
            (index as u32, generation)
        }
    }

    pub fn get(&self, index: u32, generation: u32) -> Option<&T> {
        if let Some(Slot::Occupied {
            generation: g,
            data,
        }) = self.slots.get(index as usize)
            && *g == generation
        {
            return Some(data);
        }
        None
    }

    pub fn get_mut(&mut self, index: u32, generation: u32) -> Option<&mut T> {
        if let Some(Slot::Occupied {
            generation: g,
            data,
        }) = self.slots.get_mut(index as usize)
            && *g == generation
        {
            return Some(data);
        }
        None
    }

    pub fn remove(&mut self, index: u32, generation: u32) -> Option<T> {
        let valid =
            if let Some(Slot::Occupied { generation: g, .. }) = self.slots.get(index as usize) {
                *g == generation
            } else {
                false
            };
        if valid {
            let next_gen = generation
                .checked_add(1)
                .expect("Arena generation overflow");
            let slot = std::mem::replace(
                &mut self.slots[index as usize],
                Slot::Free {
                    generation: next_gen,
                    next: self.free_head,
                },
            );
            self.free_head = Some(index as usize);
            if let Slot::Occupied { data, .. } = slot {
                return Some(data);
            } else {
                unreachable!();
            }
        }
        None
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (u32, u32, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, slot)| {
            if let Slot::Occupied { generation, data } = slot {
                Some((i as u32, *generation, data))
            } else {
                None
            }
        })
    }
}

```

---

## `rust/src/core/core_state.rs`

```rs
use crate::core::{IoHandle, JobHandle, JobRuntime, JobState, ProcessHandle};
use crate::arena::Arena;
use std::collections::HashMap;

const K: u64 = 0x9E3779B97F4A7C15;
const PROCESS_COUNT_KEY: u64 = 0xA1;
const IO_COUNT_KEY: u64 = 0xB2;

#[inline]
pub fn mix(a: u64, b: u64) -> u64 {
    a.wrapping_mul(K) ^ b
}

#[inline]
pub fn hash_job(job: &JobState) -> u64 {
    let mut h = 0;

    h ^= mix(1, job.id);
    h ^= mix(2, job.owner as u64);
    h ^= mix(3, job.lifecycle as u64);
    h ^= mix(4, job.io_state as u64);

    if job.timed_out {
        h ^= mix(5, 1);
    }

    if let Some(p) = job.process {
        h ^= mix(6, p.index as u64);
    }

    if let Some(io) = job.io {
        h ^= mix(7, io.index as u64);
    }

    h
}

pub struct CoreState {
    pub jobs: Arena<JobState>,
    pub(crate) job_id_map: HashMap<u64, JobHandle>,
    pub runtime: Vec<Option<JobRuntime>>,
    pub process_index: Vec<Option<JobHandle>>,
    pub io_index: Vec<Option<JobHandle>>,
    pub process_count: usize,
    pub io_count: usize,
    pub hash: u64,
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreState {
    pub fn new() -> Self {
        Self {
            jobs: Arena::new(),
            job_id_map: HashMap::new(),
            runtime: Vec::new(),
            process_index: Vec::new(),
            io_index: Vec::new(),
            process_count: 0,
            io_count: 0,
            hash: 0,
        }
    }

    #[inline]
    pub fn job_handle(&self, id: u64) -> Option<JobHandle> {
        self.job_id_map.get(&id).copied()
    }

    #[inline]
    pub fn job(&self, h: JobHandle) -> &JobState {
        let job = self.jobs.get(h.index, h.generation);
        debug_assert!(job.is_some(), "Job handle dangling in jobs arena");
        job.unwrap()
    }

    #[inline]
    pub fn job_mut(&mut self, h: JobHandle) -> &mut JobState {
        let job = self.jobs.get_mut(h.index, h.generation);
        debug_assert!(job.is_some(), "Job handle dangling in jobs arena (mut)");
        job.unwrap()
    }

    #[inline]
    pub fn runtime(&self, h: JobHandle) -> &JobRuntime {
        debug_assert!((h.index as usize) < self.runtime.len(), "Runtime missing for job");
        self.runtime[h.index as usize].as_ref().unwrap()
    }

    #[inline]
    pub fn runtime_mut(&mut self, h: JobHandle) -> &mut JobRuntime {
        debug_assert!((h.index as usize) < self.runtime.len(), "Runtime missing for job (mut)");
        self.runtime[h.index as usize].as_mut().unwrap()
    }

    #[inline]
    pub fn job_by_process(&self, p: ProcessHandle) -> Option<JobHandle> {
        if (p.index as usize) < self.process_index.len() {
            self.process_index[p.index as usize]
        } else {
            None
        }
    }

    #[inline]
    pub fn job_by_io(&self, io: IoHandle) -> Option<JobHandle> {
        if (io.index as usize) < self.io_index.len() {
            self.io_index[io.index as usize]
        } else {
            None
        }
    }


    #[inline]
    pub fn remove_job(&mut self, id: u64) -> Option<JobState> {
        if let Some(h) = self.job_id_map.remove(&id) {
            if let Some(job) = self.jobs.remove(h.index, h.generation) {
                // XOR out job state from hash
                self.hash ^= mix(id, hash_job(&job));

                if let Some(p) = job.process {
                    self.remove_process_index(p);
                }
                if let Some(io) = job.io {
                    self.remove_io_index(io);
                }

                debug_assert!(self.runtime[h.index as usize].is_some(), "Runtime missing during cleanup");
                self.runtime[h.index as usize].take();

                return Some(job);
            }
        }
        None
    }

    #[inline]
    pub fn insert_job(&mut self, id: u64, owner: u32, exec: crate::core::ExecSpec, policy: crate::core::ExecPolicy) {
        if self.job_id_map.contains_key(&id) { return; }

        let (index, generation) = self.jobs.insert(JobState {
            id, owner, exec, policy, process: None, io: None, timed_out: false,
            lifecycle: crate::core::JobLifecycle::Submitted, io_state: crate::core::JobIoState::Pending,
        });

        let handle = crate::core::JobHandle {
            index, generation, _marker: std::marker::PhantomData,
        };

        self.job_id_map.insert(id, handle);

        if self.runtime.len() <= index as usize {
            self.runtime.resize(index as usize + 1, None);
        }
        self.runtime[index as usize] = Some(JobRuntime { process: None, io: None });

        let job = self.jobs.get(index, generation).unwrap();
        self.hash ^= mix(id, hash_job(job));
    }
    // Additional helpers for indexing mutations
    #[inline]
    pub fn remove_process_index(&mut self, p: ProcessHandle) {
        if (p.index as usize) < self.process_index.len() {
            if let Some(h) = self.process_index[p.index as usize] {
                // XOR out old value
                self.hash ^= mix(p.index as u64, h.index as u64);

                // Update count and XOR its change
                self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
                self.process_count -= 1;
                self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);

                self.process_index[p.index as usize] = None;
            }
        }
    }

    #[inline]
    pub fn insert_process_index(&mut self, p: ProcessHandle, h: JobHandle) {
        if self.process_index.len() <= p.index as usize {
            self.process_index.resize(p.index as usize + 1, None);
        }

        // If there's an existing handle, XOR it out first
        if let Some(old) = self.process_index[p.index as usize] {
            self.hash ^= mix(p.index as u64, old.index as u64);
        } else {
            // Updating count since it was None
            self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
            self.process_count += 1;
            self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
        }

        self.process_index[p.index as usize] = Some(h);

        // XOR in new value
        self.hash ^= mix(p.index as u64, h.index as u64);
    }

    #[inline]
    pub fn remove_io_index(&mut self, io: IoHandle) {
        if (io.index as usize) < self.io_index.len() {
            if let Some(h) = self.io_index[io.index as usize] {
                // XOR out old value
                self.hash ^= mix(io.index as u64, h.index as u64);

                // Update count and XOR its change
                self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
                self.io_count -= 1;
                self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);

                self.io_index[io.index as usize] = None;
            }
        }
    }

    #[inline]
    pub fn insert_io_index(&mut self, io: IoHandle, h: JobHandle) {
        if self.io_index.len() <= io.index as usize {
            self.io_index.resize(io.index as usize + 1, None);
        }

        // If there's an existing handle, XOR it out first
        if let Some(old) = self.io_index[io.index as usize] {
            self.hash ^= mix(io.index as u64, old.index as u64);
        } else {
            // Updating count since it was None
            self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
            self.io_count += 1;
            self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
        }

        self.io_index[io.index as usize] = Some(h);

        // XOR in new value
        self.hash ^= mix(io.index as u64, h.index as u64);
    }
}
```

---

## `rust/src/core/io.rs`

```rs
use crate::core::{Action, Event, IoStream, Module};

pub struct IoModule;

impl Module for IoModule {
    fn handle(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        _action: &Action,
    ) -> Vec<Action> {
        Vec::new()
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::ProcessStarted { io, .. } => {
                // Here the core module issues Watch actions purely via intent.
                // It doesn't know about `DrainState` or slots anymore.
                // We request watching for all default streams.
                // EffectExecutor will decide if the stream actually exists or needs a watch based on its internal DrainState.
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stdout,
                });
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stderr,
                });
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stdin,
                });
            }
            Event::IoReady {
                io,
                readable: _,
                writable: _,
                error: _,
                ..
            } => {
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::SetJobIoState {
                        id: job.id,
                        state: crate::core::JobIoState::Active,
                    });
                    actions.push(Action::PerformIo { io: *io });
                }
            }
            Event::IoClosed { io } => {
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::SetJobIoState {
                        id: job.id,
                        state: crate::core::JobIoState::Closed,
                    });
                }
            }
            Event::WatchStreamFailed { io, err } => {
                actions.push(Action::HandleIoFailure {
                    io: *io,
                    reason: err.clone(),
                });
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            Event::IoFailed { io, reason } => {
                actions.push(Action::HandleIoFailure {
                    io: *io,
                    reason: reason.clone(),
                });
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            _ => {}
        }
        actions
    }
}

```

---

## `rust/src/core/lifecycle.rs`

```rs
use crate::core::{Action, Event, JobLifecycle, Module};

pub struct LifecycleModule;

impl Module for LifecycleModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match action {
            Action::Admitted { id } => {
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Admitted,
                });
                actions.push(Action::StartProcess { id: *id });
            }
            Action::SignalProcess { process, signal } => {
                if let Some(job) = state.job_by_process(*process) {
                    if *signal == crate::core::ControlSignal::GracefulStop {
                        actions.push(Action::TimeoutReached { id: job.id });
                        actions.push(Action::SetLifecycle {
                            id: job.id,
                            state: JobLifecycle::Terminating,
                        });
                    } else if *signal == crate::core::ControlSignal::ForceKill {
                        actions.push(Action::SetLifecycle {
                            id: job.id,
                            state: JobLifecycle::Killed,
                        });
                    }
                }
            }
            Action::SetJobIoState {
                id,
                state: io_state,
            } => {
                if *io_state == crate::core::JobIoState::Closed
                    && let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::PollProcess { process });
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(job) = state.job(*id)
                    && job.lifecycle == JobLifecycle::Running
                {
                    actions.push(Action::SetLifecycle {
                        id: *id,
                        state: JobLifecycle::Terminating,
                    });
                    if let Some(process) = job.process {
                        actions.push(Action::SignalProcess {
                            process,
                            signal: crate::core::ControlSignal::GracefulStop,
                        });
                    }
                }
            }
            Action::KillDeadlineReached { id } => {
                if let Some(job) = state.job(*id)
                    && job.lifecycle != JobLifecycle::Finished
                    && job.lifecycle != JobLifecycle::Killed
                {
                    actions.push(Action::SetLifecycle {
                        id: *id,
                        state: JobLifecycle::Killed,
                    });
                    if let Some(process) = job.process {
                        actions.push(Action::SignalProcess {
                            process,
                            signal: crate::core::ControlSignal::ForceKill,
                        });
                    }
                }
            }
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::ProcessStarted {
                id,
                process: process_handle,
                io: io_handle,
            } => {
                actions.push(Action::AssignProcess {
                    id: *id,
                    process: *process_handle,
                });
                actions.push(Action::AssignIo {
                    id: *id,
                    io: *io_handle,
                });
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Running,
                });
                actions.push(Action::Started { id: *id });
            }
            Event::ProcessSpawnFailed { id, err: _ } => {
                let job = state.job(*id);
                let owner = job.as_ref().map(|j| j.owner).unwrap_or(0);
                let was_submitted = job
                    .as_ref()
                    .map(|j| j.lifecycle == crate::core::JobLifecycle::Submitted)
                    .unwrap_or(false);
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Finished,
                });
                actions.push(Action::Rejected {
                    id: *id,
                    owner,
                    was_submitted,
                });
            }
            _ => {}
        }
        actions
    }
}

```

---

## `rust/src/core/mod.rs`

```rs
use serde::{Deserialize, Serialize};
pub mod policy;
pub mod validation;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CancelPolicy {
    None,
    Graceful,
    Kill,
}

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessTag;

#[derive(Debug, Serialize, Deserialize)]
pub struct IoTag;

#[derive(Debug, Serialize, Deserialize)]
pub struct JobTag;
pub type JobHandle = Handle<JobTag>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Handle<T> {
    pub index: u32,
    pub generation: u32,
    #[serde(skip, default)]
    pub _marker: PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

pub type ProcessHandle = Handle<ProcessTag>;
pub type IoHandle = Handle<IoTag>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecSpec {
    pub argv: Vec<String>,
    pub stdin: Option<Vec<u8>>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub max_output: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPolicy {
    pub timeout_ms: Option<u32>,
    pub kill_grace_ms: u32,
    pub cancel: CancelPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecResult {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ExecError {
    SpawnFailed,
    RuntimeError,
    Internal(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecOutcome {
    pub id: u64,
    pub result: Result<ExecResult, ExecError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub enum LogEvent {
    Submit { id: u64 },
    Spawn { id: u64, pid: i32 },
    Cancel { id: u64 },
    ForceKill { id: u64 },
    Exit { id: u64, status: Option<i32> },
    Timeout { id: u64 },
    Error { id: u64, err: String },

    TickStart,
    Observability { queue_len: usize, actions_processed: usize, dropped: usize },
    TickEnd,
    AddonReceived,
    AddonTranslated,
    AddonDropped,
    ActionDispatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlSignal {
    GracefulStop,
    ForceKill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemService {
    ResolveIdentity,
    ResolveDirectory,
    DiscoverPaths,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Event {
    Tick,
    ProcessStarted {
        id: u64,
        process: ProcessHandle,
        io: IoHandle,
    },
    ProcessSpawnFailed {
        id: u64,
        err: String,
    },
    ProcessExited {
        process: ProcessHandle,
        status: Option<i32>,
    },
    IoReady {
        io: IoHandle,
        stream: IoStream,
        readable: bool,
        writable: bool,
        error: bool,
    },
    IoClosed {
        io: IoHandle,
    },
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,
    AddonCompleted {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    SystemResponse {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
    SystemFailure {
        request_id: u64,
        kind: SystemService,
        err: String,
    },
    TimeAdvanced(u64),

    // Internal Core/Addon Errors
    AddonFailed {
        addon_id: u32,
        key: String,
        err: String,
    },
    WatchStreamFailed {
        io: IoHandle,
        err: String,
    },
    DroppedAction {
        kind: ActionKind,
    },
    KillProcessFailed {
        process: ProcessHandle,
        err: String,
    },
    ReactorError {
        err: String,
    },
    IoFailed {
        io: IoHandle,
        reason: String,
    },
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActionKind {
    Submit = 0,
    Admitted,
    Rejected,
    Started,
    Controlled,
    Finished,
    QueryResult,
    SetJobIoState,
    AssignProcess,
    AssignIo,
    SetLifecycle,
    StartProcess,
    SignalProcess,
    PollProcess,
    PerformIo,
    RegisterInterest,
    RemoveInterest,
    EmitLog,
    Control,
    Query,
    TimeoutReached,
    KillDeadlineReached,
    ForegroundChanged,
    PackagesChanged,
    SystemRequest,
    AddonTask,
    AddonEvent,
    CleanupJob,
    TrackTimeout,
    UntrackTimeout,
    UpdateTimeoutState,
    AdvanceTime,
    HandleAddonFailure,
    HandleSystemFailure,
    HandleProcessFailure,
    HandleIoFailure,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub command: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Intent {
    Submit {
        id: u64,
        owner: u32,
        job: JobRequest,
    },
    Control {
        id: u64,
        signal: ControlSignal,
    },
    Query {
        id: u64,
    },
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
}

pub fn validate_intent(intent: &Intent) -> bool {
    match intent {
        Intent::Submit { job, .. } => !job.command.is_empty() && job.command.len() < 64,
        Intent::Control { .. } => true,
        Intent::Query { .. } => true,
        Intent::ForegroundChanged { pid } => *pid > 0,
        Intent::PackagesChanged => true,
        Intent::SystemRequest { .. } => true,
        Intent::AddonTask { .. } => true,
    }
}

fn default_policy(cmd: &[String]) -> ExecPolicy {
    if cmd.first().map(|s| s.as_str()) == Some("dumpsys") {
        ExecPolicy {
            timeout_ms: Some(3000),
            kill_grace_ms: 500,
            cancel: CancelPolicy::Kill,
        }
    } else {
        ExecPolicy {
            timeout_ms: Some(1000),
            kill_grace_ms: 300,
            cancel: CancelPolicy::Kill,
        }
    }
}

pub fn expand_intent(intent: Intent, now: u64) -> Vec<Action> {
    match intent {
        Intent::Submit { id, owner, job } => {
            let mut actions = vec![Action::Submit {
                id,
                owner,
                job: job.clone(),
            }];

            let policy = default_policy(&job.command);
            if let Some(timeout_ms) = policy.timeout_ms {
                actions.push(Action::TrackTimeout {
                    id,
                    deadline: now + (timeout_ms as u64),
                    kill_grace_ms: policy.kill_grace_ms,
                });
            }

            actions
        }
        Intent::Control { id, signal } => {
            vec![Action::Control { id, signal }]
        }
        Intent::Query { id } => {
            vec![Action::Query { id }]
        }
        Intent::ForegroundChanged { pid } => {
            vec![Action::ForegroundChanged { pid }]
        }
        Intent::PackagesChanged => {
            vec![Action::PackagesChanged]
        }
        Intent::SystemRequest { request_id, kind, payload } => {
            vec![Action::SystemRequest { request_id, kind, payload }]
        }
        Intent::AddonTask { addon_id, key, payload } => {
            vec![Action::AddonTask { addon_id, key, payload }]
        }
    }
}

#[derive(Clone)]
pub enum Action {
    AdvanceTime {
        delta: u64,
    },
    Submit {
        id: u64,
        owner: u32,
        job: JobRequest,
    },

    // State Transitions
    Admitted {
        id: u64,
    },
    Rejected {
        id: u64,
        owner: u32,
        was_submitted: bool,
    },
    Started {
        id: u64,
    },
    Controlled {
        id: u64,
    },
    Finished {
        id: u64,
        owner: u32,
        was_submitted: bool,
        result: Result<ExecResult, ExecError>,
    },
    QueryResult {
        id: u64,
        result: Option<ExecOutcome>,
    },

    SetJobIoState {
        id: u64,
        state: JobIoState,
    },
    AssignProcess {
        id: u64,
        process: ProcessHandle,
    },
    AssignIo {
        id: u64,
        io: IoHandle,
    },
    SetLifecycle {
        id: u64,
        state: JobLifecycle,
    },

    // Semantic Intents
    StartProcess {
        id: u64,
    },
    SignalProcess {
        process: ProcessHandle,
        signal: ControlSignal,
    },
    PollProcess {
        process: ProcessHandle,
    },
    PerformIo {
        io: IoHandle,
    },
    RegisterInterest {
        io: IoHandle,
        stream: IoStream,
    },
    RemoveInterest {
        io: IoHandle,
        stream: IoStream,
    },
    EmitLog {
        owner: u32,
        level: LogLevel,
        event: LogEvent,
    },

    // Input actions
    Control {
        id: u64,
        signal: ControlSignal,
    },
    Query {
        id: u64,
    },
    TimeoutReached {
        id: u64,
    },
    KillDeadlineReached {
        id: u64,
    },

    // Low-level observation triggers
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,

    // Generic System Call Protocol
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },

    // Generic Addon Extension Points
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    AddonEvent {
        addon_id: u32,
        key: String,
    },
    CleanupJob {
        id: u64,
    },

    // Timeout intent tracking (pure actions)
    TrackTimeout {
        id: u64,
        deadline: u64,
        kill_grace_ms: u32,
    },
    UntrackTimeout {
        id: u64,
    },
    UpdateTimeoutState {
        id: u64,
        state: TimeoutState,
    },

    HandleSystemFailure {
        request_id: u64,
        kind: SystemService,
        err: String,
    },
    HandleAddonFailure {
        addon_id: u32,
        key: String,
        err: String,
    },
    HandleProcessFailure {
        process: ProcessHandle,
        err: String,
    },
    HandleIoFailure {
        io: IoHandle,
        reason: String,
    },
}

impl Action {
    pub fn kind(&self) -> ActionKind {
        match self {
            Action::AdvanceTime { .. } => ActionKind::AdvanceTime,
            Action::Submit { .. } => ActionKind::Submit,
            Action::Admitted { .. } => ActionKind::Admitted,
            Action::Rejected { .. } => ActionKind::Rejected,
            Action::Started { .. } => ActionKind::Started,
            Action::Controlled { .. } => ActionKind::Controlled,
            Action::Finished { .. } => ActionKind::Finished,
            Action::QueryResult { .. } => ActionKind::QueryResult,
            Action::SetJobIoState { .. } => ActionKind::SetJobIoState,
            Action::AssignProcess { .. } => ActionKind::AssignProcess,
            Action::AssignIo { .. } => ActionKind::AssignIo,
            Action::SetLifecycle { .. } => ActionKind::SetLifecycle,
            Action::StartProcess { .. } => ActionKind::StartProcess,
            Action::SignalProcess { .. } => ActionKind::SignalProcess,
            Action::PollProcess { .. } => ActionKind::PollProcess,
            Action::PerformIo { .. } => ActionKind::PerformIo,
            Action::RegisterInterest { .. } => ActionKind::RegisterInterest,
            Action::RemoveInterest { .. } => ActionKind::RemoveInterest,
            Action::EmitLog { .. } => ActionKind::EmitLog,
            Action::Control { .. } => ActionKind::Control,
            Action::Query { .. } => ActionKind::Query,
            Action::TimeoutReached { .. } => ActionKind::TimeoutReached,
            Action::KillDeadlineReached { .. } => ActionKind::KillDeadlineReached,
            Action::ForegroundChanged { .. } => ActionKind::ForegroundChanged,
            Action::PackagesChanged => ActionKind::PackagesChanged,
            Action::SystemRequest { .. } => ActionKind::SystemRequest,
            Action::AddonTask { .. } => ActionKind::AddonTask,
            Action::AddonEvent { .. } => ActionKind::AddonEvent,
            Action::CleanupJob { .. } => ActionKind::CleanupJob,
            Action::TrackTimeout { .. } => ActionKind::TrackTimeout,
            Action::UntrackTimeout { .. } => ActionKind::UntrackTimeout,
            Action::UpdateTimeoutState { .. } => ActionKind::UpdateTimeoutState,
            Action::HandleSystemFailure { .. } => ActionKind::HandleSystemFailure,
            Action::HandleAddonFailure { .. } => ActionKind::HandleAddonFailure,
            Action::HandleProcessFailure { .. } => ActionKind::HandleProcessFailure,
            Action::HandleIoFailure { .. } => ActionKind::HandleIoFailure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,   // lifecycle, failure, control
    Normal,     // core flow, intent
    Background, // warmup, probes, logs
}

impl Action {
    pub fn priority(&self) -> Priority {
        match self {
            Action::Control { .. }
            | Action::SignalProcess { .. }
            | Action::KillDeadlineReached { .. }
            | Action::TimeoutReached { .. }
            | Action::Admitted { .. }
            | Action::Rejected { .. }
            | Action::Started { .. }
            | Action::Controlled { .. }
            | Action::Finished { .. }
            | Action::SetJobIoState { .. }
            | Action::AssignProcess { .. }
            | Action::AssignIo { .. }
            | Action::SetLifecycle { .. }
            | Action::UpdateTimeoutState { .. }
            | Action::TrackTimeout { .. }
            | Action::UntrackTimeout { .. }
            | Action::AdvanceTime { .. }
            | Action::HandleSystemFailure { .. }
            | Action::HandleAddonFailure { .. }
            | Action::HandleProcessFailure { .. }
            | Action::HandleIoFailure { .. } => Priority::Critical,
            Action::Submit { .. }
            | Action::StartProcess { .. }
            | Action::PollProcess { .. }
            | Action::PerformIo { .. }
            | Action::RegisterInterest { .. }
            | Action::RemoveInterest { .. }
            | Action::Query { .. }
            | Action::QueryResult { .. }
            | Action::CleanupJob { .. } => Priority::Normal,
            Action::ForegroundChanged { .. }
            | Action::PackagesChanged
            | Action::SystemRequest { .. }
            | Action::AddonTask { .. }
            | Action::AddonEvent { .. }
            | Action::EmitLog { .. } => Priority::Background,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct CauseId(pub u64);

#[derive(Clone)]
pub struct ActionMeta {
    pub id: CauseId,
    pub parent: Option<CauseId>,
    pub source: crate::high_level::identity::Principal,
    pub reply_to: Option<u32>,
}

#[derive(Clone)]
pub struct RoutedAction {
    pub action: Action,
    pub meta: ActionMeta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IoStream {
    Stdout,
    Stderr,
    Stdin,
}

pub enum Effect {
    Log {
        owner: u32,
        level: LogLevel,
        event: LogEvent,
    },
    WatchStream {
        io: IoHandle,
        stream: IoStream,
    },
    UnwatchStream {
        io: IoHandle,
        stream: IoStream,
    },
    StartProcess {
        id: u64,
        exec: ExecSpec,
        policy: ExecPolicy,
    },
    KillProcess {
        process: ProcessHandle,
        signal: ControlSignal,
    },
    PollProcess {
        process: ProcessHandle,
    },
    PerformIo {
        io: IoHandle,
    },
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
}

pub trait Module {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action>;
    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action>;
}

pub mod core_state;
pub mod io;
pub mod lifecycle;
pub mod process;
pub mod reducer;
pub mod replay;
pub mod result;
pub mod scheduler;
pub mod state_view;
pub mod verify;

pub const CORE_OWNER: u32 = 0;
pub const WARMUP_OWNER: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobLifecycle {
    Submitted,
    Admitted,
    Running,
    Terminating,
    Killed,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobIoState {
    Pending,
    Active,
    Closed,
}

#[derive(Clone)]
pub struct JobState {
    pub id: u64,
    pub owner: u32,
    pub exec: ExecSpec,
    pub policy: ExecPolicy,
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
    pub timed_out: bool,
    pub lifecycle: JobLifecycle,
    pub io_state: JobIoState,
}

#[derive(Clone)]
pub struct StoredResult {
    pub result: Result<ExecResult, ExecError>,
    pub owner: u32,
    pub created: u64,
}

#[derive(Clone)]
pub struct JobRuntime {
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
}

#[derive(Clone)]
pub struct TimeoutEntry {
    pub id: u64,
    pub state: TimeoutState,
    pub deadline: u64,
    pub kill_grace_ms: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeoutState {
    WaitingForDeadline,
    WaitingForKillGrace(u64),
}

pub struct ExecutionState {
    pub core: crate::core::core_state::CoreState,
    pub timeout: crate::core::policy::TimeoutStateStore,
    pub result: crate::core::result::ResultState,
    pub clock: u64,
    pub hash: u64,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            core: crate::core::core_state::CoreState::new(),
            timeout: crate::core::policy::TimeoutStateStore::new(),
            result: crate::core::result::ResultState::new(),
            clock: 0,
            hash: 0,
        }
    }

    pub fn update_hash(&mut self) {
        self.hash = self.core.hash ^ self.timeout.hash ^ self.result.hash;
    }
}

impl crate::core::state_view::StateView for ExecutionState {
    fn job(&self, id: u64) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_handle(id)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_process(&self, process: ProcessHandle) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_process(process)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_io(&self, io: IoHandle) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_io(io)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn result(&self, id: u64) -> Option<crate::core::state_view::ResultView> {
        self.result
            .results
            .get(&id)
            .map(|r| crate::core::state_view::ResultView {
                result: r.result.clone(),
                owner: r.owner,
            })
    }

    fn active_jobs(&self) -> usize {
        self.result.active_jobs
    }

    fn max_jobs(&self) -> usize {
        self.result.max_jobs
    }

    fn timeouts(&self) -> Vec<crate::core::state_view::TimeoutView> {
        self.timeout
            .timeouts
            .values()
            .map(|t| crate::core::state_view::TimeoutView {
                id: t.id,
                state: t.state.clone(),
                deadline: t.deadline,
                kill_grace_ms: t.kill_grace_ms,
            })
            .collect()
    }

    fn now(&self) -> u64 {
        self.clock
    }
}

pub struct Dispatcher {
    pub modules: Vec<Box<dyn Module>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            // ORDER IS SEMANTIC. DO NOT REORDER:
            // 1. AdmissionControl
            // 2. Lifecycle
            // 3. Process
            // 4. IO
            // 5. Result
            // 6. Timeout
            modules: vec![
                Box::new(crate::core::policy::AdmissionControlModule),
                Box::new(crate::core::lifecycle::LifecycleModule),
                Box::new(crate::core::process::ProcessModule),
                Box::new(crate::core::io::IoModule),
                Box::new(crate::core::result::ResultModule),
                Box::new(crate::core::policy::TimeoutPolicyModule::new()),
            ],
        }
    }

    pub fn dispatch(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        for module in &self.modules {
            actions.extend(module.handle(state, action));
        }
        actions
    }

    pub fn dispatch_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        for module in &self.modules {
            actions.extend(module.handle_event(state, event));
        }
        actions
    }

    pub fn compute_timeout_ms(&self, state: &dyn crate::core::state_view::StateView) -> i32 {
        let mut min_ms: i32 = -1;
        let now = state.now();
        for entry in state.timeouts() {
            let deadline = match entry.state {
                TimeoutState::WaitingForDeadline => entry.deadline,
                TimeoutState::WaitingForKillGrace(d) => d,
            };

            let ms = if deadline > now {
                (deadline - now) as i32
            } else {
                0
            };

            if min_ms == -1 || ms < min_ms {
                min_ms = ms;
            }
        }
        min_ms
    }
}

pub struct Core {
    pub dispatcher: Dispatcher,
    pub reducers: Vec<Box<dyn crate::core::reducer::Reducer>>,
    pub routing: std::collections::HashMap<ActionKind, Vec<usize>>,
}

impl Core {
    pub fn new() -> Self {
        let reducers: Vec<Box<dyn crate::core::reducer::Reducer>> = vec![
            Box::new(crate::core::reducer::TimeReducer),
            Box::new(crate::core::reducer::ResultReducer),
            Box::new(crate::core::reducer::IoReducer),
            Box::new(crate::core::reducer::JobReducer),
            Box::new(crate::core::reducer::TimeoutReducer),
            Box::new(crate::core::reducer::LogReducer),
            Box::new(crate::core::reducer::AddonReducer),
        ];

        for reducer in reducers.iter() {
            assert!(
                !reducer.handles().is_empty(),
                "Reducer must handle at least one action"
            );
        }

        let mut routing: std::collections::HashMap<ActionKind, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, reducer) in reducers.iter().enumerate() {
            for kind in reducer.handles() {
                routing.entry(*kind).or_default().push(idx);
            }
        }

        Self {
            dispatcher: Dispatcher::new(),
            reducers,
            routing,
        }
    }
}

```

---

## `rust/src/core/policy.rs`

```rs
use crate::core::{Action, Module, TimeoutEntry};
use std::collections::BTreeMap;

pub struct TimeoutStateStore {
    pub timeouts: BTreeMap<u64, TimeoutEntry>,
    pub hash: u64,
}

impl Default for TimeoutStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutStateStore {
    pub fn new() -> Self {
        Self {
            timeouts: BTreeMap::new(),
            hash: 0,
        }
    }
}

pub struct TimeoutPolicyModule;

impl Default for TimeoutPolicyModule {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutPolicyModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl Module for TimeoutPolicyModule {
    fn handle(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        _action: &Action,
    ) -> Vec<Action> {
        Vec::new()
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &crate::core::Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let crate::core::Event::Tick = event {
            let now = state.now();

            // Phase 1: Collect expired items deterministically
            // BTreeMap guarantees deterministic order
            for entry in state.timeouts() {
                match entry.state {
                    crate::core::TimeoutState::WaitingForDeadline => {
                        if now >= entry.deadline {
                            actions.push(Action::UpdateTimeoutState {
                                id: entry.id,
                                state: crate::core::TimeoutState::WaitingForKillGrace(
                                    now + (entry.kill_grace_ms as u64),
                                ),
                            });
                            actions.push(Action::TimeoutReached { id: entry.id });
                        }
                    }
                    crate::core::TimeoutState::WaitingForKillGrace(grace_deadline) => {
                        if now >= grace_deadline {
                            actions.push(Action::KillDeadlineReached { id: entry.id });
                            actions.push(Action::UntrackTimeout { id: entry.id });
                        }
                    }
                }
            }
        }

        actions
    }
}

#[derive(Default)]
pub struct AdmissionControlModule;

impl Module for AdmissionControlModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Action::Submit { id, owner, .. } = action {
            if state.active_jobs() >= state.max_jobs() {
                actions.push(Action::Rejected {
                    id: *id,
                    owner: *owner,
                    was_submitted: false,
                });
            } else {
                actions.push(Action::Admitted { id: *id });
            }
        }
        actions
    }

    fn handle_event(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        _event: &crate::core::Event,
    ) -> Vec<Action> {
        Vec::new()
    }
}

```

---

## `rust/src/core/process.rs`

```rs
use crate::core::{Action, Event, Module};

pub struct ProcessModule;

impl Module for ProcessModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match action {
            Action::Control { id, signal } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: *signal,
                    });
                    actions.push(Action::Controlled { id: *id });
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: crate::core::ControlSignal::GracefulStop,
                    });
                }
            }
            Action::KillDeadlineReached { id } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: crate::core::ControlSignal::ForceKill,
                    });
                }
            }
            // `StartProcess`, `SignalProcess`, and `PollProcess` are mapped directly to `Effect`s in `resolve_effects`.
            // We don't generate additional intent from them here.
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::ProcessSpawnFailed { id, err } => {
                let job = state.job(*id);
                let owner = job.as_ref().map(|j| j.owner).unwrap_or(0);
                let was_submitted = job
                    .as_ref()
                    .map(|j| j.lifecycle == crate::core::JobLifecycle::Submitted)
                    .unwrap_or(false);
                actions.push(Action::Finished {
                    id: *id,
                    owner,
                    was_submitted,
                    result: Err(crate::core::ExecError::SpawnFailed),
                });
                actions.push(Action::EmitLog {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: *id,
                        err: err.clone(),
                    },
                });
            }
            Event::KillProcessFailed { process, err } => {
                actions.push(Action::HandleProcessFailure {
                    process: *process,
                    err: err.clone(),
                });
                if let Some(job) = state.job_by_process(*process) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            Event::TimeAdvanced(delta) => {
                actions.push(Action::AdvanceTime { delta: *delta });
            }
            Event::ReactorError { err } => {
                actions.push(Action::EmitLog {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: err.clone(),
                    },
                });
            }
            _ => {}
        }
        actions
    }
}

```

---

## `rust/src/core/reducer.rs`

```rs
use crate::core::ActionKind;
use crate::core::Effect;
use crate::core::{Action, JobIoState, JobLifecycle};

pub struct ReducerCtx<'a> {
    pub core: &'a mut crate::core::core_state::CoreState,
    pub timeout: &'a mut crate::core::policy::TimeoutStateStore,
    pub result: &'a mut crate::core::result::ResultState,
    pub clock: &'a mut u64,
}

pub trait Reducer {
    fn handles(&self) -> &'static [ActionKind];
    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>);
}

pub struct JobReducer;

impl Reducer for JobReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::Submit,
                    ActionKind::SetLifecycle,
                    ActionKind::Finished,
                    ActionKind::Rejected,
                    ActionKind::CleanupJob,
                    ActionKind::TimeoutReached,
                    ActionKind::AssignProcess,
                    ActionKind::StartProcess,
                    ActionKind::PollProcess,
                    ActionKind::SignalProcess,
                    ActionKind::HandleProcessFailure,
                    ActionKind::SetJobIoState,
                    ActionKind::AssignIo,
                    ActionKind::HandleIoFailure,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        let core = &mut ctx.core;
        match action {
            Action::Submit { id, owner, job: job_req } => {
                let policy = if job_req.command.first().map(|s| s.as_str()) == Some("dumpsys") {
                    crate::core::ExecPolicy { timeout_ms: Some(3000), kill_grace_ms: 500, cancel: crate::core::CancelPolicy::Kill }
                } else {
                    crate::core::ExecPolicy { timeout_ms: Some(1000), kill_grace_ms: 300, cancel: crate::core::CancelPolicy::Kill }
                };
                let exec = crate::core::ExecSpec {
                    argv: job_req.command.clone(), stdin: None, capture_stdout: true, capture_stderr: true, max_output: 1024 * 1024,
                };

                core.insert_job(*id, *owner, exec, policy);
            }
            Action::SetLifecycle { id, state: lifecycle_state } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.lifecycle != *lifecycle_state {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).lifecycle = *lifecycle_state;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::Finished { id, .. } | Action::Rejected { id, .. } => {
                // ONLY semantic transition, NO removals here
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.lifecycle != JobLifecycle::Finished {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).lifecycle = JobLifecycle::Finished;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::CleanupJob { id } => {
                core.remove_job(*id);
            }
            Action::AssignProcess { id, process } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.process != Some(*process) {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).process = Some(*process);
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                    if let Some(old) = core.runtime(h).process {
                        core.remove_process_index(old);
                    }
                    core.runtime_mut(h).process = Some(*process);
                    core.insert_process_index(*process, h);
                }
            }
            Action::AssignIo { id, io } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.io != Some(*io) {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).io = Some(*io);
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                    if let Some(old) = core.runtime(h).io {
                        core.remove_io_index(old);
                    }
                    core.runtime_mut(h).io = Some(*io);
                    core.insert_io_index(*io, h);
                }
            }
            Action::SetJobIoState { id, state } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.io_state != *state {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).io_state = *state;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if !old_job.timed_out {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).timed_out = true;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::PollProcess { process } => {
                effects.push(crate::core::Effect::PollProcess { process: *process });
            }
            Action::StartProcess { id } => {
                if let Some(h) = core.job_handle(*id) {
                    let job = core.job(h);
                    effects.push(crate::core::Effect::StartProcess { id: *id, exec: job.exec.clone(), policy: job.policy.clone() });
                }
            }
            Action::SignalProcess { process, signal } => {
                effects.push(crate::core::Effect::KillProcess { process: *process, signal: *signal });
            }
            Action::HandleProcessFailure { process, err } => {
                if let Some(h) = core.job_by_process(*process) {
                    let id = core.job(h).id;
                    effects.push(crate::core::Effect::Log {
                        owner: core.job(h).owner,
                        level: crate::core::LogLevel::Error,
                        event: crate::core::LogEvent::Error { id, err: err.clone() },
                    });

                    let old_job = core.job(h).clone();
                    if old_job.process.is_some() {
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).process = None;
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::HandleIoFailure { io, reason } => {
                if let Some(h) = core.job_by_io(*io) {
                    let id = core.job(h).id;
                    effects.push(crate::core::Effect::Log {
                        owner: core.job(h).owner,
                        level: crate::core::LogLevel::Error,
                        event: crate::core::LogEvent::Error { id, err: reason.clone() },
                    });

                    let old_job = core.job(h).clone();
                    if old_job.io.is_some() || old_job.io_state != JobIoState::Closed {
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(&old_job));
                        let job_mut = core.job_mut(h);
                        job_mut.io = None;
                        job_mut.io_state = JobIoState::Closed;
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct IoReducer;

impl Reducer for IoReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::RegisterInterest,
                    ActionKind::RemoveInterest,
                    ActionKind::PerformIo,
                ]
            })
            .as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        match action {
            Action::RegisterInterest { io, stream } => {
                effects.push(crate::core::Effect::WatchStream {
                    io: *io,
                    stream: stream.clone(),
                });
            }
            Action::RemoveInterest { io, stream } => {
                effects.push(crate::core::Effect::UnwatchStream { io: *io, stream: stream.clone() });
            }
            Action::PerformIo { io } => {
                effects.push(crate::core::Effect::PerformIo { io: *io });
            }
            _ => {}
        }
    }
}
pub struct TimeoutReducer;

impl Reducer for TimeoutReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::TrackTimeout,
                    ActionKind::UntrackTimeout,
                    ActionKind::UpdateTimeoutState,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        let ts = &mut ctx.timeout;
        match action {
            Action::TrackTimeout {
                id,
                deadline,
                kill_grace_ms,
            } => {
                if !ts.timeouts.contains_key(id) {
                    let entry = crate::core::TimeoutEntry {
                        id: *id,
                        state: crate::core::TimeoutState::WaitingForDeadline,
                        deadline: *deadline,
                        kill_grace_ms: *kill_grace_ms,
                    };
                    let st_hash = match entry.state {
                        crate::core::TimeoutState::WaitingForDeadline => 0,
                        crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                    };
                    ts.timeouts.insert(*id, entry);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(st_hash);
                }
            }
            Action::UntrackTimeout { id } => {
                if let Some(entry) = ts.timeouts.remove(id) {
                    let st_hash = match entry.state {
                        crate::core::TimeoutState::WaitingForDeadline => 0,
                        crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                    };
                    ts.hash ^= id.wrapping_mul(0x5BD1E995);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(st_hash);
                }
            }
            Action::UpdateTimeoutState {
                id,
                state: new_state,
            } => {
                if let Some(entry) = ts.timeouts.get_mut(id) {
                    if entry.state != *new_state {
                        let old_hash = match entry.state {
                            crate::core::TimeoutState::WaitingForDeadline => 0,
                            crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                        };
                        let new_hash = match new_state {
                            crate::core::TimeoutState::WaitingForDeadline => 0,
                            crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                        };
                        ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(old_hash);
                        entry.state = new_state.clone();
                        ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(new_hash);
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct AddonReducer;

impl Reducer for AddonReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::AddonTask,
                    ActionKind::AddonEvent,
                    ActionKind::SystemRequest,
                    ActionKind::HandleAddonFailure,
                    ActionKind::HandleSystemFailure,
                ]
            })
            .as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        match action {
            Action::AddonTask { addon_id, key, payload } => {
                effects.push(Effect::AddonTask { addon_id: *addon_id, key: key.clone(), payload: payload.clone() });
            }
            Action::SystemRequest { request_id, kind, payload } => {
                effects.push(Effect::SystemRequest { request_id: *request_id, kind: *kind, payload: payload.clone() });
            }
            Action::HandleAddonFailure { addon_id, key, err } => {
                 effects.push(Effect::Log {
                    owner: *addon_id,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: format!("Addon {} key {} failed: {}", addon_id, key, err),
                    },
                });
            }
            Action::HandleSystemFailure { request_id, kind, err } => {
                 effects.push(Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: *request_id,
                        err: format!("SystemRequest {:?} failed: {}", kind, err),
                    },
                });
            }
            _ => {}
        }
    }
}

pub struct ResultReducer;

impl Reducer for ResultReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::Admitted,
                    ActionKind::Finished,
                    ActionKind::Rejected,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        let clock = *ctx.clock;
        match action {
            Action::Admitted { id: _ } => {
                let rs = &mut ctx.result;
                rs.hash ^= rs.active_jobs as u64;
                rs.active_jobs += 1;
                rs.hash ^= rs.active_jobs as u64;
            }
            Action::Finished {
                id,
                result,
                owner,
                was_submitted,
            } => {
                let owner = *owner;
                let was_submitted = *was_submitted;
                let rs = &mut ctx.result;
                if !was_submitted && rs.active_jobs > 0 {
                    rs.hash ^= rs.active_jobs as u64;
                    rs.active_jobs -= 1;
                    rs.hash ^= rs.active_jobs as u64;
                }
                if let Some(old) = rs.results.insert(
                    *id,
                    crate::core::StoredResult {
                        result: result.clone(),
                        owner,
                        created: clock,
                    },
                ) {
                    rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(old.owner as u64);
                }
                rs.result_order.push_back(*id);
                rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(owner as u64);
                if rs.result_order.len() > 100 {
                    if let Some(old_id) = rs.result_order.pop_front() {
                        if let Some(removed) = rs.results.remove(&old_id) {
                            rs.hash ^= old_id
                                .wrapping_mul(0x1234567)
                                .wrapping_add(removed.owner as u64);
                        }
                    }
                }
            }
            Action::Rejected {
                id,
                owner,
                was_submitted,
            } => {
                let owner = *owner;
                let was_submitted = *was_submitted;
                let rs = &mut ctx.result;
                if !was_submitted && rs.active_jobs > 0 {
                    rs.hash ^= rs.active_jobs as u64;
                    rs.active_jobs -= 1;
                    rs.hash ^= rs.active_jobs as u64;
                }
                if let Some(old) = rs.results.insert(
                    *id,
                    crate::core::StoredResult {
                        result: Err(crate::core::ExecError::Internal("Rejected".to_string())),
                        owner,
                        created: clock,
                    },
                ) {
                    rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(old.owner as u64);
                }
                rs.result_order.push_back(*id);
                rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(owner as u64);
                if rs.result_order.len() > 100 {
                    if let Some(old_id) = rs.result_order.pop_front() {
                        if let Some(removed) = rs.results.remove(&old_id) {
                            rs.hash ^= old_id
                                .wrapping_mul(0x1234567)
                                .wrapping_add(removed.owner as u64);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
pub struct LogReducer;

impl Reducer for LogReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES.get_or_init(|| vec![ActionKind::EmitLog]).as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        if let Action::EmitLog {
            owner,
            level,
            event,
        } = action
        {
            effects.push(Effect::Log {
                owner: *owner,
                level: *level,
                event: event.clone(),
            });
        }
    }
}
pub struct TimeReducer;

impl Reducer for TimeReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| vec![ActionKind::AdvanceTime])
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        if let Action::AdvanceTime { delta } = action {
            *ctx.clock += delta;
        }
    }
}

```

---

## `rust/src/core/replay.rs`

```rs
use crate::core::{Event, Intent};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TickStats {
    pub hash: u64,
    pub actions_processed: usize,
    pub dropped_actions: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ReplayInput {
    Event(Event),
    LegacyIntent(Intent), // Legacy support for intent without principal
    Intent(crate::high_level::identity::Principal, Intent),
    TickHash(u64), // Legacy
    TickEnd(TickStats),
    Time(std::time::Duration), // We need to record time because if we don't, time-dependent logic (timeouts, etc) will diverge during replay.
}

thread_local! {
    pub static EPOCH: std::time::Instant = std::time::Instant::now();
}
pub fn hash_str(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
```

---

## `rust/src/core/result.rs`

```rs
use crate::core::{Action, Event, Module, StoredResult};
use std::collections::{BTreeMap, VecDeque};

pub struct ResultState {
    pub results: BTreeMap<u64, StoredResult>,
    pub result_order: VecDeque<u64>,
    pub active_jobs: usize,
    pub max_jobs: usize,
    pub hash: u64,
}

impl Default for ResultState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultState {
    pub fn new() -> Self {
        Self {
            results: BTreeMap::new(),
            result_order: VecDeque::new(),
            active_jobs: 0,
            max_jobs: 64,
            hash: 0,
        }
    }
}

pub struct ResultModule;

impl Module for ResultModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match action {
            Action::Query { id } => {
                if let Some(r) = state.result(*id) {
                    let outcome = crate::core::ExecOutcome {
                        id: *id,
                        result: r.result.clone(),
                    };
                    actions.push(Action::QueryResult {
                        id: *id,
                        result: Some(outcome),
                    });
                } else {
                    actions.push(Action::QueryResult {
                        id: *id,
                        result: None,
                    });
                }
            }
            Action::Finished {
                id,
                result: _,
                owner: _,
                was_submitted: _,
            } => {
                actions.push(Action::CleanupJob { id: *id });
            }
            Action::Rejected {
                id,
                owner: _,
                was_submitted: _,
            } => {
                actions.push(Action::CleanupJob { id: *id });
            }
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Event::ProcessExited { process, status } = event {
            if let Some(job) = state.job_by_process(*process) {
                // For now, assume process exits without explicit buffered IO completion.
                // A proper design will receive IoDataReceived or the drain contents from runtime.
                // We simply synthesize empty stdout/stderr if the drain parts aren't forwarded.
                let result = crate::core::ExecResult {
                    status: *status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    timed_out: job.timed_out,
                };
                let was_submitted = job.lifecycle == crate::core::JobLifecycle::Submitted;
                actions.push(Action::Finished {
                    id: job.id,
                    owner: job.owner,
                    was_submitted,
                    result: Ok(result),
                });
            }
        }
        actions
    }
}

```

---

## `rust/src/core/scheduler.rs`

```rs
use crate::core::{Event, Priority, RoutedAction};
use std::collections::VecDeque;

pub const MAX_ACTIONS_PER_TICK: usize = 2048;
pub const MAX_QUEUE: usize = 4096;
const MAX_PER_ACTION_KIND: usize = 1_000;

pub struct Scheduler {
    critical_queue: VecDeque<RoutedAction>,
    normal_queue: VecDeque<RoutedAction>,
    background_queue: VecDeque<RoutedAction>,
    pub total_len: usize,
    per_kind_counts: [usize; 43], // Based on ActionKind count
    step_budget: usize,
    steps_executed: usize,
    rr_index: usize,
    rr_count: usize,
}

impl Scheduler {
    pub fn new(step_budget: usize) -> Self {
        Self {
            critical_queue: VecDeque::with_capacity(MAX_QUEUE),
            normal_queue: VecDeque::with_capacity(MAX_QUEUE),
            background_queue: VecDeque::with_capacity(MAX_QUEUE),
            total_len: 0,
            per_kind_counts: [0; 43],
            step_budget,
            steps_executed: 0,
            rr_index: 0,
            rr_count: 0,
        }
    }

    pub fn enqueue(&mut self, action: RoutedAction) -> Option<Event> {
        let kind = action.action.kind();
        let kind_idx = kind as usize;
        let count = self.per_kind_counts[kind_idx];
        if count >= MAX_PER_ACTION_KIND {
            return Some(crate::core::Event::DroppedAction { kind });
        }

        let mut dropped_event = None;

        if self.total_len >= MAX_QUEUE {
            // Drop lowest priority, oldest (FIFO drop -> pop_front)
            let mut evicted = None;
            let action_prio = action.action.priority();

            if !self.background_queue.is_empty() {
                if action_prio <= Priority::Background {
                    evicted = self.background_queue.pop_front();
                }
            } else if !self.normal_queue.is_empty() {
                if action_prio <= Priority::Normal {
                    evicted = self.normal_queue.pop_front();
                }
            } else if !self.critical_queue.is_empty() {
                if action_prio <= Priority::Critical {
                    evicted = self.critical_queue.pop_front();
                }
            }

            if let Some(ev) = evicted {
                let ev_kind = ev.action.kind();
                self.per_kind_counts[ev_kind as usize] -= 1;
                self.total_len -= 1;
                dropped_event = Some(crate::core::Event::DroppedAction { kind: ev_kind });
            } else {
                // Cannot evict, so drop the incoming action
                return Some(crate::core::Event::DroppedAction { kind });
            }
        }

        self.per_kind_counts[kind_idx] += 1;
        match action.action.priority() {
            Priority::Critical => self.critical_queue.push_back(action),
            Priority::Normal => self.normal_queue.push_back(action),
            Priority::Background => self.background_queue.push_back(action),
        }
        self.total_len += 1;

        dropped_event
    }

    pub fn next(&mut self) -> Option<RoutedAction> {
        if self.steps_executed >= self.step_budget {
            return None;
        }

        let schedule = [
            (Priority::Critical, 4),
            (Priority::Normal, 2),
            (Priority::Background, 1),
        ];

        let start_index = self.rr_index;
        let mut checked_all = false;

        loop {
            let (prio, quota) = schedule[self.rr_index];

            if self.rr_count < quota {
                let routed_opt = match prio {
                    Priority::Critical => self.critical_queue.pop_front(),
                    Priority::Normal => self.normal_queue.pop_front(),
                    Priority::Background => self.background_queue.pop_front(),
                };

                if let Some(routed) = routed_opt {
                    self.rr_count += 1;
                    self.total_len -= 1;
                    self.steps_executed += 1;
                    let kind = routed.action.kind();
                    self.per_kind_counts[kind as usize] -= 1;
                    return Some(routed);
                } else {
                    self.rr_index = (self.rr_index + 1) % schedule.len();
                    self.rr_count = 0;
                }
            } else {
                self.rr_index = (self.rr_index + 1) % schedule.len();
                self.rr_count = 0;
            }

            if self.rr_index == start_index && self.rr_count == 0 {
                if checked_all {
                    return None;
                }
                checked_all = true;
            }
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.steps_executed >= self.step_budget || self.total_len == 0
    }
}

```

---

## `rust/src/core/state_view.rs`

```rs
use crate::core::{IoHandle, JobIoState, JobLifecycle, ProcessHandle};

pub struct JobView {
    pub id: u64,
    pub owner: u32,
    pub lifecycle: JobLifecycle,
    pub io_state: JobIoState,
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
    pub timed_out: bool,
}

pub struct TimeoutView {
    pub id: u64,
    pub state: crate::core::TimeoutState,
    pub deadline: u64,
    pub kill_grace_ms: u32,
}

pub struct ResultView {
    pub result: Result<crate::core::ExecResult, crate::core::ExecError>,
    pub owner: u32,
}

pub trait StateView {
    fn job(&self, id: u64) -> Option<JobView>;
    fn job_by_process(&self, process: ProcessHandle) -> Option<JobView>;
    fn job_by_io(&self, io: IoHandle) -> Option<JobView>;

    fn result(&self, id: u64) -> Option<ResultView>;

    fn active_jobs(&self) -> usize;
    fn max_jobs(&self) -> usize;

    fn timeouts(&self) -> Vec<TimeoutView>;

    fn now(&self) -> u64;
}

```

---

## `rust/src/core/validation.rs`

```rs
use crate::high_level::identity::Request;
use crate::core::Intent;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidJobId,
    InvalidHandle,
    DuplicateProcessAssignment,
    IllegalTransition,
    Unknown,
}

pub fn validate_request(
    req: &Request,
    state: &dyn crate::core::state_view::StateView,
) -> Result<(), ErrorCode> {
    match &req.intent {
        Intent::Submit { .. } => {
            // Further submit validation if needed
        }
        Intent::Control { id, .. } | Intent::Query { id } => {
            let job = state.job(*id);
            if job.is_none() {
                return Err(ErrorCode::InvalidJobId);
            }
        }
        _ => {}
    }
    Ok(())
}

```

---

## `rust/src/core/verify.rs`

```rs
use crate::core::ExecutionState;

pub fn verify_global(state: &ExecutionState) {
    let core = &state.core;
    let _timeout = &state.timeout;
    let _result = &state.result;

    // Verify jobs and runtime
    let mut actual_process_count = 0;
    let mut actual_io_count = 0;

    // We can't iterate Arena without implementing iter, but let's check id_map instead
    for (u64_id, handle) in &core.job_id_map {
        let job = core.jobs.get(handle.index, handle.generation).expect("dangling job_id_map entry");
        assert_eq!(*u64_id, job.id);

        let rt = core.runtime.get(handle.index as usize)
            .expect("missing runtime vector entry")
            .as_ref()
            .expect("job missing runtime mapping");

        assert!(job.process == rt.process, "job process handle mismatch with runtime");
        assert!(job.io == rt.io, "job io handle mismatch with runtime");

        if let Some(p) = rt.process {
            actual_process_count += 1;
            let p_handle = core.process_index.get(p.index as usize)
                .expect("missing process vector entry")
                .as_ref()
                .expect("process index dangling");
            assert_eq!(*p_handle, *handle, "process index mismatch");
        }

        if let Some(io) = rt.io {
            actual_io_count += 1;
            let io_handle = core.io_index.get(io.index as usize)
                .expect("missing io vector entry")
                .as_ref()
                .expect("io index dangling");
            assert_eq!(*io_handle, *handle, "io index mismatch");
        }
    }

    assert_eq!(core.process_count, actual_process_count, "process count drift");
    assert_eq!(core.io_count, actual_io_count, "io count drift");
}

```

---

## `rust/src/high_level/addon.rs`

```rs
use crate::high_level::capability::CapabilityToken;
use crate::low_level::reactor::Event as ReactorEvent;
use crate::core::state_view::StateView;
use crate::core::Event;

pub struct AddonSpec {
    pub id: u32,
    pub capability: CapabilityToken,
    pub max_actions_per_tick: u32,
}

pub trait Addon {
    fn on_reactor_event(&mut self, _state: &dyn StateView, _event: &ReactorEvent) -> Vec<crate::high_level::identity::Request> {
        Vec::new()
    }
    fn on_core_event(&mut self, _state: &dyn StateView, _event: &Event) -> Vec<crate::high_level::identity::Request> {
        Vec::new()
    }
}

pub struct NoOpAddon;
impl Addon for NoOpAddon {}

pub struct EchoAddon;
impl Addon for EchoAddon {}

```

---

## `rust/src/high_level/addons/mod.rs`

```rs
pub mod preload;

```

---

## `rust/src/high_level/addons/preload.rs`

```rs
use crate::core::state_view::StateView;
use crate::core::{Event, Intent, SystemService};
use crate::high_level::addon::Addon;
use crate::high_level::identity::{Principal, Request};
use std::collections::{BTreeMap, BTreeSet};

pub struct PreloadAddon {
    pub dedup_cache: BTreeMap<String, u64>,
    pub negative_cache: BTreeMap<String, u64>,
    pub package_map: BTreeMap<String, std::path::PathBuf>,
    pub in_flight: BTreeSet<String>,
}

impl PreloadAddon {
    pub fn new() -> Self {
        Self {
            dedup_cache: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
            package_map: BTreeMap::new(),
            in_flight: BTreeSet::new(),
        }
    }

    fn submit(&self, intent: Intent) -> Request {
        Request {
            principal: Principal::Addon(102), // Preload ID
            client_id: None,
            cause: crate::core::CauseId(0), // Will be assigned by core
            intent,
        }
    }
}

impl Addon for PreloadAddon {
    fn on_core_event(
        &mut self,
        state: &dyn StateView,
        event: &Event,
    ) -> Vec<Request> {
        let mut reqs = Vec::new();
        match event {
            Event::ForegroundChanged { pid } => {
                let payload = serde_json::to_vec(pid).unwrap_or_default();
                reqs.push(self.submit(Intent::SystemRequest {
                    request_id: 0, // Using CauseId in core is better but for now 0
                    kind: SystemService::ResolveIdentity,
                    payload,
                }));
            }
            Event::PackagesChanged => {
                reqs.push(self.submit(Intent::PackagesChanged));
            }
            Event::SystemResponse { request_id: _, kind, payload } => {
                match kind {
                    SystemService::ResolveIdentity => {
                        if let Ok(package_name) = String::from_utf8(payload.clone()) {
                             if self.in_flight.contains(&package_name) {
                                return reqs;
                            }

                            let now = state.now();
                            if let Some(t) = self.negative_cache.get(&package_name)
                                && now.saturating_sub(*t) < 300_000
                            {
                                return reqs;
                            }

                            if let Some(last_warmup) = self.dedup_cache.get(&package_name)
                                && now.saturating_sub(*last_warmup) < 60_000
                            {
                                return reqs;
                            }

                            if let Some(base_dir) = self.package_map.get(&package_name) {
                                let payload = serde_json::to_vec(&(package_name.clone(), base_dir.to_string_lossy().into_owned())).unwrap_or_default();
                                reqs.push(self.submit(Intent::SystemRequest {
                                    request_id: 0,
                                    kind: SystemService::DiscoverPaths,
                                    payload,
                                }));
                            } else {
                                let payload = package_name.clone().into_bytes();
                                reqs.push(self.submit(Intent::SystemRequest {
                                    request_id: 0,
                                    kind: SystemService::ResolveDirectory,
                                    payload,
                                }));
                            }
                        }
                    }
                    SystemService::ResolveDirectory => {
                         if let Ok((package_name, base_dir)) = serde_json::from_slice::<(String, String)>(&payload) {
                             self.package_map.insert(package_name.clone(), std::path::PathBuf::from(&base_dir));
                             let payload = serde_json::to_vec(&(package_name, base_dir)).unwrap_or_default();
                             reqs.push(self.submit(Intent::SystemRequest {
                                request_id: 0,
                                kind: SystemService::DiscoverPaths,
                                payload,
                            }));
                         }
                    }
                    SystemService::DiscoverPaths => {
                        if let Ok((package_name, paths)) = serde_json::from_slice::<(String, Vec<String>)>(&payload) {
                            self.in_flight.insert(package_name.clone());
                            // AddonTask is still the correct way for warmup work
                            let mut task_payload = vec![1u8]; // Type 1 = Warmup
                            task_payload.extend(serde_json::to_vec(&paths).unwrap_or_default());

                            reqs.push(self.submit(Intent::AddonTask {
                                addon_id: 102,
                                key: format!("warmup:{}", package_name),
                                payload: task_payload,
                            }));
                        }
                    }
                }
            }
            Event::AddonCompleted { addon_id, key, .. } if *addon_id == 102 => {
                if key.starts_with("warmup:") {
                    let package = &key[7..];
                    self.in_flight.remove(package);
                    self.dedup_cache.insert(package.to_string(), state.now());
                }
            }
            Event::AddonFailed { addon_id, key, .. } if *addon_id == 102 => {
                if key.starts_with("warmup:") {
                    let package = &key[7..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                } else if key.starts_with("resolve_dir:") {
                    let package = &key[12..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                } else if key.starts_with("discover_paths:") {
                    let package = &key[15..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                }
            }
            _ => {}
        }
        reqs
    }
}

```

---

## `rust/src/high_level/android.rs`

```rs
#[cfg(target_os = "android")]
pub fn getprop(key: &str) -> Option<String> {
    use libc::c_char;

    let mut buf = [0u8; 92]; // PROP_VALUE_MAX

    let key_c = std::ffi::CString::new(key).ok()?;

    let len = unsafe {
        crate::low_level::spawn::__system_property_get(
            key_c.as_ptr() as *const c_char,
            buf.as_mut_ptr() as *mut c_char,
        )
    };

    if len > 0 {
        Some(String::from_utf8_lossy(&buf[..len as usize]).into_owned())
    } else {
        None
    }
}

#[cfg(target_os = "android")]
unsafe extern "C" {
    fn __system_property_set(name: *const libc::c_char, value: *const libc::c_char) -> libc::c_int;
}

#[cfg(target_os = "android")]
pub fn setprop(key: &str, value: &str) -> bool {
    let k = std::ffi::CString::new(key).ok();
    let v = std::ffi::CString::new(value).ok();

    if let (Some(k), Some(v)) = (k, v) {
        unsafe { __system_property_set(k.as_ptr(), v.as_ptr()) == 0 }
    } else {
        false
    }
}

use crate::core::CancelPolicy;

pub struct ExecConfig {
    pub timeout_ms: Option<u32>,
    pub kill_grace_ms: u32,
    pub cancel: CancelPolicy,
    pub max_output: usize,
}

// Emulating the old struct to allow high_level::api mapping.
// It could just return (ExecSpec, ExecPolicy) directly but we keep it modular.
pub struct AndroidExecRequest {
    pub argv: Vec<String>,
    pub stdin: Option<Vec<u8>>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub timeout_ms: Option<u32>,
    pub kill_grace_ms: u32,
    pub cancel: CancelPolicy,
    pub max_output: usize,
}

pub fn cmd(service: &str, args: &[&str], cfg: ExecConfig) -> AndroidExecRequest {
    let mut argv = Vec::with_capacity(2 + args.len());
    argv.push("/system/bin/cmd".to_string());
    argv.push(service.to_string());
    for a in args {
        argv.push(a.to_string());
    }

    AndroidExecRequest {
        argv,
        stdin: None,
        capture_stdout: true,
        capture_stderr: true,
        timeout_ms: cfg.timeout_ms,
        kill_grace_ms: cfg.kill_grace_ms,
        cancel: cfg.cancel,
        max_output: cfg.max_output,
    }
}

pub fn dumpsys(service: &str, args: &[&str], cfg: ExecConfig) -> AndroidExecRequest {
    let mut argv = Vec::with_capacity(2 + args.len());
    argv.push("/system/bin/dumpsys".to_string());
    argv.push(service.to_string());
    for a in args {
        argv.push(a.to_string());
    }

    AndroidExecRequest {
        argv,
        stdin: None,
        capture_stdout: true,
        capture_stderr: true,
        timeout_ms: cfg.timeout_ms,
        kill_grace_ms: cfg.kill_grace_ms,
        cancel: cfg.cancel,
        max_output: cfg.max_output,
    }
}

```

---

## `rust/src/high_level/api.rs`

```rs
use crate::core::{CancelPolicy, ExecPolicy, ExecSpec};
use serde::{Deserialize, Serialize};

/// Token/Capability enforcement layers bounding operation intent.
// Replaced by CapabilityToken



/// Strict command schema defining finite execution space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Cmd { service: String, args: Vec<String> },
    Dumpsys { service: String, args: Vec<String> },
    GetResult { id: u64 },
    Cancel { id: u64 },
}

use crate::high_level::android::ExecConfig;

impl Command {
    pub fn map_to_exec(self) -> (ExecSpec, ExecPolicy) {
        match self {
            Command::Cmd { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 1024 * 1024,
                };
                let req = crate::high_level::android::cmd(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::Dumpsys { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 4 * 1024 * 1024,
                };
                let req = crate::high_level::android::dumpsys(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::GetResult { .. } | Command::Cancel { .. } => unreachable!(),
        }
    }
}

```

---

## `rust/src/high_level/bridge.rs`

```rs

```

---

## `rust/src/high_level/capability.rs`

```rs
use crate::core::ActionKind;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct CapabilityToken {
    pub restricted_actions: u64,
}

impl CapabilityToken {
    pub fn empty() -> Self {
        Self {
            restricted_actions: !0,
        }
    }

    pub fn allow_all() -> Self {
        Self {
            restricted_actions: 0,
        }
    }

    pub fn new(restricted_bits: u64) -> Self {
        Self {
            restricted_actions: restricted_bits,
        }
    }

    pub fn allows(&self, kind: ActionKind) -> bool {
        let bit = 1u64 << (kind as u64);
        (self.restricted_actions & bit) == 0
    }
}

pub struct CapabilityRegistry {
    pub map: HashMap<u32, CapabilityToken>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, uid: u32) -> Option<&CapabilityToken> {
        self.map.get(&uid)
    }

    pub fn insert(&mut self, uid: u32, token: CapabilityToken) {
        self.map.insert(uid, token);
    }

    pub fn allows(&self, principal: &crate::high_level::identity::Principal, kind: crate::core::ActionKind) -> bool {
        let uid = match principal {
            crate::high_level::identity::Principal::System => 0,
            crate::high_level::identity::Principal::User(u) => *u,
            crate::high_level::identity::Principal::Addon(u) => *u,
        };
        if let Some(token) = self.get(uid) {
            token.allows(kind)
        } else {
            false // default to denied
        }
    }
}

```

---

## `rust/src/high_level/identity.rs`

```rs
use crate::core::Intent;

#[derive(Clone, Debug)]
#[derive(PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Principal {
    System,
    User(u32),
    Addon(u32),
}

impl Principal {
    pub fn new_user(uid: u32) -> Self {
        if uid == 0 {
            Principal::System
        } else {
            Principal::User(uid)
        }
    }
}

#[derive(Clone)]
pub struct Request {
    pub client_id: Option<u32>,
    pub principal: Principal,
    pub intent: Intent,
    pub cause: crate::core::CauseId,
}

```

---

## `rust/src/high_level/mod.rs`

```rs
pub mod addon;
pub mod addons;
pub mod android;
pub mod api;
pub mod bridge;
pub mod capability;
pub mod identity;

```

---

## `rust/src/lib.rs`

```rs
#![allow(non_snake_case)]

#[macro_use]
pub mod low_level;

pub mod mid_level;

pub mod arena;
pub mod core;
pub mod high_level;
pub mod runtime;

#[derive(Debug)]
pub enum RuntimeLimit {
    StepBudgetExceeded,
    QueueOverflow,
    ActionRepetitionExceeded,
    CauseDepthExceeded,
}

pub struct DaemonConfig {
    pub enable_warmup: bool,
    pub record_path: Option<String>,
}

pub struct TraceStore {
    pub parents: std::collections::HashMap<crate::core::CauseId, Option<crate::core::CauseId>>,
    pub order: std::collections::VecDeque<crate::core::CauseId>,
}

impl Default for TraceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStore {
    pub fn new() -> Self {
        Self {
            parents: std::collections::HashMap::new(),
            order: std::collections::VecDeque::new(),
        }
    }

    pub fn insert(&mut self, id: crate::core::CauseId, parent: Option<crate::core::CauseId>) {
        self.parents.insert(id, parent);
        self.order.push_back(id);
        if self.order.len() > 10_000 && let Some(old_id) = self.order.pop_front() {
            self.parents.remove(&old_id);
        }
    }
}

pub fn run_daemon(config: DaemonConfig) -> Result<(), crate::low_level::spawn::SysError> {
    const MAX_ACTIONS_PER_TICK: usize = 10_000;

    use crate::core::{Core, ExecutionState};
    use crate::high_level::capability::{CapabilityRegistry, CapabilityToken};
    use crate::low_level::reactor::{Fd, Reactor, Token};
    use crate::mid_level::ipc::IpcModule;

    let mut reactor = Reactor::new()?;
    let ipc_fd = Fd::new(
        unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        },
        "ipc",
    )?;

    let socket_path = "/data/local/tmp/coreshift.sock";
    let _ = std::fs::remove_file(socket_path);

    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as u16;
    let path_bytes = socket_path.as_bytes();
    for (i, &b) in path_bytes.iter().enumerate() {
        addr.sun_path[i] = b as _;
    }

    let ret = unsafe {
        libc::bind(
            ipc_fd.as_raw_fd(),
            &addr as *const libc::sockaddr_un as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t,
        )
    };
    if ret < 0 {
        return Err(crate::low_level::spawn::SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "bind(AF_UNIX)",
        ));
    }

    let ret = unsafe { libc::listen(ipc_fd.as_raw_fd(), 128) };
    if ret < 0 {
        return Err(crate::low_level::spawn::SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "listen",
        ));
    }

    let ipc_token = Token(1);
    use std::os::unix::io::AsRawFd;
    reactor.add_with_token(ipc_fd.as_raw_fd(), ipc_token, true, false)?;

    let mut state = ExecutionState::new();
    let core = Core::new();
    use crate::high_level::addon::{Addon, AddonSpec, EchoAddon, NoOpAddon};
    use crate::high_level::addons::preload::PreloadAddon;

    let mut addons: Vec<(Box<dyn Addon>, AddonSpec)> = vec![
        (
            Box::new(NoOpAddon),
            AddonSpec {
                id: 100,
                capability: CapabilityToken::empty(),
                max_actions_per_tick: 50,
            },
        ),
        (
            Box::new(EchoAddon),
            AddonSpec {
                id: 101,
                capability: CapabilityToken::empty(),
                max_actions_per_tick: 50,
            },
        ),
    ];

    if config.enable_warmup {
        addons.push((
            Box::new(PreloadAddon::new()),
            AddonSpec {
                id: 102,
                capability: CapabilityToken::allow_all(), 
                max_actions_per_tick: 100,
            },
        ));
    }

    let mut effect_executor =
        crate::runtime::EffectExecutor::new(reactor, "/data/local/tmp/coreshift");

    let mut capabilities = CapabilityRegistry::new();
    capabilities.insert(0, CapabilityToken::allow_all()); // System / IPC root

    // Capability assignment for addons
    for (_, spec) in &addons {
        capabilities.insert(spec.id, spec.capability.clone());
    }

    let mut ipc = IpcModule::new(ipc_fd, ipc_token);

    let mut inotify_fd_opt = None;
    if config.enable_warmup && let Ok(fd_obj) = effect_executor.reactor.setup_inotify() {
        let inotify_fd = fd_obj.raw();

        let cgroup_path = std::ffi::CString::new("/dev/cpuset/top-app/cgroup.procs").unwrap();
        let _wd_cgroup = unsafe {
            libc::inotify_add_watch(
                inotify_fd,
                cgroup_path.as_ptr(),
                libc::IN_CLOSE_WRITE | libc::IN_MODIFY,
            )
        };

        let pkg_xml_path = std::ffi::CString::new("/data/system/packages.xml").unwrap();
        let _wd_pkg_xml = unsafe {
            libc::inotify_add_watch(
                inotify_fd,
                pkg_xml_path.as_ptr(),
                libc::IN_MODIFY | libc::IN_CREATE | libc::IN_DELETE,
            )
        };

        let pkg_list_path = std::ffi::CString::new("/data/system/packages.list").unwrap();
        let _wd_pkg_list = unsafe {
            libc::inotify_add_watch(
                inotify_fd,
                pkg_list_path.as_ptr(),
                libc::IN_MODIFY | libc::IN_CREATE | libc::IN_DELETE,
            )
        };

        inotify_fd_opt = Some((fd_obj, _wd_cgroup, _wd_pkg_xml, _wd_pkg_list));
    }

    let mut next_action_id = 1u64;
    let mut trace_store = TraceStore::new();
    let mut pending_events = Vec::new();
    let mut next_events = Vec::new();

    let mut record_file = config.record_path.map(|p| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .expect("Failed to open record file")
    });

    let mut last_tick_time = std::time::Instant::now();
    let mut tick_counter = 0u64;

    loop {
        tick_counter += 1;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_tick_time).as_millis() as u64;

        const TICK_MS: u64 = 16;
        let ticks = elapsed / TICK_MS;

        let mut sys_events = Vec::new();
        for _ in 0..ticks {
            sys_events.push(crate::core::Event::TimeAdvanced(TICK_MS));
        }

        if ticks > 0 {
            last_tick_time += std::time::Duration::from_millis(ticks * TICK_MS);
        }

        let mut reactor_events = Vec::new();
        let timeout = core.dispatcher.compute_timeout_ms(&state);

        let reactor_res = effect_executor.process_reactor_events(&mut reactor_events, timeout);
        match reactor_res {
            Ok(evs) => {
                sys_events.extend(evs);
            }
            Err(e) => {
                sys_events.push(crate::core::Event::ReactorError {
                    err: format!("reactor wait failed: {}", e),
                });
            }
        }

        sys_events.append(&mut pending_events);

        let mut scheduler = crate::core::scheduler::Scheduler::new(MAX_ACTIONS_PER_TICK);

        // Phase 1: Collect
        let mut collected_actions: Vec<(crate::core::Action, crate::core::ActionMeta)> = Vec::new();

        let _ = effect_executor.apply(crate::core::Effect::Log {
            owner: crate::core::CORE_OWNER,
            level: crate::core::LogLevel::Info,
            event: crate::core::LogEvent::TickStart,
        });

        for rev in reactor_events {
            let ipc_msgs = ipc.handle_event(&mut effect_executor.reactor, &rev);
            let mut ipc_intents = Vec::new();
            for msg in ipc_msgs {
                let uid = msg.uid;
                let cmd = msg.command;
                let client_id = msg.client_id;

                let intent_opt = match cmd {
                    crate::high_level::api::Command::Cmd { .. }
                    | crate::high_level::api::Command::Dumpsys { .. } => {
                        let (exec, _policy) = cmd.map_to_exec();
                        let id = next_action_id;
                        next_action_id += 1;
                        Some(crate::core::Intent::Submit {
                            id,
                            owner: 0,
                            job: crate::core::JobRequest { command: exec.argv },
                        })
                    }
                    crate::high_level::api::Command::GetResult { id } => {
                        Some(crate::core::Intent::Query { id })
                    }
                    crate::high_level::api::Command::Cancel { id } => {
                        Some(crate::core::Intent::Control {
                            id,
                            signal: crate::core::ControlSignal::GracefulStop,
                        })
                    }
                };

                if let Some(mut intent) = intent_opt {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    if let crate::core::Intent::Submit { ref mut id, .. } = intent {
                        *id = cause.0;
                    }

                    ipc_intents.push(crate::high_level::identity::Request {
                        principal: crate::high_level::identity::Principal::new_user(uid),
                        client_id: Some(client_id),
                        cause,
                        intent,
                    });
                }
            }

            let mut addon_reqs = Vec::new();
            for (addon, spec) in &mut addons {
                let reqs = addon.on_reactor_event(&state, &rev);
                let count = std::cmp::min(reqs.len(), spec.max_actions_per_tick as usize);
                for mut req in reqs.into_iter().take(count) {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    if let crate::core::Intent::Submit { ref mut id, .. } = req.intent {
                        *id = cause.0;
                    }

                    req.cause = cause;
                    req.principal = crate::high_level::identity::Principal::Addon(spec.id);
                    req.client_id = None;
                    addon_reqs.push(req);
                }
            }

            let all_reqs = ipc_intents.into_iter().chain(addon_reqs.into_iter());

            for req in all_reqs {
                if crate::core::validation::validate_request(&req, &state).is_ok() {
                    // We expand first, then check caps for each action.
                    let actions = crate::core::expand_intent(req.intent.clone(), state.clock);
                    let mut allowed = true;
                    for action in &actions {
                        if !capabilities.allows(&req.principal, action.kind()) {
                            allowed = false;
                            break;
                        }
                    }

                    if allowed {
                        if let Some(f) = &mut record_file {
                            let _ = bincode::serialize_into(
                                f,
                                &crate::core::replay::ReplayInput::Intent(
                                    req.principal.clone(),
                                    req.intent.clone(),
                                ),
                            );
                        }
                        for action in actions {
                            collected_actions.push((
                                action,
                                crate::core::ActionMeta {
                                    id: req.cause,
                                    parent: Some(req.cause),
                                    source: req.principal.clone(),
                                    reply_to: req.client_id,
                                },
                            ));
                        }
                    }
                }
            }

            if let Some((inotify_fd_obj, wd_cgroup, wd_pkg_xml, wd_pkg_list)) = &inotify_fd_opt
                && Some(rev.token) == effect_executor.reactor.inotify_token
                && rev.readable
            {
                let mut fds = libc::pollfd {
                    fd: inotify_fd_obj.raw(),
                    events: libc::POLLIN,
                    revents: 0,
                };

                let ret = unsafe { libc::poll(&mut fds, 1, 0) };
                if ret > 0 {
                    let mut len: libc::c_int = 0;
                    if unsafe { libc::ioctl(inotify_fd_obj.raw(), libc::FIONREAD, &mut len) } >= 0
                        && len > 0
                    {
                        let mut buf = vec![0u8; len as usize];
                        let n = unsafe {
                            libc::read(
                                inotify_fd_obj.raw(),
                                buf.as_mut_ptr() as *mut libc::c_void,
                                len as usize,
                            )
                        };

                        if n > 0 {
                            let mut offset = 0;
                            let mut cgroup_changed = false;
                            let mut packages_changed = false;
                            let base = std::mem::size_of::<libc::inotify_event>();

                            while offset + base <= n as usize {
                                let event = unsafe {
                                    &*(buf.as_ptr().add(offset) as *const libc::inotify_event)
                                };
                                let size = base + event.len as usize;

                                if offset + size > n as usize {
                                    break;
                                }

                                if event.wd == *wd_pkg_xml || event.wd == *wd_pkg_list {
                                    packages_changed = true;
                                } else if event.wd == *wd_cgroup {
                                    cgroup_changed = true;
                                }

                                offset += size;
                            }

                            if packages_changed {
                                sys_events.push(crate::core::Event::PackagesChanged);
                            }

                            if cgroup_changed
                                && let Ok(cgroup_content) =
                                    std::fs::read_to_string("/dev/cpuset/top-app/cgroup.procs")
                            {
                                for pid_str in cgroup_content.split_whitespace() {
                                    if let Ok(pid) = pid_str.parse::<i32>() {
                                        sys_events.push(crate::core::Event::ForegroundChanged { pid });
                                    }
                                }
                            }
                        } else if n < 0 {
                            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                            let _ = effect_executor.apply(crate::core::Effect::Log {
                                owner: crate::core::CORE_OWNER,
                                level: crate::core::LogLevel::Error,
                                event: crate::core::LogEvent::Error {
                                    id: 0,
                                    err: format!("READ_ERR {}", err),
                                },
                            });
                        }
                    }
                }
            }
        }

        sys_events.push(crate::core::Event::Tick);

        while !sys_events.is_empty() || !collected_actions.is_empty() {
            if !sys_events.is_empty() {
                let current_events = std::mem::take(&mut sys_events);
                for ev in current_events {
                    if let Some(f) = &mut record_file {
                        let _ = bincode::serialize_into(
                            f,
                            &crate::core::replay::ReplayInput::Event(ev.clone()),
                        );
                    }

                    // Feed core events to addons
                    for (addon, spec) in &mut addons {
                        let reqs = addon.on_core_event(&state, &ev);
                        for mut req in reqs {
                            let cause = crate::core::CauseId(next_action_id);
                            next_action_id += 1;
                            trace_store.insert(cause, None);
                            if let crate::core::Intent::Submit { ref mut id, .. } = req.intent {
                                *id = cause.0;
                            }
                            req.cause = cause;
                            req.principal = crate::high_level::identity::Principal::Addon(spec.id);
                            
                            // Validate and Expand addon requests immediately
                            if crate::core::validation::validate_request(&req, &state).is_ok() {
                                let actions = crate::core::expand_intent(req.intent, state.clock);
                                for action in actions {
                                    if capabilities.allows(&req.principal, action.kind()) {
                                        collected_actions.push((action, crate::core::ActionMeta {
                                            id: req.cause,
                                            parent: Some(req.cause),
                                            source: req.principal.clone(),
                                            reply_to: None,
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    let sys_actions = core.dispatcher.dispatch_event(&state, &ev);
                    for action in sys_actions {
                        let cause = crate::core::CauseId(next_action_id);
                        next_action_id += 1;
                        trace_store.insert(cause, None);
                        collected_actions.push((
                            action,
                            crate::core::ActionMeta {
                                id: cause,
                                parent: None,
                                source: crate::high_level::identity::Principal::System,
                                reply_to: None,
                            },
                        ));
                    }
                }
            }

            if !collected_actions.is_empty() {
                let current_actions = std::mem::take(&mut collected_actions);
                for (action, meta) in current_actions {
                    debug_assert!(
                        capabilities.allows(&meta.source, action.kind()),
                        "Capability enforcement before enqueue"
                    );
                    if !capabilities.allows(&meta.source, action.kind()) {
                        continue;
                    }

                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(meta.id));

                    let new_meta = crate::core::ActionMeta {
                        id: next_id,
                        parent: Some(meta.id),
                        source: meta.source.clone(),
                        reply_to: meta.reply_to,
                    };
                    if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                        action,
                        meta: new_meta,
                    }) {
                        sys_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }
        }

        if tick_counter % 64 == 0 {
            crate::core::verify::verify_global(&state);
        }

        // Phase 3: Enqueue (Merged into Phase 2 loop above)

        // Phase 4: Resolve
        let mut generated_effects = Vec::with_capacity(16);
        let mut action_effects = Vec::with_capacity(16);
        let mut tick_actions_processed = 0;
        let mut tick_dropped_actions = 0;
        let mut per_source_count: std::collections::HashMap<
            crate::high_level::identity::Principal,
            usize,
        > = std::collections::HashMap::new();

        loop {
            let mut made_progress = false;

            while let Some(routed) = scheduler.next() {
                debug_assert!(true, "Action has source (in rust we checked this statically)");

                let count = per_source_count
                    .entry(routed.meta.source.clone())
                    .or_insert(0);
                if *count >= 256 {
                    tick_dropped_actions += 1;

                    continue; // Drop action due to source limit
                }
                *count += 1;

                ipc.intercept_action(&routed.action, routed.meta.reply_to);

                tick_actions_processed += 1;

                action_effects.clear();

                if let Some(indices) = core.routing.get(&routed.action.kind()) {
                    for &idx in indices {
                        let reducer = &core.reducers[idx];
                        let mut ctx = crate::core::reducer::ReducerCtx {
                            core: &mut state.core,
                            timeout: &mut state.timeout,
                            result: &mut state.result,
                            clock: &mut state.clock,
                        };
                        reducer.apply(&mut ctx, &routed.action, &mut action_effects);
                    }
                }

                for effect in action_effects.drain(..) {
                    generated_effects.push(effect);
                }
                state.update_hash();

                if !matches!(routed.action, crate::core::Action::EmitLog { .. }) {
                    let _ = effect_executor.apply(crate::core::Effect::Log {
                        owner: crate::core::CORE_OWNER,
                        level: crate::core::LogLevel::Info,
                        event: crate::core::LogEvent::ActionDispatched,
                    });
                }

                let new_actions = core.dispatcher.dispatch(&state, &routed.action);

                for action in new_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(routed.meta.id));
                    if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                        action,
                        meta: crate::core::ActionMeta {
                            id: next_id,
                            parent: Some(routed.meta.id),
                            source: routed.meta.source.clone(),
                            reply_to: routed.meta.reply_to,
                        },
                    }) {
                        sys_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }

            if !sys_events.is_empty() || !collected_actions.is_empty() {
                made_progress = true;
                while !sys_events.is_empty() || !collected_actions.is_empty() {
                    if !sys_events.is_empty() {
                        let current_events = std::mem::take(&mut sys_events);
                        for ev in current_events {
                            if let Some(f) = &mut record_file {
                                let _ = bincode::serialize_into(
                                    f,
                                    &crate::core::replay::ReplayInput::Event(ev.clone()),
                                );
                            }
                            let sys_actions = core.dispatcher.dispatch_event(&state, &ev);
                            for action in sys_actions {
                                let cause = crate::core::CauseId(next_action_id);
                                next_action_id += 1;
                                trace_store.insert(cause, None);
                                collected_actions.push((
                                    action,
                                    crate::core::ActionMeta {
                                        id: cause,
                                        parent: None,
                                        source: crate::high_level::identity::Principal::System,
                                        reply_to: None,
                                    },
                                ));
                            }
                        }
                    }

                    if !collected_actions.is_empty() {
                        let current_actions = std::mem::take(&mut collected_actions);
                        for (action, meta) in current_actions {
                            debug_assert!(
                                capabilities.allows(&meta.source, action.kind()),
                                "Capability enforcement before enqueue"
                            );
                            if !capabilities.allows(&meta.source, action.kind()) {
                                continue;
                            }

                            let next_id = crate::core::CauseId(next_action_id);
                            next_action_id += 1;
                            trace_store.insert(next_id, Some(meta.id));

                            if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                                action,
                                meta: crate::core::ActionMeta {
                                    id: next_id,
                                    parent: Some(meta.id), // Here cause is already bound to ingress
                                    source: meta.source.clone(),
                                    reply_to: meta.reply_to,
                                },
                            }) {
                                sys_events.push(ev);
                            }
                        }
                    }
                }
            }

            if !made_progress {
                break;
            }
        } // Close resolve loop

        generated_effects.push(crate::core::Effect::Log {
            owner: crate::core::CORE_OWNER,
            level: crate::core::LogLevel::Info,
            event: crate::core::LogEvent::TickEnd,
        });

        // Apply effects at the end of the tick boundary to avoid interleaving events
        for effect in generated_effects {
            let events = effect_executor.apply(effect);
            next_events.extend(events);
        }

        std::mem::swap(&mut pending_events, &mut next_events);
        next_events.clear();

        if state.clock % 100 == 0 {
            let _ = effect_executor.apply(crate::core::Effect::Log {
                owner: crate::core::CORE_OWNER,
                level: crate::core::LogLevel::Info,
                event: crate::core::LogEvent::Observability {
                    queue_len: scheduler.total_len,
                    actions_processed: tick_actions_processed,
                    dropped: tick_dropped_actions,
                },
            });
        }

        if let Some(f) = &mut record_file {
            let stats = crate::core::replay::TickStats {
                hash: state.hash,
                actions_processed: tick_actions_processed,
                dropped_actions: tick_dropped_actions,
            };
            let _ = bincode::serialize_into(f, &crate::core::replay::ReplayInput::TickEnd(stats));
        }
    }
}

pub fn run_replay(path: &str) -> u64 {
    use crate::core::replay::ReplayInput;
    use crate::core::{Core, ExecutionState};
    use std::fs::File;
    use std::io::BufReader;

    let mut state = ExecutionState::new();
    let core = Core::new();
    let mut scheduler = crate::core::scheduler::Scheduler::new(10_000);
    let mut trace_store = TraceStore::new();
    let mut next_action_id = 1u64;

    let file = File::open(path).expect("Failed to open replay file");
    let mut reader = BufReader::new(file);

    let mut inputs = Vec::new();
    while let Ok(input) = bincode::deserialize_from::<_, ReplayInput>(&mut reader) {
        inputs.push(input);
    }

    let mut tick_idx = 0;
    let mut current_input_idx = 0;

    let mut action_effects = Vec::with_capacity(16);

    while current_input_idx < inputs.len() {
        let mut tick_events = Vec::new();
        let mut tick_intents: Vec<crate::high_level::identity::Request> = Vec::new();
        let mut expected_hash = None;

        // Replay modes natively serialize TimeAdvanced via the sys_events queue.
        // We gracefully ignore the legacy `Time` format, if any.

        while current_input_idx < inputs.len() {
            match &inputs[current_input_idx] {
                ReplayInput::Time(dur) => {
                    // Legacy support: map old time format to explicit constant TICK_MS quantization.
                    let elapsed = dur.as_millis() as u64;
                    let ticks = elapsed / 16;
                    for _ in 0..ticks {
                        tick_events.push(crate::core::Event::TimeAdvanced(16));
                    }
                }
                ReplayInput::TickHash(h) => {
                    expected_hash = Some(crate::core::replay::TickStats {
                        hash: *h,
                        actions_processed: 0,
                        dropped_actions: 0,
                    });
                    current_input_idx += 1;
                    break;
                }
                ReplayInput::TickEnd(stats) => {
                    expected_hash = Some(stats.clone());
                    current_input_idx += 1;
                    break;
                }
                ReplayInput::Event(e) => {
                    tick_events.push(e.clone());
                }
                ReplayInput::LegacyIntent(i) => {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    let mut cloned_intent = i.clone();
                    if let crate::core::Intent::Submit { ref mut id, .. } = cloned_intent {
                        *id = cause.0;
                    }

                    tick_intents.push(crate::high_level::identity::Request {
                        principal: crate::high_level::identity::Principal::System,
                        client_id: None,
                        cause,
                        intent: cloned_intent,
                    });
                }
                ReplayInput::Intent(p, i) => {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    let mut cloned_intent = i.clone();
                    if let crate::core::Intent::Submit { ref mut id, .. } = cloned_intent {
                        *id = cause.0;
                    }

                    tick_intents.push(crate::high_level::identity::Request {
                        principal: p.clone(),
                        client_id: None,
                        cause,
                        intent: cloned_intent,
                    });
                }
            }
            current_input_idx += 1;
        }

        for req in tick_intents {
            let actions = crate::core::expand_intent(req.intent, state.clock);
            for action in actions {
                let next_id = crate::core::CauseId(next_action_id);
                trace_store.insert(next_id, None);
                if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                    action,
                    meta: crate::core::ActionMeta {
                        id: next_id,
                        parent: None,
                        source: req.principal.clone(),
                        reply_to: req.client_id,
                    },
                }) {
                    tick_events.push(ev);
                }
                next_action_id += 1;
            }
        }

        // Ensure any intents that dropped actions get their dropped actions processed
        let mut collected_actions: Vec<crate::core::Action> = Vec::new();
        while !tick_events.is_empty() || !collected_actions.is_empty() {
            if !tick_events.is_empty() {
                let current_events = std::mem::take(&mut tick_events);
                for event in current_events {
                    let actions = core.dispatcher.dispatch_event(&state, &event);
                    for action in actions {
                        collected_actions.push(action);
                    }
                }
            }

            if !collected_actions.is_empty() {
                let current_actions = std::mem::take(&mut collected_actions);
                for action in current_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, None);
                    if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                        action,
                        meta: crate::core::ActionMeta {
                            id: next_id,
                            parent: None,
                            source: crate::high_level::identity::Principal::System,
                            reply_to: None,
                        },
                    }) {
                        tick_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }
        }

        let mut tick_actions_processed = 0;
        let mut tick_dropped_actions = 0;
        let mut per_source_count: std::collections::HashMap<
            crate::high_level::identity::Principal,
            usize,
        > = std::collections::HashMap::new();

        loop {
            let mut made_progress = false;

            while let Some(routed) = scheduler.next() {
                let count = per_source_count
                    .entry(routed.meta.source.clone())
                    .or_insert(0);
                if *count >= 256 {
                    tick_dropped_actions += 1;

                    continue; // Drop action due to source limit
                }
                *count += 1;

                tick_actions_processed += 1;

                action_effects.clear();

                if let Some(indices) = core.routing.get(&routed.action.kind()) {
                    for &idx in indices {
                        let reducer = &core.reducers[idx];
                        let mut ctx = crate::core::reducer::ReducerCtx {
                            core: &mut state.core,
                            timeout: &mut state.timeout,
                            result: &mut state.result,
                            clock: &mut state.clock,
                        };
                        reducer.apply(&mut ctx, &routed.action, &mut action_effects);
                        // Drop effects in replay mode
                    }
                }
                state.update_hash();

                let new_actions = core.dispatcher.dispatch(&state, &routed.action);
                for action in new_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(routed.meta.id));
                    if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                        action,
                        meta: crate::core::ActionMeta {
                            id: next_id,
                            parent: Some(routed.meta.id),
                            source: routed.meta.source.clone(),
                            reply_to: routed.meta.reply_to,
                        },
                    }) {
                        tick_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }

            // Pending events dropped by scheduler during replay are collected here,
            // but normally they would go into the next tick.

            if !tick_events.is_empty() || !collected_actions.is_empty() {
                made_progress = true;
                while !tick_events.is_empty() || !collected_actions.is_empty() {
                    if !tick_events.is_empty() {
                        let current_events = std::mem::take(&mut tick_events);
                        for event in current_events {
                            let actions = core.dispatcher.dispatch_event(&state, &event);
                            for action in actions {
                                collected_actions.push(action);
                            }
                        }
                    }

                    if !collected_actions.is_empty() {
                        let current_actions = std::mem::take(&mut collected_actions);
                        for action in current_actions {
                            let next_id = crate::core::CauseId(next_action_id);
                            trace_store.insert(next_id, None);
                            if let Some(ev) = scheduler.enqueue(crate::core::RoutedAction {
                                action,
                                meta: crate::core::ActionMeta {
                                    id: next_id,
                                    parent: None,
                                    source: crate::high_level::identity::Principal::System,
                                    reply_to: None,
                                },
                            }) {
                                tick_events.push(ev);
                            }
                            next_action_id += 1;
                        }
                    }
                }
            }

            if !made_progress {
                break;
            }
        }

        if let Some(expected) = expected_hash {
            let actual = state.hash;
            assert_eq!(
                actual, expected.hash,
                "Determinism violation: state hash diverged at tick {}",
                tick_idx
            );
            // Ignore stats matching for legacy hashes
            if expected.actions_processed > 0 || expected.dropped_actions > 0 {
                assert_eq!(
                    tick_actions_processed, expected.actions_processed,
                    "Determinism violation: actions processed diverged at tick {}",
                    tick_idx
                );
                assert_eq!(
                    tick_dropped_actions, expected.dropped_actions,
                    "Determinism violation: dropped actions diverged at tick {}",
                    tick_idx
                );
            }
            tick_idx += 1;
        }
    }

    println!("Replay finished deterministically.");
    state.hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_replay() {
        use crate::core::replay::{ReplayInput, TickStats};

        let path = "test_replay.bin";
        let mut file = std::fs::File::create(path).unwrap();

        // Write some dummy inputs
        let stats = TickStats {
            hash: 0,
            actions_processed: 0,
            dropped_actions: 0,
        };
        bincode::serialize_into(&mut file, &ReplayInput::TickEnd(stats)).unwrap();

        let hash1 = run_replay(path);
        let hash2 = run_replay(path);

        assert_eq!(hash1, hash2);

        let _ = std::fs::remove_file(path);
    }
}

```

---

## `rust/src/low_level/io/buffer.rs`

```rs
use crate::low_level::reactor::Fd;
use crate::low_level::spawn::SysError;

const READ_CHUNK: usize = 65536;

#[derive(Default)]
#[repr(align(64))]
pub struct BufferState {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub limit: usize,
}

impl BufferState {
    pub fn new(limit: usize) -> Self {
        Self {
            stdout: Vec::with_capacity(1024),
            stderr: Vec::with_capacity(1024),
            limit,
        }
    }

    #[inline(always)]
    pub fn read_from_fd(
        &mut self,
        fd: &Fd,
        is_stdout: bool,
        early_exit: &mut Option<impl FnMut(&[u8]) -> bool>,
    ) -> Result<bool, SysError> {
        let dest = if is_stdout {
            &mut self.stdout
        } else {
            &mut self.stderr
        };

        loop {
            let cap = dest.capacity();
            let len = dest.len();
            let remaining_limit = self.limit.saturating_sub(len);

            if remaining_limit == 0 {
                // Limit reached, just discard data.
                let mut drop_buf = [0u8; 8192];
                match fd.read(drop_buf.as_mut_ptr(), drop_buf.len()) {
                    Ok(n) if n > 0 => continue,
                    Ok(_) => {
                        return Ok(true); // EOF
                    }
                    Err(SysError::Syscall { code, .. })
                        if code == libc::EAGAIN || code == libc::EWOULDBLOCK =>
                    {
                        return Ok(false);
                    } // Would block
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            // Ensure capacity and read directly into uninitialized space
            let to_read = remaining_limit.min(READ_CHUNK);
            if cap - len < to_read {
                dest.reserve(to_read);
            }

            let ptr = unsafe { dest.as_mut_ptr().add(len) };
            match fd.read(ptr, to_read) {
                Ok(n) if n > 0 => {
                    unsafe {
                        dest.set_len(len + n);
                    }

                    if is_stdout
                        && let Some(f) = early_exit
                        && f(&dest[len..len + n])
                    {
                        return Ok(true); // Early exit implies EOF/done
                    }
                }
                Ok(_) => {
                    return Ok(true); // EOF
                }
                Err(SysError::Syscall { code, .. })
                    if code == libc::EAGAIN || code == libc::EWOULDBLOCK =>
                {
                    return Ok(false);
                } // Would block
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    pub fn into_parts(mut self) -> (Vec<u8>, Vec<u8>) {
        (
            std::mem::take(&mut self.stdout),
            std::mem::take(&mut self.stderr),
        )
    }
}

```

---

## `rust/src/low_level/io/drain.rs`

```rs
use crate::low_level::io::buffer::BufferState;
use crate::low_level::io::writer::WriterState;
use crate::low_level::reactor::{Fd, Token};
use crate::low_level::spawn::SysError;

pub struct FdSlot {
    pub token: Option<Token>,
    pub fd: Fd,
}

#[repr(align(64))]
pub struct DrainState<F>
where
    F: FnMut(&[u8]) -> bool,
{
    pub stdout_slot: Option<FdSlot>,
    pub stderr_slot: Option<FdSlot>,
    pub stdin_slot: Option<FdSlot>,

    pub buffer: BufferState,
    pub writer: WriterState,

    pub early_exit: Option<F>,
}

impl<F> DrainState<F>
where
    F: FnMut(&[u8]) -> bool,
{
    pub fn new(
        _job_id: u64,
        stdin_fd: Option<Fd>,
        stdin_buf: Option<Box<[u8]>>,
        stdout_fd: Option<Fd>,
        stderr_fd: Option<Fd>,
        limit: usize,
        early_exit: Option<F>,
    ) -> Result<Self, SysError> {
        let mut stdin_slot = None;
        let mut stdout_slot = None;
        let mut stderr_slot = None;

        // Tokens remain purely unassigned until explicitly mapped by a Reactor
        if let (Some(fd), Some(_)) = (&stdin_fd, &stdin_buf) {
            fd.set_nonblock()?;
            stdin_slot = Some(FdSlot {
                token: None,
                fd: stdin_fd.unwrap(),
            });
        }

        if let Some(fd) = &stdout_fd {
            fd.set_nonblock()?;
            stdout_slot = Some(FdSlot {
                token: None,
                fd: stdout_fd.unwrap(),
            });
        }

        if let Some(fd) = &stderr_fd {
            fd.set_nonblock()?;
            stderr_slot = Some(FdSlot {
                token: None,
                fd: stderr_fd.unwrap(),
            });
        }

        Ok(Self {
            stdin_slot,
            stdout_slot,
            stderr_slot,
            buffer: BufferState::new(limit),
            writer: WriterState::new(stdin_buf),
            early_exit,
        })
    }

    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.stdin_slot.is_none() && self.stdout_slot.is_none() && self.stderr_slot.is_none()
    }

    #[inline(always)]
    pub fn write_stdin(&mut self) -> Result<Option<FdSlot>, SysError> {
        let fd = if let Some(s) = &self.stdin_slot {
            &s.fd
        } else {
            return Ok(None);
        };

        let done = self.writer.write_to_fd(fd)?;
        if done {
            let slot = self.stdin_slot.take();
            return Ok(slot);
        }
        Ok(None)
    }

    #[inline(always)]
    pub fn read_fd(&mut self, is_stdout: bool) -> Result<Option<FdSlot>, SysError> {
        let eof = {
            let slot = if is_stdout {
                &self.stdout_slot
            } else {
                &self.stderr_slot
            };
            let fd = if let Some(s) = slot {
                &s.fd
            } else {
                return Ok(None);
            };
            self.buffer
                .read_from_fd(fd, is_stdout, &mut self.early_exit)?
        };

        if eof {
            if is_stdout {
                let slot = self.stdout_slot.take();
                return Ok(slot);
            } else {
                let slot = self.stderr_slot.take();
                return Ok(slot);
            }
        }

        Ok(None)
    }

    pub fn take_all_slots(&mut self) -> Vec<FdSlot> {
        let mut slots = Vec::new();
        if let Some(slot) = self.stdin_slot.take() {
            slots.push(slot);
        }
        if let Some(slot) = self.stdout_slot.take() {
            slots.push(slot);
        }
        if let Some(slot) = self.stderr_slot.take() {
            slots.push(slot);
        }
        slots
    }

    pub fn into_parts(mut self) -> (Vec<u8>, Vec<u8>) {
        std::mem::take(&mut self.buffer).into_parts()
    }
}

```

---

## `rust/src/low_level/io/mod.rs`

```rs
pub mod buffer;
pub mod drain;
pub mod writer;
pub use drain::DrainState;

```

---

## `rust/src/low_level/io/writer.rs`

```rs
use crate::low_level::reactor::Fd;
use crate::low_level::spawn::SysError;

const WRITE_CHUNK: usize = 65536;

pub struct WriterState {
    pub buf: Option<Box<[u8]>>,
    pub off: usize,
}

impl WriterState {
    pub fn new(buf: Option<Box<[u8]>>) -> Self {
        Self { buf, off: 0 }
    }

    #[inline(always)]
    pub fn write_to_fd(&mut self, fd: &Fd) -> Result<bool, SysError> {
        if let Some(buf) = &self.buf {
            while self.off < buf.len() {
                let remaining = buf.len() - self.off;
                let chunk = remaining.min(WRITE_CHUNK);

                match fd.write(buf[self.off..].as_ptr(), chunk) {
                    Ok(n) if n > 0 => {
                        self.off += n;
                    }
                    Ok(_) => {
                        self.buf = None;
                        return Ok(true); // Done
                    }
                    Err(e) => {
                        let SysError::Syscall { code, .. } = &e;
                        if *code == libc::EAGAIN || *code == libc::EWOULDBLOCK {
                            return Ok(false); // Would block
                        } else if *code == libc::EPIPE {
                            self.buf = None;
                            return Ok(true); // Broken pipe (treat as end of write stream)
                        } else {
                            self.buf = None;
                            return Err(e); // Propagate actual error
                        }
                    }
                }
            }
            // Done writing
            self.buf = None;
            return Ok(true);
        }
        Ok(true)
    }
}

```

---

## `rust/src/low_level/reactor/mod.rs`

```rs
use crate::low_level::spawn::{SysError, syscall_ret};
use std::io::Error as IoError;

#[inline(always)]
fn errno() -> i32 {
    IoError::last_os_error().raw_os_error().unwrap_or(0)
}

/// A safe wrapper for file descriptors ensuring they are closed when dropped.
#[derive(Clone)]
pub struct Fd(RawFd);

use std::os::unix::io::{AsRawFd, RawFd};

impl AsRawFd for Fd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl Fd {
    #[inline(always)]
    pub fn new(fd: RawFd, op: &'static str) -> Result<Self, SysError> {
        if fd < 0 {
            Err(SysError::sys(errno(), op))
        } else {
            Ok(Self(fd))
        }
    }

    #[inline(always)]
    pub fn raw(&self) -> RawFd {
        self.0
    }

    pub fn dup2(&self, target: RawFd) -> Result<(), SysError> {
        loop {
            let r = unsafe { libc::dup2(self.0, target) };
            if r < 0 {
                let e = errno();
                if e == libc::EINTR {
                    continue;
                }
                return syscall_ret(r, "dup2");
            }
            return Ok(());
        }
    }

    pub fn set_nonblock(&self) -> Result<(), SysError> {
        let flags = unsafe { libc::fcntl(self.0, libc::F_GETFL) };
        syscall_ret(flags, "fcntl(F_GETFL)")?;
        let r = unsafe { libc::fcntl(self.0, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        syscall_ret(r, "fcntl(F_SETFL)")
    }

    pub fn set_cloexec(&self) -> Result<(), SysError> {
        let flags = unsafe { libc::fcntl(self.0, libc::F_GETFD) };
        syscall_ret(flags, "fcntl(F_GETFD)")?;
        let r = unsafe { libc::fcntl(self.0, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
        syscall_ret(r, "fcntl(F_SETFD)")
    }

    pub fn read(&self, buf: *mut u8, count: usize) -> Result<usize, SysError> {
        loop {
            let n = unsafe { libc::read(self.0, buf as *mut libc::c_void, count) };
            if n < 0 {
                let e = errno();
                if e == libc::EINTR {
                    continue;
                }
                syscall_ret(-1, "read")?;
            }
            return Ok(n as usize);
        }
    }

    pub fn write(&self, buf: *const u8, count: usize) -> Result<usize, SysError> {
        loop {
            let n = unsafe { libc::write(self.0, buf as *const libc::c_void, count) };
            if n < 0 {
                let e = errno();
                if e == libc::EINTR {
                    continue;
                }
                syscall_ret(-1, "write")?;
            }
            return Ok(n as usize);
        }
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        if self.0 >= 0 {
            unsafe {
                libc::close(self.0);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token(pub u64);

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub token: Token,
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
}

pub struct Reactor {
    epfd: RawFd,
    next_token: u64,
    events_buf: Vec<libc::epoll_event>,
    signalfd: Option<Fd>,
    pub sigchld_token: Option<Token>,
    pub inotify_token: Option<Token>,
}

impl Reactor {
    pub fn new() -> Result<Self, SysError> {
        let epfd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        syscall_ret(epfd, "epoll_create1")?;
        Ok(Self {
            epfd,
            next_token: 1,
            events_buf: Vec::with_capacity(64),
            signalfd: None,
            sigchld_token: None,
            inotify_token: None,
        })
    }

    pub fn setup_inotify(&mut self) -> Result<Fd, SysError> {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        syscall_ret(fd, "inotify_init1")?;

        let fd_obj = Fd::new(fd, "inotify")?;
        let token = self.add(&fd_obj, true, false)?;
        self.inotify_token = Some(token);

        Ok(fd_obj)
    }

    pub fn setup_signalfd(&mut self) -> Result<(), SysError> {
        let mut mask: libc::sigset_t = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&mut mask) };
        unsafe { libc::sigaddset(&mut mask, libc::SIGCHLD) };

        // Block SIGCHLD so signalfd can intercept it
        let r = unsafe { libc::sigprocmask(libc::SIG_BLOCK, &mask, std::ptr::null_mut()) };
        syscall_ret(r, "sigprocmask")?;

        let sfd = unsafe { libc::signalfd(-1, &mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) };
        syscall_ret(sfd, "signalfd")?;

        let fd = Fd::new(sfd, "signalfd")?;
        let token = self.add(&fd, true, false)?;

        self.signalfd = Some(fd);
        self.sigchld_token = Some(token);

        Ok(())
    }

    pub fn drain_signalfd(&self) {
        if let Some(fd) = &self.signalfd {
            let mut buf = [0u8; std::mem::size_of::<libc::signalfd_siginfo>()];
            loop {
                match fd.read(buf.as_mut_ptr(), buf.len()) {
                    Ok(n) if n < buf.len() => break,
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        }
    }

    #[inline(always)]
    pub fn add(&mut self, fd: &Fd, readable: bool, writable: bool) -> Result<Token, SysError> {
        let token = Token(self.next_token);
        self.next_token += 1;
        self.add_with_token(fd.raw(), token, readable, writable)?;
        Ok(token)
    }

    #[inline(always)]
    pub fn add_with_token(
        &mut self,
        raw_fd: RawFd,
        token: Token,
        readable: bool,
        writable: bool,
    ) -> Result<(), SysError> {
        let mut events = libc::EPOLLET as u32;
        if readable {
            events |= libc::EPOLLIN as u32;
        }
        if writable {
            events |= libc::EPOLLOUT as u32;
        }
        let mut ev = libc::epoll_event {
            events,
            u64: token.0,
        };
        let r = unsafe { libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_ADD, raw_fd, &mut ev) };
        syscall_ret(r, "epoll_ctl_add")?;
        Ok(())
    }

    #[inline(always)]
    pub fn del(&self, fd: &Fd) {
        self.del_raw(fd.raw());
    }

    #[inline(always)]
    pub fn del_raw(&self, raw: RawFd) {
        unsafe {
            let _ = libc::epoll_ctl(self.epfd, libc::EPOLL_CTL_DEL, raw, std::ptr::null_mut());
        }
    }

    #[inline(always)]
    pub fn wait(
        &mut self,
        buffer: &mut Vec<Event>,
        max_events: usize,
        timeout: i32,
    ) -> Result<usize, SysError> {
        buffer.clear();

        // Ensure buffer has enough capacity
        if buffer.capacity() < max_events {
            buffer.reserve(max_events.saturating_sub(buffer.len()));
        }

        if self.events_buf.capacity() < max_events {
            self.events_buf
                .reserve(max_events.saturating_sub(self.events_buf.len()));
        }

        let n = unsafe {
            libc::epoll_wait(
                self.epfd,
                self.events_buf.as_mut_ptr(),
                max_events as i32,
                timeout,
            )
        };

        if n > 0 {
            for i in 0..n as usize {
                let ev = unsafe { *self.events_buf.as_ptr().add(i) };
                let is_read = (ev.events & libc::EPOLLIN as u32) != 0;
                let is_write = (ev.events & libc::EPOLLOUT as u32) != 0;
                let is_err = (ev.events & (libc::EPOLLERR | libc::EPOLLHUP) as u32) != 0;

                buffer.push(Event {
                    token: Token(ev.u64),
                    readable: is_read || is_err,
                    writable: is_write || is_err,
                    error: is_err,
                });
            }
            return Ok(n as usize);
        }

        if n < 0 {
            let e = errno();
            if e == libc::EINTR {
                return Ok(0);
            }
            return Err(SysError::sys(e, "epoll_wait"));
        }
        Ok(0)
    }

    pub fn fd(&self) -> RawFd {
        self.epfd
    }
}

impl Drop for Reactor {
    fn drop(&mut self) {
        if self.epfd >= 0 {
            unsafe {
                libc::close(self.epfd);
            }
        }
    }
}

```

---

## `rust/src/low_level/spawn/mod.rs`

```rs
use std::fmt;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

use crate::low_level::reactor::Fd;
use crate::low_level::sys::{CancelPolicy, ExecContext, ProcessGroup, SignalRuntime};
use libc::{
    O_CLOEXEC, O_NONBLOCK, WEXITSTATUS, WIFEXITED, WIFSIGNALED, WTERMSIG, c_char, pid_t, pipe2,
    waitpid,
};

unsafe extern "C" {
    pub static mut environ: *mut *mut libc::c_char;
}

#[cfg(target_os = "android")]
unsafe extern "C" {
    pub fn __system_property_get(
        name: *const libc::c_char,
        value: *mut libc::c_char,
    ) -> libc::c_int;
}

pub const POSIX_SPAWN_SETPGROUP: i32 = 2;
pub const POSIX_SPAWN_SETSIGDEF: i32 = 4;
pub const POSIX_SPAWN_SETSIGMASK: i32 = 8;

unsafe extern "C" {
    pub fn posix_spawn(
        pid: *mut libc::pid_t,
        path: *const libc::c_char,
        file_actions: *const libc::posix_spawn_file_actions_t,
        attrp: *const libc::posix_spawnattr_t,
        argv: *const *mut libc::c_char,
        envp: *const *mut libc::c_char,
    ) -> libc::c_int;

    pub fn posix_spawn_file_actions_addclose(
        file_actions: *mut libc::posix_spawn_file_actions_t,
        fd: libc::c_int,
    ) -> libc::c_int;

    pub fn posix_spawn_file_actions_adddup2(
        file_actions: *mut libc::posix_spawn_file_actions_t,
        fd: libc::c_int,
        newfd: libc::c_int,
    ) -> libc::c_int;

    pub fn posix_spawn_file_actions_destroy(
        file_actions: *mut libc::posix_spawn_file_actions_t,
    ) -> libc::c_int;

    pub fn posix_spawn_file_actions_init(
        file_actions: *mut libc::posix_spawn_file_actions_t,
    ) -> libc::c_int;

    pub fn posix_spawnattr_destroy(attr: *mut libc::posix_spawnattr_t) -> libc::c_int;

    pub fn posix_spawnattr_init(attr: *mut libc::posix_spawnattr_t) -> libc::c_int;

    pub fn posix_spawnattr_setflags(
        attr: *mut libc::posix_spawnattr_t,
        flags: libc::c_short,
    ) -> libc::c_int;

    pub fn posix_spawnattr_setpgroup(
        attr: *mut libc::posix_spawnattr_t,
        pgroup: libc::pid_t,
    ) -> libc::c_int;

    pub fn posix_spawnattr_setsigdefault(
        attr: *mut libc::posix_spawnattr_t,
        sigdefault: *const libc::sigset_t,
    ) -> libc::c_int;

    pub fn posix_spawnattr_setsigmask(
        attr: *mut libc::posix_spawnattr_t,
        sigmask: *const libc::sigset_t,
    ) -> libc::c_int;
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SysError {
    Syscall { code: i32, op: String },
}

impl SysError {
    pub fn sys(code: i32, op: &str) -> Self {
        SysError::Syscall {
            code,
            op: op.to_string(),
        }
    }
    pub fn raw_os_error(&self) -> Option<i32> {
        match self {
            SysError::Syscall { code, .. } => Some(*code),
        }
    }
}

impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syscall { code, op } => write!(f, "{} failed (code={})", op, code),
        }
    }
}

impl std::error::Error for SysError {}

#[inline(always)]
pub fn syscall_ret(ret: i32, op: &'static str) -> Result<(), SysError> {
    if ret == -1 {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        Err(SysError::sys(code, op))
    } else {
        Ok(())
    }
}

#[inline(always)]
pub fn posix_ret(ret: i32, op: &'static str) -> Result<(), SysError> {
    if ret != 0 {
        Err(SysError::sys(ret, op))
    } else {
        Ok(())
    }
}

#[inline(always)]
fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Creates a pipe with O_CLOEXEC | O_NONBLOCK flags.
/// Invariants: FDs returned are strictly non-negative and will close automatically on drop.
#[inline(always)]
fn make_pipe() -> Result<(Fd, Fd), SysError> {
    let mut fds = [0; 2];
    let r = unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK) };
    syscall_ret(r, "pipe2")?;
    Ok((Fd::new(fds[0], "pipe2")?, Fd::new(fds[1], "pipe2")?))
}

struct Pipes {
    stdin_r: Option<Fd>,
    stdin_w: Option<Fd>,
    stdout_r: Option<Fd>,
    stdout_w: Option<Fd>,
    stderr_r: Option<Fd>,
    stderr_w: Option<Fd>,
}

impl Pipes {
    fn new(in_buf: Option<&[u8]>, out: bool, err: bool) -> Result<Self, SysError> {
        let (stdin_r, stdin_w) = if in_buf.is_some() {
            let (r, w) = make_pipe()?;
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        let (stdout_r, stdout_w) = if out {
            let (r, w) = make_pipe()?;
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        let (stderr_r, stderr_w) = if err {
            let (r, w) = make_pipe()?;
            (Some(r), Some(w))
        } else {
            (None, None)
        };

        Ok(Self {
            stdin_r,
            stdin_w,
            stdout_r,
            stdout_w,
            stderr_r,
            stderr_w,
        })
    }

    #[inline(always)]
    fn close_all(&mut self) {
        self.stdin_r.take();
        self.stdin_w.take();
        self.stdout_r.take();
        self.stdout_w.take();
        self.stderr_r.take();
        self.stderr_w.take();
    }
}

/// Close FDs quickly.
/// Invariant: "FDs >= 3 are always closed in child except those specified to keep".
unsafe fn close_range_fast(keep_fd: Option<RawFd>) {
    #[cfg(target_os = "android")]
    {
        // try SYS_close_range (available on 5.9+)
        if let Some(fd) = keep_fd {
            let r1 = unsafe { libc::syscall(436, 3, (fd - 1).max(2) as libc::c_uint, 0) };
            let r2 = unsafe { libc::syscall(436, (fd + 1) as libc::c_uint, !0u32, 0) };
            if r1 == 0 && r2 == 0 {
                return;
            }
        } else {
            if unsafe { libc::syscall(436, 3, !0u32, 0) } == 0 {
                return;
            }
        }
    }
    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        if let Some(fd) = keep_fd {
            let r1 = unsafe {
                libc::syscall(libc::SYS_close_range, 3, (fd - 1).max(2) as libc::c_uint, 0)
            };
            let r2 =
                unsafe { libc::syscall(libc::SYS_close_range, (fd + 1) as libc::c_uint, !0u32, 0) };
            if r1 == 0 && r2 == 0 {
                return;
            }
        } else {
            if unsafe { libc::syscall(libc::SYS_close_range, 3, !0u32, 0) } == 0 {
                return;
            }
        }
    }

    let skip_fd = keep_fd.unwrap_or(-1);
    let dir_fd = unsafe {
        libc::open(
            b"/proc/self/fd\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if dir_fd >= 0 {
        let dir = unsafe { libc::fdopendir(dir_fd) };
        if !dir.is_null() {
            loop {
                let entry = unsafe { libc::readdir(dir) };
                if entry.is_null() {
                    break;
                }
                let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
                if let Ok(s) = name.to_str()
                    && let Ok(fd) = s.parse::<i32>()
                    && fd > 2
                    && fd != skip_fd
                    && fd != dir_fd
                    && fd >= 0
                {
                    unsafe {
                        libc::close(fd);
                    }
                }
            }
            unsafe {
                libc::closedir(dir);
            }
        } else {
            unsafe {
                libc::close(dir_fd);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(i32),
    Signaled(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnBackend {
    Auto,
    PosixSpawn,
    Fork,
}

#[inline(always)]
fn decode_status(status: i32) -> ExitStatus {
    if WIFEXITED(status) {
        ExitStatus::Exited(WEXITSTATUS(status))
    } else if WIFSIGNALED(status) {
        ExitStatus::Signaled(WTERMSIG(status))
    } else {
        ExitStatus::Exited(-1)
    }
}

#[derive(Clone)]
pub struct Process {
    pid: pid_t,
}

impl Process {
    pub fn new(pid: pid_t) -> Self {
        Self { pid }
    }

    pub fn pid(&self) -> pid_t {
        self.pid
    }

    pub fn wait_step(&self) -> Result<Option<ExitStatus>, SysError> {
        loop {
            let mut status = 0;
            let r = unsafe { waitpid(self.pid, &mut status, libc::WNOHANG) };
            if r == 0 {
                return Ok(None);
            }
            if r < 0 {
                let e = errno();
                if e == libc::EINTR {
                    continue;
                }
                if e == libc::ECHILD {
                    return Ok(Some(ExitStatus::Exited(0)));
                }
                syscall_ret(-1, "waitpid_step")?;
            }
            return Ok(Some(decode_status(status)));
        }
    }

    pub fn wait_blocking(&self) -> Result<ExitStatus, SysError> {
        loop {
            let mut status = 0;
            let r = unsafe { waitpid(self.pid, &mut status, 0) };
            if r < 0 {
                let e = errno();
                if e == libc::EINTR {
                    continue;
                }
                if e == libc::ECHILD {
                    return Ok(ExitStatus::Exited(0));
                }
                syscall_ret(-1, "waitpid_blocking")?;
            }
            return Ok(decode_status(status));
        }
    }

    pub fn kill(&self, sig: i32) -> Result<(), SysError> {
        let r = unsafe { libc::kill(self.pid, sig) };
        if r < 0 {
            let e = errno();
            if e == libc::ESRCH {
                return Ok(());
            }
            syscall_ret(-1, "kill")?;
        }
        Ok(())
    }

    pub fn kill_pgroup(&self, sig: i32) -> Result<(), SysError> {
        let r = unsafe { libc::kill(-self.pid, sig) };
        if r < 0 {
            let e = errno();
            if e == libc::ESRCH {
                return Ok(());
            }
            syscall_ret(-1, "kill_pgroup")?;
        }
        Ok(())
    }
}

pub struct SpawnOptions {
    pub ctx: ExecContext,
    pub stdin: Option<Box<[u8]>>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub wait: bool,
    pub pgroup: ProcessGroup,
    pub max_output: usize,
    pub timeout_ms: Option<u32>,
    pub kill_grace_ms: u32,
    pub cancel: CancelPolicy,
    pub backend: SpawnBackend,
    pub early_exit: Option<fn(&[u8]) -> bool>,
}

#[derive(Debug)]
pub struct Output {
    pub pid: pid_t,
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
enum Backend {
    PosixSpawn,
    Fork,
}

fn select_backend() -> Backend {
    static BACKEND: std::sync::OnceLock<Backend> = std::sync::OnceLock::new();
    *BACKEND.get_or_init(|| {
        #[cfg(target_os = "android")]
        {
            let mut value = [0u8; 92]; // PROP_VALUE_MAX = 92
            let name = b"ro.build.version.sdk\0";
            let len = unsafe {
                __system_property_get(
                    name.as_ptr() as *const libc::c_char,
                    value.as_mut_ptr() as *mut libc::c_char,
                )
            };
            if len > 0 {
                let s = std::str::from_utf8(&value[..len as usize]).unwrap_or("");
                if let Ok(api) = s.parse::<u32>() {
                    if api < 32 {
                        return Backend::Fork;
                    }
                }
            }
        }
        Backend::PosixSpawn
    })
}

#[inline(always)]
fn force_fork(opts: &SpawnOptions) -> bool {
    opts.pgroup.isolated || opts.ctx.cwd.is_some()
}

fn resolve_backend(opts: &SpawnOptions) -> Backend {
    if force_fork(opts) {
        return Backend::Fork;
    }

    match opts.backend {
        SpawnBackend::Auto => select_backend(),
        SpawnBackend::PosixSpawn => Backend::PosixSpawn,
        SpawnBackend::Fork => Backend::Fork,
    }
}

/// Spawns a new process with bounded execution constraints.
///
/// **Contracts & Limitations:**
/// - `backend` is advisory. The system will force a `Fork` fallback to maintain correctness
///   if incompatible options (like `cwd` or `setsid`) are requested.
/// - The `posix_spawn` path provides **best-effort FD isolation**, closing all tracked FDs up to `MAX_USED_FD`.
///   It is not strictly isolated from externally opened FDs without O_CLOEXEC.
/// - If `wait = false`, the caller is **strictly responsible** for reaping the child process.
/// - The standard stream read buffer uses a fixed-size stack allocation internally to prevent heap churn.
/// - No relative ordering or interleaving is guaranteed between the collected `stdout` and `stderr` buffers.
use crate::low_level::io::DrainState;

pub struct RunningProcess {
    pub process: Process,
    pub drain: DrainState<fn(&[u8]) -> bool>,
}

use crate::low_level::reactor::Reactor;

pub fn spawn_start(job_id: u64, opts: SpawnOptions) -> Result<RunningProcess, SysError> {
    let backend = resolve_backend(&opts);

    let (pid, drain) = match backend {
        Backend::PosixSpawn => spawn_posix_internal(job_id, opts)?,
        Backend::Fork => spawn_fork_internal(job_id, opts)?,
    };

    Ok(RunningProcess {
        process: Process::new(pid),
        drain,
    })
}

pub fn spawn(opts: SpawnOptions) -> Result<Output, SysError> {
    let wait = opts.wait;
    let timeout_ms = opts.timeout_ms;
    let kill_grace_ms = opts.kill_grace_ms;
    let cancel = opts.cancel;
    let pgroup = opts.pgroup;

    let mut reactor = Reactor::new()?;
    let running = spawn_start(0, opts)?; // ID=0 is arbitrary for synchronous unmanaged spawn

    let pid = running.process.pid();
    let mut drain = running.drain;

    // To prevent FD leak and adhere to ownership, we shouldn't use `mem::forget`.
    // Wait, wait_loop expects `drain` to still have slots!
    // But `take_all_slots` empties them. We should iterate mutably, or reassign tokens if we keep them in `drain`.
    if let Some(mut slot) = drain.stdin_slot.take() {
        slot.token = Some(reactor.add(&slot.fd, true, true)?);
        drain.stdin_slot = Some(slot);
    }
    if let Some(mut slot) = drain.stdout_slot.take() {
        slot.token = Some(reactor.add(&slot.fd, true, true)?);
        drain.stdout_slot = Some(slot);
    }
    if let Some(mut slot) = drain.stderr_slot.take() {
        slot.token = Some(reactor.add(&slot.fd, true, true)?);
        drain.stderr_slot = Some(slot);
    }

    if !wait {
        let (stdout, stderr) = drain.into_parts();
        return Ok(Output {
            pid,
            status: None,
            stdout,
            stderr,
            timed_out: false,
        });
    }

    wait_loop(
        pid,
        drain,
        reactor,
        timeout_ms,
        kill_grace_ms,
        cancel,
        pgroup,
    )
}

fn spawn_posix_internal(
    job_id: u64,
    opts: SpawnOptions,
) -> Result<(pid_t, DrainState<fn(&[u8]) -> bool>), SysError> {
    let mut pipes = Pipes::new(
        opts.stdin.as_deref(),
        opts.capture_stdout,
        opts.capture_stderr,
    )?;

    let exe_ptr = match &opts.ctx.argv {
        crate::low_level::sys::ExecArgv::Dynamic(v) => v[0].as_ptr(),
    };

    let argv = opts.ctx.get_argv_ptrs();
    let envp = opts.ctx.get_envp_ptrs();

    let actions = MaybeUninit::zeroed();
    let mut actions = unsafe { actions.assume_init() };
    if let Err(e) = posix_ret(
        unsafe { posix_spawn_file_actions_init(&mut actions) },
        "file_actions_init",
    ) {
        pipes.close_all();
        return Err(e);
    }

    struct Actions(*mut libc::posix_spawn_file_actions_t);
    impl Drop for Actions {
        fn drop(&mut self) {
            unsafe {
                posix_spawn_file_actions_destroy(self.0);
            }
        }
    }
    let _guard = Actions(&mut actions);

    if let (Some(r), Some(w)) = (&pipes.stdin_r, &pipes.stdin_w) {
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_adddup2(&mut actions, r.raw(), 0) },
            "dup2 stdin",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, r.raw()) },
            "close stdin pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, w.raw()) },
            "close stdin write pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
    }

    if let (Some(r), Some(w)) = (&pipes.stdout_r, &pipes.stdout_w) {
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_adddup2(&mut actions, w.raw(), 1) },
            "dup2 stdout",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, w.raw()) },
            "close stdout pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, r.raw()) },
            "close stdout read pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
    }

    if let (Some(r), Some(w)) = (&pipes.stderr_r, &pipes.stderr_w) {
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_adddup2(&mut actions, w.raw(), 2) },
            "dup2 stderr",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, w.raw()) },
            "close stderr pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
        if let Err(e) = posix_ret(
            unsafe { posix_spawn_file_actions_addclose(&mut actions, r.raw()) },
            "close stderr read pipe",
        ) {
            pipes.close_all();
            return Err(e);
        }
    }

    // Prevent FD leaks in posix_spawn by strictly closing open descriptors
    // instead of blindly closing all possible FDs.
    let dir_fd = unsafe {
        libc::open(
            b"/proc/self/fd\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if dir_fd >= 0 {
        let dir = unsafe { libc::fdopendir(dir_fd) };
        if !dir.is_null() {
            loop {
                let entry = unsafe { libc::readdir(dir) };
                if entry.is_null() {
                    break;
                }
                let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
                if let Ok(s) = name.to_str()
                    && let Ok(fd) = s.parse::<i32>()
                    && fd > 2
                    && fd != dir_fd
                {
                    // Note: actions run in the child process, so we close the fd there
                    unsafe {
                        posix_spawn_file_actions_addclose(&mut actions, fd);
                    }
                }
            }
            unsafe {
                libc::closedir(dir);
            }
        } else {
            unsafe {
                libc::close(dir_fd);
            }
        }
    }

    let attr = MaybeUninit::zeroed();
    let mut attr = unsafe { attr.assume_init() };
    if let Err(e) = posix_ret(unsafe { posix_spawnattr_init(&mut attr) }, "attr_init") {
        pipes.close_all();
        return Err(e);
    }

    struct Attr(*mut libc::posix_spawnattr_t);
    impl Drop for Attr {
        fn drop(&mut self) {
            unsafe {
                posix_spawnattr_destroy(self.0);
            }
        }
    }
    let _attr = Attr(&mut attr);

    let mut flags = 0;

    if let Some(pg) = opts.pgroup.leader {
        flags |= POSIX_SPAWN_SETPGROUP;
        if let Err(e) = posix_ret(
            unsafe { posix_spawnattr_setpgroup(&mut attr, pg) },
            "setpgroup",
        ) {
            pipes.close_all();
            return Err(e);
        }
    }

    flags |= POSIX_SPAWN_SETSIGMASK | POSIX_SPAWN_SETSIGDEF;

    if let Err(e) = posix_ret(
        unsafe { posix_spawnattr_setflags(&mut attr, flags as _) },
        "setflags",
    ) {
        pipes.close_all();
        return Err(e);
    }

    let empty_mask = SignalRuntime::empty_set();
    let def = SignalRuntime::set_with(&[libc::SIGPIPE]);

    if let Err(e) = posix_ret(
        unsafe { posix_spawnattr_setsigmask(&mut attr, &empty_mask) },
        "setsigmask",
    ) {
        pipes.close_all();
        return Err(e);
    }
    if let Err(e) = posix_ret(
        unsafe { posix_spawnattr_setsigdefault(&mut attr, &def) },
        "setsigdefault",
    ) {
        pipes.close_all();
        return Err(e);
    }

    let mut pid: pid_t = 0;

    let envp_ptr = envp.as_ref().map_or_else(
        || unsafe { environ as *const *mut c_char },
        |e: &arrayvec::ArrayVec<*mut c_char, 64>| e.as_ptr() as *const *mut c_char,
    );

    if let Err(e) = posix_ret(
        unsafe { posix_spawn(&mut pid, exe_ptr, &actions, &attr, argv.as_ptr(), envp_ptr) },
        "posix_spawn",
    ) {
        pipes.close_all();
        return Err(e);
    }

    drop(pipes.stdin_r.take());
    drop(pipes.stdout_w.take());
    drop(pipes.stderr_w.take());

    let drain = crate::low_level::io::DrainState::new(
        job_id,
        pipes
            .stdin_w
            .take()
            .and_then(|fd| if opts.stdin.is_some() { Some(fd) } else { None }),
        opts.stdin,
        pipes.stdout_r.take(),
        pipes.stderr_r.take(),
        opts.max_output,
        opts.early_exit,
    )?;

    Ok((pid, drain))
}

fn spawn_fork_internal(
    job_id: u64,
    opts: SpawnOptions,
) -> Result<(pid_t, DrainState<fn(&[u8]) -> bool>), SysError> {
    let mut pipes = Pipes::new(
        opts.stdin.as_deref(),
        opts.capture_stdout,
        opts.capture_stderr,
    )?;

    let exe_ptr = match &opts.ctx.argv {
        crate::low_level::sys::ExecArgv::Dynamic(v) => v[0].as_ptr(),
    };

    let argv = opts.ctx.get_argv_ptrs();
    let envp = opts.ctx.get_envp_ptrs();
    let cwd_cstr = &opts.ctx.cwd;

    let pid = unsafe { libc::fork() };

    if pid < 0 {
        pipes.close_all();
        syscall_ret(-1, "fork")?;
    }

    if pid == 0 {
        // Child

        // dup stdin
        if let (Some(r), Some(_)) = (&pipes.stdin_r, &pipes.stdin_w)
            && r.raw() != 0
        {
            // SAFETY: r.raw() is a valid fd. Target 0 is valid.
            unsafe {
                libc::dup2(r.raw(), 0);
            }
        }

        // dup stdout
        if let (Some(_), Some(w)) = (&pipes.stdout_r, &pipes.stdout_w)
            && w.raw() != 1
        {
            // SAFETY: w.raw() is a valid fd. Target 1 is valid.
            unsafe {
                libc::dup2(w.raw(), 1);
            }
        }

        // dup stderr
        if let (Some(_), Some(w)) = (&pipes.stderr_r, &pipes.stderr_w)
            && w.raw() != 2
        {
            // SAFETY: w.raw() is a valid fd. Target 2 is valid.
            unsafe {
                libc::dup2(w.raw(), 2);
            }
        }

        // SAFETY: Closes all unused file descriptors.
        unsafe {
            close_range_fast(None);
        }

        // setsid
        if opts.pgroup.isolated {
            // SAFETY: safe to call setsid in child.
            unsafe {
                libc::setsid();
            }
        }

        // chdir
        if let Some(cwd) = cwd_cstr {
            // SAFETY: cwd is a valid null-terminated CString.
            unsafe {
                if libc::chdir(cwd.as_ptr()) != 0 {
                    libc::_exit(127);
                }
            }
        }

        // setpgid
        if let Some(pg) = opts.pgroup.leader {
            // SAFETY: valid pgroup.
            unsafe {
                libc::setpgid(0, pg);
            }
        }

        let envp_ptr = envp.as_ref().map_or_else(
            || unsafe { environ as *const *mut c_char },
            |e: &arrayvec::ArrayVec<*mut c_char, 64>| e.as_ptr() as *const *mut c_char,
        );

        // unblock signals and reset SIGPIPE
        // SAFETY: valid signal mask array manipulation
        let _ = SignalRuntime::unblock_all();
        SignalRuntime::reset_default(libc::SIGPIPE);

        // exec
        // SAFETY: exe_ptr is null-terminated. argv and envp_ptr are valid null-terminated arrays.
        unsafe {
            libc::execve(
                exe_ptr,
                argv.as_ptr() as *const *const _,
                envp_ptr as *const *const _,
            );
            libc::_exit(127);
        }
    }

    // Parent
    drop(pipes.stdin_r.take());
    drop(pipes.stdout_w.take());
    drop(pipes.stderr_w.take());

    let drain = crate::low_level::io::DrainState::new(
        job_id,
        pipes
            .stdin_w
            .take()
            .and_then(|fd| if opts.stdin.is_some() { Some(fd) } else { None }),
        opts.stdin,
        pipes.stdout_r.take(),
        pipes.stderr_r.take(),
        opts.max_output,
        opts.early_exit,
    )?;

    Ok((pid, drain))
}

enum KillState {
    None,
    TermSent,
    KillSent,
}

fn wait_loop(
    pid: pid_t,
    mut drain: crate::low_level::io::DrainState<fn(&[u8]) -> bool>,
    mut reactor: Reactor,
    timeout_ms: Option<u32>,
    kill_grace_ms: u32,
    cancel: CancelPolicy,
    pgroup: ProcessGroup,
) -> Result<Output, SysError> {
    let process = Process::new(pid);
    let mut status_raw = process.wait_step()?;
    let mut state = KillState::None;
    let mut timed_out = false;

    let start_time = std::time::Instant::now();
    let deadline = timeout_ms.map(|t| std::time::Duration::from_millis(t as u64));

    loop {
        let mut poll_timeout = -1;

        if let Some(dl) = deadline {
            let elapsed = start_time.elapsed();
            if elapsed >= dl {
                timed_out = true;
                let elapsed_over = (elapsed - dl).as_millis();

                let target_is_group = pgroup.isolated || pgroup.leader.is_some();

                match state {
                    KillState::None => {
                        if cancel == CancelPolicy::Graceful {
                            let r = if target_is_group {
                                process.kill_pgroup(libc::SIGTERM)
                            } else {
                                process.kill(libc::SIGTERM)
                            };
                            if r.is_err() {
                                state = KillState::KillSent; // Process already gone
                            } else {
                                state = KillState::TermSent;
                            }
                        } else if cancel == CancelPolicy::Kill {
                            let _ = if target_is_group {
                                process.kill_pgroup(libc::SIGKILL)
                            } else {
                                process.kill(libc::SIGKILL)
                            };
                            state = KillState::KillSent;
                        } else {
                            // CancelPolicy::None just times out without killing
                        }
                    }
                    KillState::TermSent if elapsed_over > kill_grace_ms as u128 => {
                        let _ = if target_is_group {
                            process.kill_pgroup(libc::SIGKILL)
                        } else {
                            process.kill(libc::SIGKILL)
                        };
                        state = KillState::KillSent;
                    }
                    _ => {}
                }
                poll_timeout = 100; // Poll frequently while waiting for kill to take effect
            } else {
                let remaining = dl - elapsed;
                poll_timeout = remaining.as_millis().min(i32::MAX as u128) as i32;
            }
        }

        if status_raw.is_none()
            && let Some(s) = process.wait_step()?
        {
            status_raw = Some(s);
        }

        if drain.is_done() {
            let s = match status_raw {
                Some(s) => s,
                None => process.wait_blocking()?,
            };

            for slot in drain.take_all_slots() {
                reactor.del(&slot.fd);
            }
            let (stdout, stderr) = drain.into_parts();
            return Ok(Output {
                pid,
                status: Some(s),
                stdout,
                stderr,
                timed_out,
            });
        }

        let timeout = if status_raw.is_some() {
            if poll_timeout == -1 || poll_timeout > 1 {
                1
            } else {
                poll_timeout
            }
        } else {
            poll_timeout
        };

        let mut events = Vec::new();
        let nevents = reactor.wait(&mut events, 64, timeout)?;

        for ev in events.iter().take(nevents) {
            let fd_token = Some(ev.token);

            if ev.error {
                if drain
                    .stdout_slot
                    .as_ref()
                    .is_some_and(|s| s.token == fd_token)
                {
                    let slot = drain.stdout_slot.take();
                    reactor.del(&slot.unwrap().fd);
                } else if drain
                    .stderr_slot
                    .as_ref()
                    .is_some_and(|s| s.token == fd_token)
                {
                    let slot = drain.stderr_slot.take();
                    reactor.del(&slot.unwrap().fd);
                } else if drain
                    .stdin_slot
                    .as_ref()
                    .is_some_and(|s| s.token == fd_token)
                {
                    let slot = drain.stdin_slot.take();
                    reactor.del(&slot.unwrap().fd);
                    drain.writer.buf = None;
                }
                continue;
            }

            if drain
                .stdout_slot
                .as_ref()
                .is_some_and(|s| s.token == fd_token)
                && ev.readable
            {
                let _ = drain.read_fd(true)?;
            } else if drain
                .stderr_slot
                .as_ref()
                .is_some_and(|s| s.token == fd_token)
                && ev.readable
            {
                let _ = drain.read_fd(false)?;
            } else if drain
                .stdin_slot
                .as_ref()
                .is_some_and(|s| s.token == fd_token)
                && ev.writable
            {
                let _ = drain.write_stdin()?;
            }
        }
    }
}

```

---

## `rust/src/low_level/sys/mod.rs`

```rs
use crate::low_level::spawn::{SysError, syscall_ret};
use libc::sigset_t;

pub struct SignalRuntime;

#[inline(always)]
pub fn get_clk_tck() -> u64 {
    unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 }
}

impl SignalRuntime {
    pub fn empty_set() -> sigset_t {
        let mut set: sigset_t = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&mut set) };
        set
    }

    pub fn set_with(signals: &[i32]) -> sigset_t {
        let mut set: sigset_t = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&mut set) };
        for &sig in signals {
            unsafe { libc::sigaddset(&mut set, sig) };
        }
        set
    }

    pub fn unblock_all() -> Result<(), SysError> {
        let empty_mask = Self::empty_set();
        let r = unsafe { libc::sigprocmask(libc::SIG_SETMASK, &empty_mask, std::ptr::null_mut()) };
        syscall_ret(r, "sigprocmask")
    }

    pub fn reset_default(sig: i32) {
        unsafe { libc::signal(sig, libc::SIG_DFL) };
    }
}
use libc::{c_char, pid_t};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::ptr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CancelPolicy {
    #[default]
    None,
    Graceful, // implies term then kill
    Kill,     // implies direct kill
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessGroup {
    pub leader: Option<pid_t>,
    pub isolated: bool, // Corresponds to setsid
}

impl ProcessGroup {
    pub fn new(leader: Option<pid_t>, isolated: bool) -> Self {
        Self { leader, isolated }
    }
}

use arrayvec::ArrayVec;

pub enum ExecArgv {
    Dynamic(Vec<CString>),
}

pub struct ExecContext {
    pub argv: ExecArgv,
    pub envp: Option<Vec<CString>>,
    pub cwd: Option<CString>,
}

impl ExecContext {
    pub fn new(argv: Vec<String>, env: Option<Vec<String>>, cwd: Option<String>) -> Self {
        let mut c_argv: Vec<CString> = argv
            .into_iter()
            .filter_map(|s| CString::new(s).ok())
            .collect();

        if c_argv.is_empty() {
            c_argv.push(CString::new("/bin/false").unwrap());
        }

        let c_envp = match env {
            Some(vars) => {
                let e_vars: Vec<CString> = vars
                    .into_iter()
                    .filter_map(|s| CString::new(s).ok())
                    .collect();
                Some(e_vars)
            }
            None => None,
        };

        let c_cwd = match cwd {
            Some(c) => CString::new(c).ok(),
            None => None,
        };

        Self {
            argv: ExecArgv::Dynamic(c_argv),
            envp: c_envp,
            cwd: c_cwd,
        }
    }

    pub fn get_argv_ptrs(&self) -> ArrayVec<*mut c_char, 64> {
        let mut ptrs = ArrayVec::new();
        match &self.argv {
            ExecArgv::Dynamic(v) => {
                for s in v {
                    if ptrs.try_push(s.as_ptr() as *mut c_char).is_err() {
                        break;
                    }
                }
            }
        }
        if ptrs.is_full() {
            ptrs.pop(); // Ensure room for null terminator
        }
        let _ = ptrs.try_push(ptr::null_mut());
        ptrs
    }

    pub fn get_envp_ptrs(&self) -> Option<ArrayVec<*mut c_char, 64>> {
        self.envp.as_ref().map(|envp| {
            let mut ptrs = ArrayVec::new();
            for s in envp {
                if ptrs.try_push(s.as_ptr() as *mut c_char).is_err() {
                    break;
                }
            }
            if ptrs.is_full() {
                ptrs.pop(); // Ensure room for null terminator
            }
            let _ = ptrs.try_push(ptr::null_mut());
            ptrs
        })
    }
}

```

---

## `rust/src/low_level.rs`

```rs
pub mod io;
pub mod reactor;
pub mod spawn;
pub mod sys;

```

---

## `rust/src/main.rs`

```rs
use CoreShift::DaemonConfig;

fn main() {
    let mut args = std::env::args().skip(1);

    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "preload" => {
                let _ = CoreShift::run_daemon(DaemonConfig {
                    enable_warmup: true,
                    record_path: None,
                });
            }
            "replay" => {
                if let Some(path) = args.next() {
                    CoreShift::run_replay(&path);
                } else {
                    eprintln!("Usage: replay <file>");
                }
            }
            "record" => {
                if let Some(path) = args.next() {
                    let _ = CoreShift::run_daemon(DaemonConfig {
                        enable_warmup: false,
                        record_path: Some(path),
                    });
                } else {
                    eprintln!("Usage: record <file>");
                }
            }
            _ => {
                let _ = CoreShift::run_daemon(DaemonConfig {
                    enable_warmup: false,
                    record_path: None,
                });
            }
        }
    } else {
        let _ = CoreShift::run_daemon(DaemonConfig {
            enable_warmup: false,
            record_path: None,
        });
    }
}

```

---

## `rust/src/mid_level/ipc.rs`

```rs
use crate::core::ExecOutcome;
use crate::low_level::reactor::Fd;
use crate::low_level::reactor::{Event, Token};
use crate::low_level::spawn::SysError;
use libc::{
    SO_PEERCRED, SOCK_CLOEXEC, SOCK_NONBLOCK, SOL_SOCKET, accept4, c_void, socklen_t, ucred,
};
use std::collections::HashMap;
use std::os::unix::io::{AsRawFd, RawFd};

pub enum ReadState {
    Header { needed: usize },
    Body { len: usize },
}

pub struct Conn {
    pub fd: Fd,
    pub token: Token,
    pub read_buf: Vec<u8>,
    pub write_buf: Vec<u8>,
    pub state: ReadState,
    pub uid: u32,
}

pub struct WireMsg {
    pub client_id: u32,
    pub command: Command,
    pub uid: u32,
}

pub struct IpcModule {
    pub fd: Fd,
    pub server_token: Option<Token>,

    pub clients: HashMap<u32, Conn>,
    pub client_tokens: HashMap<Token, u32>,
    next_client_id: u32,
}

impl IpcModule {
    pub fn new(fd: Fd,  token: Token) -> Self {
        Self {
            fd,
            server_token: Some(token),

            clients: HashMap::new(),
            client_tokens: HashMap::new(),
            next_client_id: 1,
        }
    }

    /// Verifies the credentials of a peer on a connected Unix domain socket.
    /// Returns `Ok(uid)` if successful, or a `SysError` if validation fails.
    pub fn verify_peer_credentials(&self, peer_fd: RawFd) -> Result<u32, SysError> {
        let mut cred: ucred = unsafe { std::mem::zeroed() };
        let mut len: socklen_t = std::mem::size_of::<ucred>() as socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                peer_fd,
                SOL_SOCKET,
                SO_PEERCRED,
                &mut cred as *mut ucred as *mut c_void,
                &mut len as *mut socklen_t,
            )
        };

        if ret != 0 {
            return Err(SysError::sys(
                std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
                "getsockopt(SO_PEERCRED)",
            ));
        }

        Ok(cred.uid)
    }

    pub fn accept_clients(&mut self, reactor: &mut crate::low_level::reactor::Reactor) {
        loop {
            let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            let mut addr_len: socklen_t = std::mem::size_of::<libc::sockaddr_un>() as socklen_t;

            let client_fd = unsafe {
                accept4(
                    self.fd.as_raw_fd(),
                    &mut addr as *mut libc::sockaddr_un as *mut libc::sockaddr,
                    &mut addr_len as *mut socklen_t,
                    SOCK_NONBLOCK | SOCK_CLOEXEC,
                )
            };

            if client_fd < 0 {
                let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                    return; // Non-blocking, no more clients for now
                }
                return;
            }

            if let Ok(client_fd_obj) = Fd::new(client_fd, "accept4") {
                // Extract UID
                let uid = match self.verify_peer_credentials(client_fd) {
                    Ok(u) => u,
                    Err(_) => continue, // Drop on verification failure
                };

                // Register with reactor
                let token = match reactor.add(&client_fd_obj, true, true) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let client_id = self.next_client_id;
                self.next_client_id = self.next_client_id.wrapping_add(1);
                if self.next_client_id == 0 {
                    self.next_client_id = 1;
                }

                let conn = Conn {
                    fd: client_fd_obj,
                    token,
                    read_buf: Vec::new(),
                    write_buf: Vec::new(),
                    state: ReadState::Header { needed: 4 },
                    uid,
                };

                self.clients.insert(client_id, conn);
                self.client_tokens.insert(token, client_id);
            }
        }
    }

    pub fn handle_event(&mut self, reactor: &mut crate::low_level::reactor::Reactor, event: &Event) -> Vec<WireMsg> {
        if Some(event.token) == self.server_token && event.readable {
            self.accept_clients(reactor);
            return Vec::new();
        }

        let mut actions = Vec::new();
        let client_id = match self.client_tokens.get(&event.token) {
            Some(&id) => id,
            None => return actions,
        };

        if event.error {
            self.disconnect(client_id, reactor);
            return actions;
        }

        let mut should_disconnect = false;

        if event.readable {
            if let Some(conn) = self.clients.get_mut(&client_id) {
                let mut buf = [0u8; 4096];
                loop {
                    match conn.fd.read(buf.as_mut_ptr(), buf.len()) {
                        Ok(0) => {
                            should_disconnect = true;
                            break;
                        }
                        Ok(n) => {
                            conn.read_buf.extend_from_slice(&buf[..n]);
                            if conn.read_buf.len() > 16 * 1024 * 1024 {
                                should_disconnect = true;
                                break;
                            }
                        }
                        Err(e) => {
                            let raw_err = e.raw_os_error();
                            if raw_err == Some(libc::EAGAIN) || raw_err == Some(libc::EWOULDBLOCK) {
                                break;
                            } else {
                                should_disconnect = true;
                                break;
                            }
                        }
                    }
                }

                if !should_disconnect {
                    // Process read_buf
                    loop {
                        match conn.state {
                            ReadState::Header { needed } => {
                                if conn.read_buf.len() >= needed {
                                    let mut len_buf = [0u8; 4];
                                    len_buf.copy_from_slice(&conn.read_buf[..4]);
                                    let body_len = u32::from_le_bytes(len_buf) as usize;

                                    if body_len > 10 * 1024 * 1024 {
                                        should_disconnect = true;
                                        break;
                                    }

                                    conn.read_buf.drain(..4);
                                    conn.state = ReadState::Body { len: body_len };
                                } else {
                                    break;
                                }
                            }
                            ReadState::Body { len } => {
                                if conn.read_buf.len() >= len {
                                    let payload = conn.read_buf.drain(..len).collect::<Vec<_>>();
                                    conn.state = ReadState::Header { needed: 4 };

                                    // Decode payload
                                    if !payload.is_empty() {
                                        let req_type = payload[0];
                                        let req = match req_type {
                                            1 => {
                                                serde_json::from_slice::<Command>(&payload[1..]).ok()
                                            }
                                            2 => {
                                                if payload.len() == 9 {
                                                    let mut id_buf = [0u8; 8];
                                                    id_buf.copy_from_slice(&payload[1..9]);
                                                    let id = u64::from_le_bytes(id_buf);
                                                    Some(Command::GetResult { id })
                                                } else {
                                                    None
                                                }
                                            }
                                            3 => {
                                                if payload.len() == 9 {
                                                    let mut id_buf = [0u8; 8];
                                                    id_buf.copy_from_slice(&payload[1..9]);
                                                    let id = u64::from_le_bytes(id_buf);
                                                    Some(Command::Cancel { id })
                                                } else {
                                                    None
                                                }
                                            }
                                            _ => None,
                                        };

                                        if let Some(cmd) = req {


                                            actions.push(WireMsg { client_id, command: cmd, uid: conn.uid });
                                        } else {
                                            should_disconnect = true;
                                            break;
                                        }
                                    } else {
                                        should_disconnect = true;
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if event.writable && !should_disconnect {
            if let Some(conn) = self.clients.get_mut(&client_id) {
                if !conn.write_buf.is_empty() {
                    let mut total_written = 0;
                    while total_written < conn.write_buf.len() {
                        match conn.fd.write(conn.write_buf[total_written..].as_ptr(), conn.write_buf.len() - total_written) {
                            Ok(0) => {
                                should_disconnect = true;
                                break;
                            }
                            Ok(n) => {
                                total_written += n;
                            }
                            Err(e) => {
                                let raw_err = e.raw_os_error();
                                if raw_err == Some(libc::EAGAIN) || raw_err == Some(libc::EWOULDBLOCK) {
                                    break;
                                } else {
                                    should_disconnect = true;
                                    break;
                                }
                            }
                        }
                    }
                    if total_written > 0 {
                        conn.write_buf.drain(..total_written);
                    }
                }
            }
        }

        if should_disconnect {
            self.disconnect(client_id, reactor);
        }

        actions
    }

    pub fn disconnect(&mut self, client_id: u32, reactor: &mut crate::low_level::reactor::Reactor) {
        if let Some(conn) = self.clients.remove(&client_id) {
            reactor.del(&conn.fd);
            self.client_tokens.remove(&conn.token);
        }

    }

    pub fn intercept_action(&mut self, action: &crate::core::Action, reply_to: Option<u32>) {
        let client_id = match reply_to {
            Some(id) => id,
            None => return,
        };
        match action {
            crate::core::Action::Started { id } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Exec(*id));
                }
            }
            crate::core::Action::Controlled { id: _ } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::CancelOk);
                }
            }
            crate::core::Action::QueryResult { id: _, result } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Result(result.clone()));
                }
            }
            crate::core::Action::Rejected {
                ..
            } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    Self::queue_response(conn, WireResponse::Error);
                }
            }
            crate::core::Action::Finished {
                id,
                result,
                ..
            } => {
                if let Some(conn) = self.clients.get_mut(&client_id) {
                    let outcome = crate::core::ExecOutcome {
                        id: *id,
                        result: result.clone(),
                    };
                    Self::queue_response(conn, WireResponse::Result(Some(outcome)));
                }
            }
            _ => {}
        }
    }

    fn queue_response(conn: &mut Conn, resp: WireResponse) {
        let payload = match resp {
            WireResponse::Exec(id) => {
                let mut p = vec![1u8];
                p.extend_from_slice(&id.to_le_bytes());
                p
            }
            WireResponse::Result(res) => {
                let mut p = vec![2u8];
                let json = serde_json::to_vec(&res).unwrap_or_default();
                p.extend_from_slice(&json);
                p
            }
            WireResponse::CancelOk => {
                vec![3u8]
            }
            WireResponse::Error => {
                vec![4u8]
            }
        };
        let len = payload.len() as u32;
        conn.write_buf.extend_from_slice(&len.to_le_bytes());
        conn.write_buf.extend_from_slice(&payload);
    }
}

use crate::high_level::api::Command;


enum WireResponse {
    Exec(u64),
    Result(Option<ExecOutcome>),
    CancelOk,
    Error,
}

```

---

## `rust/src/mid_level/mod.rs`

```rs
pub mod ipc;

```

---

## `rust/src/runtime.rs`

```rs
use crate::arena::Arena;
use crate::core::{ControlSignal, Effect, Event, IoHandle, LogEvent, LogLevel, SystemService};
use crate::low_level::io::DrainState;
use crate::low_level::reactor::{Event as ReactorEvent, Reactor};
use crate::low_level::spawn::{Process, SpawnBackend, SpawnOptions, SysError, spawn_start};
use crate::low_level::sys::{ExecContext, ProcessGroup};
use std::collections::HashMap;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn readahead(fd: libc::c_int, offset: libc::off64_t, count: libc::size_t) -> libc::ssize_t;
}

#[inline(always)]
unsafe fn do_readahead(fd: libc::c_int) {
    if fd < 0 {
        return;
    }

    #[cfg(target_os = "android")]
    unsafe {
        libc::syscall(libc::SYS_readahead, fd, 0, 0);
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    unsafe {
        readahead(fd, 0, 0);
    }
}

pub struct FileSink {
    file: std::fs::File,
}

impl FileSink {
    pub fn new(path: &str) -> Self {
        use std::fs::OpenOptions;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap());
        Self { file }
    }

    pub fn write(&mut self, level: LogLevel, msg: String) {
        use std::io::Write;
        let _ = writeln!(self.file, "[{:?}] {}", level, msg);
    }
}

pub struct LogRouter {
    sinks: HashMap<u32, FileSink>,
    default: FileSink,
}

impl LogRouter {
    pub fn new(base_dir: &str) -> Self {
        let default_path = format!("{}/core.log", base_dir);
        let _ = std::fs::create_dir_all(base_dir);
        Self {
            sinks: HashMap::new(),
            default: FileSink::new(&default_path),
        }
    }

    fn get_or_create(&mut self, owner: u32) -> &mut FileSink {
        self.sinks.entry(owner).or_insert_with(|| {
            let dir_path = format!("/data/local/tmp/coreshift/addon_{}", owner);
            let _ = std::fs::create_dir_all(&dir_path);
            let file_path = format!("{}/log.txt", dir_path);
            FileSink::new(&file_path)
        })
    }

    pub fn write(&mut self, owner: u32, level: LogLevel, msg: String) {
        if owner == crate::core::CORE_OWNER {
            self.default.write(level, msg);
        } else {
            self.get_or_create(owner).write(level, msg);
        }
    }
}

pub struct RuntimeProcess {
    pub process: Process,
    pub is_group: bool,
}

pub struct RuntimeDrain {
    pub drain: DrainState<fn(&[u8]) -> bool>,
}

pub struct EffectExecutor {
    pub reactor: Reactor,
    pub fd_map: Vec<Option<(IoHandle, crate::core::IoStream)>>,
    pub processes: Arena<RuntimeProcess>,
    pub drains: Arena<RuntimeDrain>,
    pub log_router: LogRouter,
}

impl EffectExecutor {
    pub fn new(reactor: Reactor, base_dir: &str) -> Self {
        Self {
            reactor,
            fd_map: Vec::new(),
            processes: Arena::new(),
            drains: Arena::new(),
            log_router: LogRouter::new(base_dir),
        }
    }

    pub fn process_reactor_events(
        &mut self,
        events: &mut Vec<ReactorEvent>,
        timeout_ms: i32,
    ) -> Result<Vec<Event>, SysError> {
        let nevents = self.reactor.wait(events, 64, timeout_ms)?;
        let mut sys_events = Vec::new();

        for ev in events.iter().take(nevents) {
            let idx = ev.token.0 as usize;
            if idx < self.fd_map.len() {
                if let Some((io, stream)) = self.fd_map[idx] {
                    sys_events.push(Event::IoReady {
                        io,
                        stream,
                        readable: ev.readable,
                        writable: ev.writable,
                        error: ev.error,
                    });
                }
            }
        }
        Ok(sys_events)
    }

    pub fn apply(&mut self, effect: Effect) -> Vec<Event> {
        match effect {
            Effect::Log {
                owner,
                level,
                event,
            } => {
                let msg = match event {
                    LogEvent::Submit { id } => format!("Submit id={}", id),
                    LogEvent::Spawn { id, pid } => format!("Spawn id={}, pid={}", id, pid),
                    LogEvent::Cancel { id } => format!("Cancel id={}", id),
                    LogEvent::ForceKill { id } => format!("ForceKill id={}", id),
                    LogEvent::Exit { id, status } => format!("Exit id={}, status={:?}", id, status),
                    LogEvent::Timeout { id } => format!("Timeout id={}", id),
                    LogEvent::Error { id, err } => format!("Error id={}, err={}", id, err),
                    LogEvent::TickStart => "tick_start".to_string(),
                    LogEvent::TickEnd => "tick_end".to_string(),
                    LogEvent::AddonReceived => "addon_received".to_string(),
                    LogEvent::AddonTranslated => "addon_translated".to_string(),
                    LogEvent::AddonDropped => "addon_dropped".to_string(),
                    LogEvent::ActionDispatched => "action_dispatched".to_string(),
                    LogEvent::Observability { queue_len, actions_processed, dropped } => format!("queue_len={} processed={} dropped={}", queue_len, actions_processed, dropped),
                };
                self.log_router.write(owner, level, msg);
                vec![]
            }
            Effect::WatchStream { io, stream } => {
                let fd_ref = if let Some(rdrain) = self.drains.get(io.index, io.generation) {
                    match stream {
                        crate::core::IoStream::Stdout => {
                            rdrain.drain.stdout_slot.as_ref().map(|s| &s.fd)
                        }
                        crate::core::IoStream::Stderr => {
                            rdrain.drain.stderr_slot.as_ref().map(|s| &s.fd)
                        }
                        crate::core::IoStream::Stdin => {
                            rdrain.drain.stdin_slot.as_ref().map(|s| &s.fd)
                        }
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "drain not found".to_string(),
                    }];
                };

                if let Some(fd) = fd_ref {
                    match self.reactor.add(fd, true, true) {
                        Ok(token) => {
                            let idx = token.0 as usize;
                            if idx >= self.fd_map.len() {
                                self.fd_map.resize(idx + 1, None);
                            }
                            self.fd_map[idx] = Some((io, stream));
                        }
                        Err(e) => {
                            return vec![Event::WatchStreamFailed {
                                io,
                                err: format!("reactor add failed: {}", e),
                            }];
                        }
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "stream fd not available".to_string(),
                    }];
                }
                vec![]
            }
            Effect::UnwatchStream { io, stream } => {
                if let Some(rdrain) = self.drains.get(io.index, io.generation) {
                    let raw_fd = match stream {
                        crate::core::IoStream::Stdout => {
                            rdrain.drain.stdout_slot.as_ref().map(|s| s.fd.raw())
                        }
                        crate::core::IoStream::Stderr => {
                            rdrain.drain.stderr_slot.as_ref().map(|s| s.fd.raw())
                        }
                        crate::core::IoStream::Stdin => {
                            rdrain.drain.stdin_slot.as_ref().map(|s| s.fd.raw())
                        }
                    };

                    if let Some(fd) = raw_fd {
                        self.reactor.del_raw(fd);
                        for slot in self.fd_map.iter_mut() {
                            if let Some((mapped_io, mapped_stream)) = slot {
                                if *mapped_io == io && *mapped_stream == stream {
                                    *slot = None;
                                }
                            }
                        }
                    } else {
                        return vec![Event::WatchStreamFailed {
                            io,
                            err: "unwatch: stream fd not available".to_string(),
                        }];
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "unwatch: drain not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::StartProcess { id, exec, policy } => {
                let ctx = ExecContext::new(exec.argv, None, None);
                let stdin_buf = exec.stdin.map(|v| v.into_boxed_slice());

                let is_group = false; 
                let pgroup = ProcessGroup::default();

                let opts = SpawnOptions {
                    ctx,
                    stdin: stdin_buf,
                    capture_stdout: exec.capture_stdout,
                    capture_stderr: exec.capture_stderr,
                    wait: false,
                    pgroup,
                    max_output: exec.max_output,
                    timeout_ms: None,
                    kill_grace_ms: policy.kill_grace_ms,
                    cancel: match policy.cancel {
                        crate::core::CancelPolicy::None => {
                            crate::low_level::sys::CancelPolicy::None
                        }
                        crate::core::CancelPolicy::Graceful => {
                            crate::low_level::sys::CancelPolicy::Graceful
                        }
                        crate::core::CancelPolicy::Kill => {
                            crate::low_level::sys::CancelPolicy::Kill
                        }
                    },
                    backend: SpawnBackend::Auto,
                    early_exit: None,
                };

                match spawn_start(id, opts) {
                    Ok(running) => {
                        let (p_idx, p_gen) = self.processes.insert(RuntimeProcess {
                            process: running.process,
                            is_group,
                        });
                        let proc_h = crate::core::Handle {
                            index: p_idx,
                            generation: p_gen,
                            _marker: std::marker::PhantomData,
                        };

                        let (d_idx, d_gen) = self.drains.insert(RuntimeDrain {
                            drain: running.drain,
                        });
                        let io_h = crate::core::Handle {
                            index: d_idx,
                            generation: d_gen,
                            _marker: std::marker::PhantomData,
                        };

                        vec![Event::ProcessStarted {
                            id,
                            process: proc_h,
                            io: io_h,
                        }]
                    }
                    Err(e) => {
                        vec![Event::ProcessSpawnFailed {
                            id,
                            err: format!("spawn_failed: {}", e),
                        }]
                    }
                }
            }
            Effect::KillProcess { process, signal } => {
                if let Some(rproc) = self.processes.get_mut(process.index, process.generation) {
                    let res = match signal {
                        ControlSignal::GracefulStop => {
                            if rproc.is_group {
                                rproc.process.kill_pgroup(libc::SIGTERM)
                            } else {
                                rproc.process.kill(libc::SIGTERM)
                            }
                        }
                        ControlSignal::ForceKill => {
                            if rproc.is_group {
                                rproc.process.kill_pgroup(libc::SIGKILL)
                            } else {
                                rproc.process.kill(libc::SIGKILL)
                            }
                        }
                    };
                    if let Err(e) = res {
                        return vec![Event::KillProcessFailed {
                            process,
                            err: format!("kill failed: {}", e),
                        }];
                    }
                } else {
                    return vec![Event::KillProcessFailed {
                        process,
                        err: "process not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::PollProcess { process } => {
                if let Some(rproc) = self.processes.get_mut(process.index, process.generation) {
                    let status_res = rproc.process.wait_step();
                    match status_res {
                        Ok(Some(status)) => {
                            let s = match status {
                                crate::low_level::spawn::ExitStatus::Exited(c) => c,
                                crate::low_level::spawn::ExitStatus::Signaled(sig) => -sig,
                            };
                            vec![Event::ProcessExited {
                                process,
                                status: Some(s),
                            }]
                        }
                        Ok(None) => vec![],
                        Err(e) => {
                            vec![Event::KillProcessFailed {
                                process,
                                err: format!("poll wait_step failed: {}", e),
                            }]
                        }
                    }
                } else {
                    vec![Event::KillProcessFailed {
                        process,
                        err: "process not found for polling".to_string(),
                    }]
                }
            }
            Effect::PerformIo { io } => {
                if let Some(rdrain) = self.drains.get_mut(io.index, io.generation) {
                    let mut closed = false;
                    let mut err_reason = None;
                    if rdrain.drain.stdout_slot.is_some() {
                        match rdrain.drain.read_fd(true) {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stdout read: {}", e));
                            }
                        }
                    }
                    if rdrain.drain.stderr_slot.is_some() {
                        match rdrain.drain.read_fd(false) {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stderr read: {}", e));
                            }
                        }
                    }
                    if rdrain.drain.stdin_slot.is_some() {
                        match rdrain.drain.write_stdin() {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stdin write: {}", e));
                            }
                        }
                    }

                    if let Some(reason) = err_reason {
                        return vec![Event::IoFailed { io, reason }];
                    } else if closed {
                        return vec![Event::IoClosed { io }];
                    }
                } else {
                    return vec![Event::IoFailed {
                        io,
                        reason: "drain not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::AddonTask { addon_id, key, payload } => {
                if payload.is_empty() {
                    return vec![Event::AddonFailed { addon_id, key, err: "empty payload".to_string() }];
                }

                match payload[0] {
                    1 => { // Warmup (Preload Addon Specific)
                        let start = std::time::Instant::now();
                        let mut bytes = 0;
                        let mut failure_reason = None;

                        if let Ok(paths) = serde_json::from_slice::<Vec<String>>(&payload[1..]) {
                            for path in paths {
                                match std::ffi::CString::new(path.clone()) {
                                    Ok(c_path) => unsafe {
                                        let fd = libc::open(c_path.as_ptr(), libc::O_RDONLY);
                                        if fd >= 0 {
                                            let mut st: libc::stat = std::mem::zeroed();
                                            if libc::fstat(fd, &mut st) == 0 {
                                                bytes += st.st_size as u64;
                                            }
                                            do_readahead(fd);
                                            libc::close(fd);
                                        } else {
                                            failure_reason = Some(format!("open failed for {}", path));
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        failure_reason = Some(format!("invalid CString path: {}", e));
                                        break;
                                    }
                                }
                            }
                        } else {
                             failure_reason = Some("failed to decode paths".to_string());
                        }

                        if let Some(err) = failure_reason {
                            vec![Event::AddonFailed { addon_id, key, err }]
                        } else {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let res_payload = serde_json::to_vec(&(bytes, duration_ms)).unwrap_or_default();
                            vec![Event::AddonCompleted { addon_id, key, payload: res_payload }]
                        }
                    }
                    _ => vec![Event::AddonFailed { addon_id, key, err: "unknown task type".to_string() }]
                }
            }
            Effect::SystemRequest { request_id, kind, payload } => {
                match kind {
                    SystemService::ResolveIdentity => {
                         if let Ok(pid) = serde_json::from_slice::<i32>(&payload) {
                            let cmdline_path = format!("/proc/{}/cmdline", pid);
                            match std::fs::read(&cmdline_path) {
                                Ok(cmdline) => {
                                    if let Some(null_pos) = cmdline.iter().position(|&c| c == 0) {
                                        let package_name = &cmdline[..null_pos];
                                        vec![Event::SystemResponse { request_id, kind, payload: package_name.to_vec() }]
                                    } else {
                                        vec![Event::SystemFailure { request_id, kind, err: "no null terminator".to_string() }]
                                    }
                                }
                                Err(e) => vec![Event::SystemFailure { request_id, kind, err: format!("read failed: {}", e) }]
                            }
                         } else {
                             vec![Event::SystemFailure { request_id, kind, err: "invalid pid payload".to_string() }]
                         }
                    }
                    SystemService::ResolveDirectory => {
                        if let Ok(package_name) = String::from_utf8(payload) {
                             match std::fs::read_dir("/data/app") {
                                Ok(data_app) => {
                                    let mut found = false;
                                    let mut res = vec![];
                                    for outer_entry in data_app.flatten() {
                                        let outer_path = outer_entry.path();
                                        if outer_path.is_dir() && let Ok(inner_dir) = std::fs::read_dir(&outer_path) {
                                            for inner_entry in inner_dir.flatten() {
                                                let inner_name = inner_entry.file_name();
                                                if inner_name.to_string_lossy().starts_with(&package_name) {
                                                    let base_dir = inner_entry.path().to_string_lossy().into_owned();
                                                    let resp_payload = serde_json::to_vec(&(package_name.clone(), base_dir)).unwrap_or_default();
                                                    res = vec![Event::SystemResponse { request_id, kind, payload: resp_payload }];
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        if found { break; }
                                    }
                                    if !found {
                                        vec![Event::SystemFailure { request_id, kind, err: "package dir not found".to_string() }]
                                    } else {
                                        res
                                    }
                                }
                                Err(e) => vec![Event::SystemFailure { request_id, kind, err: format!("read_dir /data/app failed: {}", e) }]
                            }
                        } else {
                            vec![Event::SystemFailure { request_id, kind, err: "invalid package name payload".to_string() }]
                        }
                    }
                    SystemService::DiscoverPaths => {
                        if let Ok((package_name, base_dir)) = serde_json::from_slice::<(String, String)>(&payload) {
                            let mut paths = Vec::new();
                            let base_path = std::path::PathBuf::from(&base_dir);

                            let lib_dir = base_path.join("lib/arm64");
                            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                                for entry in entries.flatten() {
                                    if let Some(ext) = entry.path().extension() && ext == "so" {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            let oat_dir = base_path.join("oat/arm64");
                            if let Ok(entries) = std::fs::read_dir(&oat_dir) {
                                for entry in entries.flatten() {
                                    if let Some(ext) = entry.path().extension() && (ext == "odex" || ext == "vdex" || ext == "art") {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            paths.push(base_path.join("base.apk").to_string_lossy().into_owned());

                            if let Ok(entries) = std::fs::read_dir(&base_path) {
                                for entry in entries.flatten() {
                                    let name = entry.file_name();
                                    let name_str = name.to_string_lossy();
                                    if name_str.starts_with("split_") && name_str.ends_with(".apk") {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            if !paths.is_empty() {
                                paths.sort_unstable();
                                paths.truncate(64);
                                let resp_payload = serde_json::to_vec(&(package_name, paths)).unwrap_or_default();
                                vec![Event::SystemResponse { request_id, kind, payload: resp_payload }]
                            } else {
                                vec![Event::SystemFailure { request_id, kind, err: "no paths discovered".to_string() }]
                            }
                        } else {
                            vec![Event::SystemFailure { request_id, kind, err: "invalid discovery payload".to_string() }]
                        }
                    }
                }
            }
        }
    }
}

```

---

## `scripts/build-rust-android.sh`

```sh
#!/bin/bash
set -e

# Resolve script and project directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_ROOT="$PROJECT_ROOT/rust"
JNI_LIBS_ROOT="$PROJECT_ROOT/app/src/main/jniLibs"

# Set deterministic target directory
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUST_ROOT/target}"
echo "Using CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Ensure Rust targets are installed
echo "Ensuring Rust targets are available..."
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build binaries
cd "$RUST_ROOT"

echo "Building for aarch64-linux-android..."
cargo build --release --target aarch64-linux-android -j 1

echo "Building for armv7-linux-androideabi..."
cargo build --release --target armv7-linux-androideabi -j 1

# Define expected binary paths
BINARY_ARM64="$CARGO_TARGET_DIR/aarch64-linux-android/release/CoreShift"
BINARY_ARMV7="$CARGO_TARGET_DIR/armv7-linux-androideabi/release/CoreShift"

# Validation
function check_binary() {
    if [ ! -f "$1" ]; then
        echo "ERROR: Binary not found at $1"
        echo "Found the following files in target release directories:"
        find "$CARGO_TARGET_DIR" -maxdepth 4 -type f | grep "release/" | head -n 20
        exit 1
    fi
}

check_binary "$BINARY_ARM64"
check_binary "$BINARY_ARMV7"

# Create jniLibs directories
echo "Preparing jniLibs packaging..."
mkdir -p "$JNI_LIBS_ROOT/arm64-v8a"
mkdir -p "$JNI_LIBS_ROOT/armeabi-v7a"

# Copy and rename binaries
# These are executable payloads packaged with .so extension to force Android PM extraction.
echo "Copying executable payloads to jniLibs..."
cp "$BINARY_ARM64" "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
cp "$BINARY_ARMV7" "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

# Ensure executability
chmod 755 "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
chmod 755 "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

# Print verification info
echo "Verifying packaged binaries:"
file "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
file "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

echo "Rust Android build and packaging successfully complete."

```

---

## `settings.gradle.kts`

```kts
pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "CorePolicyManager"
include(":app")
include(":core:model")
include(":core:common")
include(":core:data")
include(":core:database")
include(":core:network")
include(":core:designsystem")
include(":feature:home")
include(":feature:settings")

```
