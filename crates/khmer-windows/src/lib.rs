//! The Khmer keyboard as a Windows input method.
//!
//! This builds `khmer_tsf.dll`, a "text service" for the Windows Text Services
//! Framework (TSF). Once it is registered, Windows lists two keyboards under
//! the Khmer language and loads this DLL into each app the user types in:
//!
//! * **Singkhmer**: Latin letters that spell Khmer words, with suggestions.
//! * **Khmer**: the standard Khmer layout, with every syllable stored in the
//!   correct order whichever order its parts are typed in.
//!
//! The parts:
//!
//! * `lib.rs` (this file): what Windows calls to load the DLL and to make a
//!   text service.
//! * `service.rs`: the text service. Key presses in, text out.
//! * `keys.rs`: which key does what in each mode.
//! * `candidates.rs`: the window that lists the suggestions.
//! * `register.rs`: tells Windows that the keyboard exists (`regsvr32`).
//!
//! All Khmer logic is in `khmer-core`.
#![cfg(windows)]
// The functions Windows calls have the names Windows gives them.
#![allow(non_snake_case)]

pub mod candidates;
pub mod keys;
mod register;
pub mod service;

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows::core::{implement, IUnknown, Interface, Result, GUID, HRESULT};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_POINTER, HINSTANCE, S_FALSE, S_OK, TRUE,
};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::TextServices::ITfTextInputProcessorEx;

use service::{guard, TextService};

/// The identity of the text service. Windows finds the DLL by it.
pub const CLSID_TEXT_SERVICE: GUID = GUID::from_u128(0x1AF035EE_A180_4349_AD63_A5FC35FE4122);
/// The two keyboards the service offers.
pub const PROFILE_SINGKHMER: GUID = GUID::from_u128(0x2E577226_7197_4E08_93B6_E5F7DA9937F4);
pub const PROFILE_KHMER: GUID = GUID::from_u128(0x7B70356C_154C_445B_B08A_E771770C5538);
/// The look of text being typed (a dotted underline).
pub const DISPLAY_ATTRIBUTE_INPUT: GUID = GUID::from_u128(0x6A6FACF4_1BAA_4FC5_8EC9_D1E3D22E67D0);

/// Khmer (Cambodia), as Windows numbers languages.
pub const LANGUAGE_KHMER: u16 = 0x0453;

/// This DLL, as Windows handed it over when loading it.
static INSTANCE: AtomicIsize = AtomicIsize::new(0);

/// The module that holds this code: the DLL, or the program itself when the
/// code is built into one (the tests).
pub(crate) fn module_instance() -> HINSTANCE {
    let instance = INSTANCE.load(Ordering::Relaxed);
    if instance != 0 {
        return HINSTANCE(instance as *mut c_void);
    }
    unsafe { GetModuleHandleW(None) }.map(|module| HINSTANCE(module.0)).unwrap_or_default()
}

/// Called by Windows when the DLL is loaded into a program.
#[no_mangle]
pub extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        INSTANCE.store(instance.0 as isize, Ordering::Relaxed);
    }
    TRUE
}

/// Makes text services.
#[implement(IClassFactory)]
struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(&self, outer: Option<&IUnknown>, iid: *const GUID, object: *mut *mut c_void) -> Result<()> {
        guard(|| {
            if object.is_null() {
                return Err(E_POINTER.into());
            }
            unsafe { *object = std::ptr::null_mut() };
            if outer.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let service: ITfTextInputProcessorEx = TextService::new().into();
            unsafe { service.query(iid, object).ok() }
        })
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// Called by Windows to get the object that makes text services.
///
/// # Safety
/// `clsid` and `iid` must point to GUIDs, and `object` to room for a pointer,
/// as COM promises.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if object.is_null() || clsid.is_null() || iid.is_null() {
        return E_POINTER;
    }
    *object = std::ptr::null_mut();
    if *clsid != CLSID_TEXT_SERVICE {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = ClassFactory.into();
    factory.query(iid, object)
}

/// Windows asks whether it may unload the DLL. Never: the suggestion window's
/// code lives here, and a window must not outlive its code.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

/// `regsvr32 khmer_tsf.dll`: tells Windows the keyboard exists.
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    match guard(register::register) {
        Ok(()) => S_OK,
        Err(error) => {
            // Leave nothing half registered.
            let _ = guard(register::unregister);
            error.code()
        }
    }
}

/// `regsvr32 /u khmer_tsf.dll`: removes the keyboard from Windows.
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    match guard(register::unregister) {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}
