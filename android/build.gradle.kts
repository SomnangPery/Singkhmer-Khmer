import java.util.Properties

plugins {
    id("com.android.application") version "9.2.1" apply false
}

// Optional: `build.dir=C:/some/local/folder` in local.properties moves all
// build output there. Use it when the project sits in a synced folder such as
// OneDrive, which slows builds down and can lock files in the middle of one.
val localProperties = Properties().apply {
    val file = rootProject.file("local.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
localProperties.getProperty("build.dir")?.let { buildDir ->
    allprojects {
        layout.buildDirectory.set(file("$buildDir/${project.name}"))
    }
}
