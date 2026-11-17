use std::cell::Cell;

use windows::core::{implement, Error, Result, BSTR, GUID, VARIANT};
use windows::Win32::Foundation::{E_INVALIDARG, S_FALSE};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfContext, ITfDisplayAttributeInfo,
    ITfDisplayAttributeInfo_Impl, ITfRange, GUID_PROP_ATTRIBUTE, TF_ATTR_INPUT, TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};

use crate::DISPLAY_ATTRIBUTE_INPUT;

pub(crate) unsafe fn set_attribute(ec: u32, context: &ITfContext, range: &ITfRange, atom: u32) {
    if atom == 0 {
        return;
    }
    if let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) {
        let _ = property.SetValue(ec, range, &VARIANT::from(atom as i32));
    }
}

pub(crate) unsafe fn clear_attribute(ec: u32, context: &ITfContext, range: &ITfRange) {
    if let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) {
        let _ = property.Clear(ec, range);
    }
}

#[implement(ITfDisplayAttributeInfo)]
pub(crate) struct InputAttribute;

impl ITfDisplayAttributeInfo_Impl for InputAttribute_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(DISPLAY_ATTRIBUTE_INPUT)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("Khmer Keyboard: text being typed"))
    }

    fn GetAttributeInfo(&self, attribute: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        if attribute.is_null() {
            return Err(Error::from(E_INVALIDARG));
        }
        unsafe {
            *attribute = TF_DISPLAYATTRIBUTE {
                lsStyle: TF_LS_DOT,
                bAttr: TF_ATTR_INPUT,
                ..Default::default()
            };
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _attribute: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

#[implement(IEnumTfDisplayAttributeInfo)]
pub(crate) struct AttributeList {
    position: Cell<u32>,
}

impl AttributeList {
    pub(crate) fn new() -> AttributeList {
        AttributeList { position: Cell::new(0) }
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for AttributeList_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(AttributeList {
            position: Cell::new(self.position.get()),
        }
        .into())
    }

    fn Next(&self, count: u32, items: *mut Option<ITfDisplayAttributeInfo>, fetched: *mut u32) -> Result<()> {
        let mut given = 0;
        if count > 0 && !items.is_null() && self.position.get() == 0 {
            unsafe { items.write(Some(InputAttribute.into())) };
            self.position.set(1);
            given = 1;
        }
        if !fetched.is_null() {
            unsafe { *fetched = given };
        }
        if given == count {
            Ok(())
        } else {
            Err(Error::from(S_FALSE))
        }
    }

    fn Reset(&self) -> Result<()> {
        self.position.set(0);
        Ok(())
    }

    fn Skip(&self, count: u32) -> Result<()> {
        self.position.set(self.position.get().saturating_add(count));
        Ok(())
    }
}
