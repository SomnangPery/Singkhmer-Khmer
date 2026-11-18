import java.io.File
import javax.inject.Inject
import org.gradle.process.ExecOperations

plugins {
    id("com.android.application")
}

// The release workflow passes -PbunnyVersion=0.5.0. Its versionCode is 500.
val bunnyVersion = (findProperty("bunnyVersion") as String?) ?: "0.1.0"
val bunnyVersionCode = bunnyVersion.split('.').map { it.toInt() }
    .let { (it + listOf(0, 0, 0)).take(3) }
    .let { (major, minor, patch) -> major * 10000 + minor * 100 + patch }

android {
    namespace = "com.newkeyboard.khmer"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "com.newkeyboard.khmer"
        minSdk = 26
        targetSdk = 36
        versionCode = bunnyVersionCode
        versionName = bunnyVersion
    }

    // A key for test builds, kept in the repository so that every release is
    // signed the same way and installs over the one before. It is not secret:
    // before the app goes to the Play Store, sign it with a private key instead.
    signingConfigs {
        create("test") {
            storeFile = file("test-signing.keystore")
            storePassword = "bunny-test"
            keyAlias = "bunny-test"
            keyPassword = "bunny-test"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("test")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    testOptions {
        unitTests {
            isIncludeAndroidResources = true
        }
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.16.1")
}

val cargoBuildHost = tasks.register<Exec>("cargoBuildHost") {
    val exe = if (System.getProperty("os.name").lowercase().contains("windows")) ".exe" else ""
    val installedCargo = File(System.getProperty("user.home"), ".cargo/bin/cargo$exe")
    workingDir = rootProject.file("..")
    commandLine(if (installedCargo.exists()) installedCargo.absolutePath else "cargo", "build", "-p", "khmer-android")
    environment("CARGO_TARGET_DIR", layout.buildDirectory.dir("cargo-host").get().asFile.absolutePath)
}

tasks.withType<Test>().configureEach {
    dependsOn(cargoBuildHost)
    systemProperty("java.library.path", layout.buildDirectory.dir("cargo-host/debug").get().asFile.path)
    systemProperty("previewDir", layout.buildDirectory.dir("keyboard-previews").get().asFile.path)
}

abstract class CargoBuild @Inject constructor(private val execOperations: ExecOperations) : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Input
    abstract val targets: MapProperty<String, String>

    @get:Input
    abstract val apiLevel: Property<Int>

    @get:Internal
    abstract val repoDir: DirectoryProperty

    @get:Internal
    abstract val ndkDir: DirectoryProperty

    @get:Internal
    abstract val cargoTargetDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val os = System.getProperty("os.name").lowercase()
        val windows = os.contains("windows")
        val host = when {
            windows -> "windows-x86_64"
            os.contains("mac") -> "darwin-x86_64"
            else -> "linux-x86_64"
        }
        val exe = if (windows) ".exe" else ""
        val clang = ndkDir.get().file("toolchains/llvm/prebuilt/$host/bin/clang$exe").asFile
        check(clang.exists()) { "The Android NDK compiler was not found at $clang" }
        val installedCargo = File(System.getProperty("user.home"), ".cargo/bin/cargo$exe")
        val cargo = if (installedCargo.exists()) installedCargo.absolutePath else "cargo"

        val out = outputDir.get().asFile
        out.deleteRecursively()
        targets.get().forEach { (abi, triple) ->
            val clangTriple = if (triple.startsWith("armv7")) "armv7a-linux-androideabi" else triple
            val env = triple.uppercase().replace('-', '_')
            execOperations.exec {
                workingDir = repoDir.get().asFile
                commandLine(cargo, "build", "--release", "-p", "khmer-android", "--target", triple)
                environment("CARGO_TARGET_DIR", cargoTargetDir.get().asFile.absolutePath)
                environment("CARGO_TARGET_${env}_LINKER", clang.absolutePath)
                environment("CARGO_TARGET_${env}_RUSTFLAGS", "-Clink-arg=--target=$clangTriple${apiLevel.get()}")
            }
            val library = cargoTargetDir.get().file("$triple/release/libkhmer_android.so").asFile
            library.copyTo(File(out, "$abi/libkhmer_android.so"), overwrite = true)
        }
    }
}

androidComponents {
    onVariants { variant ->
        val name = variant.name.replaceFirstChar { it.uppercase() }
        val cargoBuild = tasks.register<CargoBuild>("cargoBuild$name") {
            val repo = rootProject.layout.projectDirectory.dir("..")
            repoDir.set(repo)
            sources.from(repo.file("Cargo.toml"), repo.file("Cargo.lock"), repo.dir("crates"))
            targets.set(
                mapOf(
                    "arm64-v8a" to "aarch64-linux-android",
                    "armeabi-v7a" to "armv7-linux-androideabi",
                    "x86_64" to "x86_64-linux-android",
                )
            )
            apiLevel.set(26)
            ndkDir.set(sdkComponents.ndkDirectory)
            cargoTargetDir.set(layout.buildDirectory.dir("cargo"))
        }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoBuild, CargoBuild::outputDir)
    }
}
