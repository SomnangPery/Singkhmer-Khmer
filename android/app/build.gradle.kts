import java.io.File
import javax.inject.Inject
import org.gradle.process.ExecOperations

plugins {
    id("com.android.application")
}

android {
    namespace = "com.newkeyboard.khmer"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "com.newkeyboard.khmer"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
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
    // Runs Android view code on the computer, without a phone.
    testImplementation("org.robolectric:robolectric:4.16.1")
}

// The tests run on the computer, so they need the engine built for the
// computer rather than for Android.
val cargoBuildHost = tasks.register<Exec>("cargoBuildHost") {
    val exe = if (System.getProperty("os.name").lowercase().contains("windows")) ".exe" else ""
    val installedCargo = File(System.getProperty("user.home"), ".cargo/bin/cargo$exe")
    workingDir = rootProject.file("..")
    commandLine(if (installedCargo.exists()) installedCargo.absolutePath else "cargo", "build", "-p", "khmer-android")
    environment("CARGO_TARGET_DIR", layout.buildDirectory.dir("cargo-host").get().asFile.absolutePath)
}

tasks.withType<Test>().configureEach {
    dependsOn(cargoBuildHost)
    // Where TypingTest finds the engine.
    systemProperty("java.library.path", layout.buildDirectory.dir("cargo-host/debug").get().asFile.path)
    // KeyboardPreviewTest draws every layout into this folder as a picture.
    systemProperty("previewDir", layout.buildDirectory.dir("keyboard-previews").get().asFile.path)
}

/**
 * Builds the Rust engine (crates/khmer-android) into one libkhmer_android.so
 * per Android processor type, so that building the app in Android Studio also
 * builds the engine. Needs Rust with the Android targets:
 *
 *   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
 */
abstract class CargoBuild @Inject constructor(private val execOperations: ExecOperations) : DefaultTask() {
    /** The Rust code and data: the build runs again only when these change. */
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    /** Android processor type -> Rust target. */
    @get:Input
    abstract val targets: MapProperty<String, String>

    /** The oldest Android version the library must run on (minSdk). */
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
        // The NDK's compiler does the linking. It is told the target on the
        // command line, which avoids the NDK's .cmd wrapper scripts.
        val clang = ndkDir.get().file("toolchains/llvm/prebuilt/$host/bin/clang$exe").asFile
        check(clang.exists()) { "The Android NDK compiler was not found at $clang" }
        // Android Studio does not always pass the user's PATH on.
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
            // Shared by all variants, so the engine is compiled once.
            cargoTargetDir.set(layout.buildDirectory.dir("cargo"))
        }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoBuild, CargoBuild::outputDir)
    }
}
