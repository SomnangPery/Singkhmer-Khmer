#![cfg(windows)]
#![allow(non_snake_case)]

pub mod candidates;
mod display;
mod edit;
pub mod keys;
mod my_words;
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

pub const CLSID_TEXT_SERVICE: GUID = GUID::from_u128(0x1AF035EE_A180_4349_AD63_A5FC35FE4122);
pub const PROFILE_SINGKHMER: GUID = GUID::from_u128(0x2E577226_7197_4E08_93B6_E5F7DA9937F4);
pub const PROFILE_KHMER: GUID = GUID::from_u128(0x7B70356C_154C_445B_B08A_E771770C5538);
pub const DISPLAY_ATTRIBUTE_INPUT: GUID = GUID::from_u128(0x6A6FACF4_1BAA_4FC5_8EC9_D1E3D22E67D0);

pub const LANGUAGE_KHMER: u16 = 0x0453;

static INSTANCE: AtomicIsize = AtomicIsize::new(0);

pub(crate) fn module_instance() -> HINSTANCE {
    let instance = INSTANCE.load(Ordering::Relaxed);
    if instance != 0 {
        return HINSTANCE(instance as *mut c_void);
    }
    unsafe { GetModuleHandleW(None) }
        .map(|module| HINSTANCE(module.0))
        .unwrap_or_default()
}

#[no_mangle]
pub extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        INSTANCE.store(instance.0 as isize, Ordering::Relaxed);
    }
    TRUE
}

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

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    match guard(register::register) {
        Ok(()) => S_OK,
        Err(error) => {
            let _ = guard(register::unregister);
            error.code()
        }
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    match guard(register::unregister) {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}
