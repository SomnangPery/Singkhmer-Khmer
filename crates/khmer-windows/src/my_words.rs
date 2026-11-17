//! The user's own words, in `%APPDATA%\Bunny Keyboard`:
//!
//! * `my-words.txt`: words the user added, which come first in the
//!   suggestions. Written by the "my words" editor (`windows/my-words.ps1`,
//!   installed next to this DLL) or by hand in Notepad. Each app that types
//!   with the keyboard reads it again whenever it has changed, so a new word
//!   works everywhere at the next key press.
//! * `learned.txt`: words the user corrected by picking a suggestion other
//!   than the first. Only the word pairs (`srolanh = ស្រលាញ់`), never the text
//!   around them. They stay on the PC; the editor's Share tab lets the user
//!   send the ones they choose.

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

fn folder() -> Option<PathBuf> {
    let folder = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(folder).join("Bunny Keyboard"))
}

pub fn file() -> Option<PathBuf> {
    Some(folder()?.join("my-words.txt"))
}

/// Adds corrections to `learned.txt`. Failing to write loses only these.
pub fn remember(corrections: &[(String, String)]) {
    use std::io::Write;
    if corrections.is_empty() {
        return;
    }
    let Some(folder) = folder() else {
        return;
    };
    let _ = std::fs::create_dir_all(&folder);
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(folder.join("learned.txt")) else {
        return;
    };
    let lines: String = corrections.iter().map(|(spelling, word)| format!("{spelling} = {word}\n")).collect();
    let _ = file.write_all(lines.as_bytes());
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

/// What the editor opens for.
pub enum Request {
    /// "Add my word": to add `spelling`.
    Add { spelling: String },
    /// "Wrong word?": the user typed `typed` and the keyboard gave `got`;
    /// `spelling` is the word they most likely mean.
    Report { spelling: String, typed: String, got: String },
}

/// Keeps only what belongs in each value, so nothing else reaches the
/// command line: Latin letters and spaces, or Khmer.
fn letters(text: &str) -> String {
    text.chars().filter(|c| c.is_ascii_alphabetic() || *c == ' ').collect::<String>().trim().to_string()
}

fn khmer(text: &str) -> String {
    text.chars().filter(|c| ('\u{1780}'..='\u{17FF}').contains(c) || c.is_ascii_alphabetic() || *c == ' ').collect()
}

/// Opens the editor.
pub fn open_editor(request: &Request) -> bool {
    let Some(editor) = editor() else {
        return false;
    };
    let windows = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let powershell = windows.join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let mut command = Command::new(powershell);
    command
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File"])
        .arg(editor)
        .creation_flags(CREATE_NO_WINDOW);
    let values = match request {
        Request::Add { spelling } => vec![("-Spelling", letters(spelling))],
        Request::Report { spelling, typed, got } => {
            command.arg("-Report");
            vec![("-Spelling", letters(spelling)), ("-Typed", letters(typed)), ("-Got", khmer(got))]
        }
    };
    for (name, value) in values {
        if !value.is_empty() {
            command.arg(name).arg(value);
        }
    }
    command.spawn().is_ok()
}
