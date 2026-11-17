import java.util.Properties

plugins {
    id("com.android.application") version "9.2.1" apply false
}

val localProperties = Properties().apply {
    val file = rootProject.file("local.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
localProperties.getProperty("build.dir")?.let { buildDir ->
    allprojects {
        layout.buildDirectory.set(file("$buildDir/${project.name}"))
    }
}
