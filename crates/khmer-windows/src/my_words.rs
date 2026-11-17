//! The user's own words: `%APPDATA%\Khmer Keyboard\my-words.txt`.
//!
//! The file is written by the "my words" editor (`windows/my-words.ps1`,
//! installed next to this DLL) or by hand in Notepad. Each app that types
//! with the keyboard reads it again whenever it has changed, so a new word
//! works everywhere at the next key press.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

use crate::module_instance;

/// The editor, next to the DLL.
const EDITOR: &str = "my-words.ps1";

/// No console window for the editor's PowerShell.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn file() -> Option<PathBuf> {
    let folder = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(folder).join("Khmer Keyboard").join("my-words.txt"))
}

/// Reads the file again if it changed since it was last read, or was
/// deleted. Cheap when nothing changed: one look at the file's date.
pub fn refresh() {
    // What was read last: `None` before the first read, then the file's
    // date (`Some(None)` when there was no file).
    static READ: Mutex<Option<Option<SystemTime>>> = Mutex::new(None);
    let Some(path) = file() else {
        return;
    };
    let changed = std::fs::metadata(&path).and_then(|data| data.modified()).ok();
    let mut read = READ.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if *read == Some(changed) {
        return;
    }
    let text = if changed.is_some() { std::fs::read_to_string(&path).unwrap_or_default() } else { String::new() };
    khmer_core::set_user_words(&text);
    *read = Some(changed);
}

/// The installed editor, if there is one (there is none in a build that was
/// not installed, such as the tests).
pub fn editor() -> Option<PathBuf> {
    let mut path = [0u16; 1024];
    let length = unsafe { GetModuleFileNameW(module_instance(), &mut path) } as usize;
    if length == 0 || length >= path.len() {
        return None;
    }
    let dll = PathBuf::from(String::from_utf16_lossy(&path[..length]));
    let editor = dll.parent()?.join(EDITOR);
    editor.is_file().then_some(editor)
}

/// Opens the editor, ready to add `spelling`.
pub fn open_editor(spelling: &str) -> bool {
    let Some(editor) = editor() else {
        return false;
    };
    // Only letters reach the command line.
    let spelling: String = spelling.chars().filter(char::is_ascii_alphabetic).collect();
    let windows = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let powershell = windows.join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(editor)
        .creation_flags(CREATE_NO_WINDOW);
    if !spelling.is_empty() {
        command.args(["-Spelling", &spelling]);
    }
    command.spawn().is_ok()
}
