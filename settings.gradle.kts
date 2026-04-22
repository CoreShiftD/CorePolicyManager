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
include(":core:domain")
include(":core:data")
include(":core:designsystem")
include(":feature:overview")
include(":feature:daemon")
include(":feature:policies")
include(":feature:settings")
