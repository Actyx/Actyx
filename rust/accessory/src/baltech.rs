#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(clippy::redundant_static_lifetimes)]
pub(crate) mod c_api {
    include!(concat!(env!("OUT_DIR"), "/bindings_baltech_api.rs"));
}

use crate::AccessoryError;
use c_api::{
    brp_CardFamilies, brp_CardType, brp_VHL_GetSnr, brp_VHL_Select, brp_create, brp_create_usb_hid, brp_destroy,
    brp_errcode, brp_map_errcode, brp_map_errcode_to_desc, brp_mempool, brp_open, brp_protocol, brp_set_io,
    BRP_ERRGRP_DEVICE, BRP_OK,
};
use std::convert::From;
use std::ffi::CStr;
use std::fmt;
use tracing::*;

/// Struct wrapping Baltech API's `brp_errcode`, providing some helpful methods.
#[derive(Debug)]
pub struct BrpResult {
    inner: brp_errcode,
}

macro_rules! try_brp {
    ($l:expr) => {
        unsafe {
            let res: BrpResult = ($l).into();
            if res.is_err() {
                return Err(AccessoryError::BaltechApiError(res).into());
            } else {
                res
            }
        }
    };
}

impl BrpResult {
    pub fn is_err(&self) -> bool {
        self.inner != BRP_OK
    }
    pub fn is_missing_tag(&self) -> bool {
        // Rust bindgen is unable to expand the macro, not sure why ..
        // taken from baltech_api/c/cmds/vhl.h l.31
        let tag = (0x0100 & 0xFF00) | (0x01 & 0xFF) | BRP_ERRGRP_DEVICE;
        self.inner == tag
    }
}

impl From<brp_errcode> for BrpResult {
    fn from(inner: brp_errcode) -> Self {
        Self { inner }
    }
}

impl fmt::Display for BrpResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errcode = unsafe {
            let str = brp_map_errcode(self.inner);
            CStr::from_ptr(str)
        };
        let desc = unsafe {
            let str = brp_map_errcode_to_desc(self.inner);
            CStr::from_ptr(str)
        };
        write!(f, "{:?} ({:?})", errcode, desc)
    }
}
#[allow(dead_code)]
pub struct Context {
    device_handle: brp_protocol,
    protocol_handle: brp_protocol,
    card_filter: brp_CardFamilies,
}

impl fmt::Display for brp_CardType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Mapping from `baltech/appnotes/vhl/appnote_vhl.c
        let desc = match *self {
            Self::brp_CardType_MifareClassic => "MifareClassic",
            Self::brp_CardType_Iso14443aGeneric => "Iso14443aGeneric",
            Self::brp_CardType_Iso14443aInterIndustry => "Iso14443aInterIndustry",
            Self::brp_CardType_MifareUltraLight => "MifareUltraLight",
            Self::brp_CardType_MifareDesfire => "MifareDesfire",
            Self::brp_CardType_InfineonSle55 => "InfineonSle55",
            Self::brp_CardType_Iso14443aIntIndustryMif => "Iso14443aIntIndustryMif",
            Self::brp_CardType_MifarePlusL2 => "MifarePlusL2",
            Self::brp_CardType_LEGICAdvantIso14443a => "LEGICAdvantIso14443a",
            Self::brp_CardType_MifarePlusL3 => "MifarePlusL3",
            Self::brp_CardType_LEGICPrimeLegacy => "LEGICPrimeLegacy",
            Self::brp_CardType_LEGICAdvantLegacy => "LEGICAdvantLegacy",
            Self::brp_CardType_Iso15693 => "Iso15693",
            Self::brp_CardType_LEGICAdvantIso15693 => "LEGICAdvantIso15693",
            Self::brp_CardType_Iso14443bUnknown => "Iso14443bUnknown",
            Self::brp_CardType_Iso14443bIntIndustry => "Iso14443bIntIndustry",
            Self::brp_CardType_IClassIso14B => "IClassIso14B",
            Self::brp_CardType_IClassIso14B2 => "IClassIso14B2",
            Self::brp_CardType_IClass => "IClass",
            Self::brp_CardType_Felica => "Felica",
            Self::brp_CardType_EM4205 => "EM4205",
            Self::brp_CardType_EM4100 => "EM4100",
            Self::brp_CardType_EM4450 => "EM4450",
            Self::brp_CardType_Pyramid => "Pyramid",
            Self::brp_CardType_HidProx32 => "HidProx32",
            Self::brp_CardType_Keri => "Keri",
            Self::brp_CardType_Quadrakey => "Quadrakey",
            Self::brp_CardType_HidIndala => "HidIndala",
            Self::brp_CardType_HidAwid => "HidAwid",
            Self::brp_CardType_HidProx => "HidProx",
            Self::brp_CardType_HidIoprox => "HidIoprox",
            Self::brp_CardType_Hitag1S => "Hitag1S",
            Self::brp_CardType_Hitag2M => "Hitag2M",
            Self::brp_CardType_Hitag2B => "Hitag2B",
            Self::brp_CardType_TTF => "TTF",
            Self::brp_CardType_STSRIX => "STSRIX",
            Self::brp_CardType_SecuraKey => "SecuraKey",
            Self::brp_CardType_GProx => "GProx",
            Self::brp_CardType_HidIndalaSecure => "HidIndalaSecure",
            Self::brp_CardType_Cotag => "Cotag",
            Self::brp_CardType_Idteck => "Idteck",
            Self::brp_CardType_BluetoothMce => "BluetoothMce",
            Self::brp_CardType_LEGICPrime => "LEGICPrime",
            Self::brp_CardType_HidSio => "HidSio",
            Self::brp_CardType_GemaltoDesfire => "GemaltoDesfire",
            _ => "UNKNOWN",
        };
        write!(f, "{}", desc)
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        info!("Destroying device connection");
        unsafe { brp_destroy(self.device_handle) };
    }
}
impl Context {
    /// Defaults to using VHL using USB HID reading ISO-14443-{A,B} cards.
    pub fn open() -> anyhow::Result<Self> {
        let device_handle = unsafe { brp_create() };
        let protocol_handle = unsafe { brp_create_usb_hid(0) };
        info!("protocol_handle {:?}", protocol_handle);
        // Compose protocol stack
        try_brp!(brp_set_io(device_handle, protocol_handle));
        trace!("Set io stack");
        try_brp!(brp_open(device_handle));
        info!("Opened connection to reader");
        Ok(Self {
            device_handle,
            protocol_handle,
            card_filter: brp_CardFamilies {
                Iso14443A: true,
                Iso14443B: true,
                ..Default::default()
            },
        })
    }
    pub fn block_read_uid(&self, reselect: bool) -> anyhow::Result<Vec<u8>> {
        loop {
            match self.try_read_uid(reselect) {
                Err(e) => match e.downcast_ref::<AccessoryError>() {
                    Some(AccessoryError::NoCardPresent) => {}
                    _ => break Err(e),
                },
                o => break o,
            }
        }
    }
    pub fn try_read_uid(&self, reselect: bool) -> anyhow::Result<Vec<u8>> {
        let mut card_type = brp_CardType::brp_CardType_Default;
        // Can't use try_brp! here, as an error indicating a missing tag on the read is special cased below
        let select_err: BrpResult =
            unsafe { brp_VHL_Select(self.device_handle, self.card_filter, reselect, false, &mut card_type) }.into();
        match select_err {
            e if e.is_missing_tag() => return Err(AccessoryError::NoCardPresent.into()),
            e if e.is_err() => return Err(AccessoryError::BaltechApiError(e).into()),
            _ => {}
        }
        trace!("Selected card on reader (Type: {})", card_type);
        let mut ptr = std::ptr::null_mut();
        let mut ptr_size = 0u64;
        try_brp!(brp_VHL_GetSnr(
            self.device_handle,
            &mut ptr,
            &mut ptr_size,
            std::ptr::null::<brp_mempool>() as *mut _,
        ));
        trace!("Read Card SNR, len: {}", ptr_size);
        let mut bytes = unsafe { std::slice::from_raw_parts(ptr, ptr_size as usize).to_vec() };
        // Expected uid is reversed
        bytes.reverse();
        Ok(bytes)
    }
}
