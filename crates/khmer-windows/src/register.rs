//! Telling Windows that the keyboard exists, and taking that back.
//!
//! Three things are written, all under "all users", which is why this needs
//! an administrator:
//!
//! 1. where the DLL for the text service is (the COM registration);
//! 2. the two keyboards it offers, under the Khmer language;
//! 3. what kind of text service it is.

use windows::core::{Error, Result, GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, MAX_PATH, TRUE, WIN32_ERROR};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    ITfCategoryMgr, ITfInputProcessorProfileMgr, CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIP_KEYBOARD,
};

use crate::{module_instance, CLSID_TEXT_SERVICE, LANGUAGE_KHMER, PROFILE_KHMER, PROFILE_SINGKHMER};

const SERVICE_NAME: &str = "Khmer Keyboard";

/// The keyboards, with the names the Windows keyboard list shows.
const PROFILES: [(GUID, &str); 2] =
    [(PROFILE_SINGKHMER, "Singkhmer (Khmer Keyboard)"), (PROFILE_KHMER, "Khmer NiDA (Khmer Keyboard)")];

/// What Windows should know about the service: it is a keyboard, it works in
/// Store apps and the Start menu too, it shows in the taskbar, and it says
/// how text being typed should look.
const CATEGORIES: [GUID; 4] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
];

/// `{1AF035EE-A180-...}`, the way the registry writes a GUID.
pub(crate) fn braced(guid: &GUID) -> String {
    format!("{{{guid:?}}}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn check(error: WIN32_ERROR) -> Result<()> {
    if error == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::from(error.to_hresult()))
    }
}

fn clsid_key() -> String {
    format!("CLSID\\{}", braced(&CLSID_TEXT_SERVICE))
}

/// Writes one text value. An empty `name` is the key's default value.
fn set_value(key: &str, name: &str, value: &str) -> Result<()> {
    let key = wide(key);
    let name = wide(name);
    let value = wide(value);
    // The registry takes the text as bytes, with its closing zero.
    let bytes: Vec<u8> = value.iter().flat_map(|unit| unit.to_le_bytes()).collect();
    let mut handle = HKEY::default();
    unsafe {
        check(RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(key.as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut handle,
            None,
        ))?;
        let written = RegSetValueExW(handle, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(handle);
        check(written)
    }
}

fn dll_path() -> Result<String> {
    let mut buffer = [0u16; MAX_PATH as usize];
    let length = unsafe { GetModuleFileNameW(module_instance(), &mut buffer) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err(Error::from_win32());
    }
    Ok(String::from_utf16_lossy(&buffer[..length]))
}

pub(crate) fn register() -> Result<()> {
    let path = dll_path()?;
    set_value(&clsid_key(), "", SERVICE_NAME)?;
    let server = format!("{}\\InprocServer32", clsid_key());
    set_value(&server, "", &path)?;
    set_value(&server, "ThreadingModel", "Apartment")?;

    unsafe {
        let profiles: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        for (profile, name) in PROFILES {
            let name: Vec<u16> = name.encode_utf16().collect();
            profiles.RegisterProfile(
                &CLSID_TEXT_SERVICE,
                LANGUAGE_KHMER,
                &profile,
                &name,
                // No icon: the taskbar shows the language's own sign.
                &[],
                0,
                HKL::default(),
                0,
                TRUE,
                0,
            )?;
        }
        let categories: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for category in CATEGORIES {
            categories.RegisterCategory(&CLSID_TEXT_SERVICE, &category, &CLSID_TEXT_SERVICE)?;
        }
    }
    Ok(())
}

/// Removes everything `register` wrote. Parts that are already gone are
/// skipped, so this can clean up after a registration that failed half way.
pub(crate) fn unregister() -> Result<()> {
    unsafe {
        if let Ok(categories) =
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)
        {
            for category in CATEGORIES {
                let _ = categories.UnregisterCategory(&CLSID_TEXT_SERVICE, &category, &CLSID_TEXT_SERVICE);
            }
        }
        if let Ok(profiles) = CoCreateInstance::<_, ITfInputProcessorProfileMgr>(
            &CLSID_TF_InputProcessorProfiles,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            for (profile, _) in PROFILES {
                let _ = profiles.UnregisterProfile(&CLSID_TEXT_SERVICE, LANGUAGE_KHMER, &profile, 0);
            }
        }
        let key = wide(&clsid_key());
        let deleted = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(key.as_ptr()));
        if deleted == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check(deleted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guid_is_written_the_way_the_registry_wants_it() {
        assert_eq!(braced(&CLSID_TEXT_SERVICE), "{1AF035EE-A180-4349-AD63-A5FC35FE4122}");
    }

    /// The install script adds the keyboards to the user's language list by
    /// these same identities, written out by hand.
    #[test]
    fn the_install_script_names_the_same_keyboards() {
        let script = include_str!("../../../windows/install.ps1");
        assert!(script.contains(&format!("$TextService = '{}'", braced(&CLSID_TEXT_SERVICE))));
        for (profile, name) in PROFILES {
            assert!(script.contains(&format!("'{}'", braced(&profile))), "{name} is missing from install.ps1");
        }
        assert!(script.contains(&format!("\"{LANGUAGE_KHMER:04X}:$TextService$_\"")));
    }
}
