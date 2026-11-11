//! CoreGraphics, CoreFoundation, and CoreAudio FFI bindings for display & audio control.

use std::ffi::{c_char, c_void};

pub type CGDirectDisplayID = u32;
pub type CGError = i32;
pub type CGGammaValue = f32;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> CGError;
    pub fn CGDisplayPixelsWide(display: CGDirectDisplayID) -> usize;
    pub fn CGDisplayPixelsHigh(display: CGDirectDisplayID) -> usize;
    pub fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    pub fn CGDisplayIsMain(display: CGDirectDisplayID) -> i32;
    pub fn CGDisplayIsBuiltin(display: CGDirectDisplayID) -> i32;
    pub fn CGDisplayVendorNumber(display: CGDirectDisplayID) -> u32;
    pub fn CGDisplayModelNumber(display: CGDirectDisplayID) -> u32;
    pub fn CGSetDisplayTransferByFormula(
        display: CGDirectDisplayID,
        red_min: CGGammaValue,
        red_max: CGGammaValue,
        red_gamma: CGGammaValue,
        green_min: CGGammaValue,
        green_max: CGGammaValue,
        green_gamma: CGGammaValue,
        blue_min: CGGammaValue,
        blue_max: CGGammaValue,
        blue_gamma: CGGammaValue,
    ) -> CGError;
    pub fn CGDisplayRestoreColorSyncSettings();
    pub fn CGEventCreate(source: *const c_void) -> *mut c_void;
    pub fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    pub fn CGDisplayCreateImage(display_id: CGDirectDisplayID) -> *mut c_void;
    pub fn CGImageRelease(image: *mut c_void);
    pub fn CGImageGetWidth(image: *const c_void) -> usize;
    pub fn CGImageGetHeight(image: *const c_void) -> usize;
    pub fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    pub fn CGColorSpaceRelease(cs: *mut c_void);
    pub fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    pub fn CGContextDrawImage(c: *mut c_void, rect: CGRect, image: *mut c_void);
    pub fn CGContextRelease(c: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFRelease(cf: *const c_void);
    pub fn CFStringGetCString(
        the_string: *const c_void,
        buffer: *mut c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> bool;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AudioObjectPropertyAddress {
    pub m_selector: u32,
    pub m_scope: u32,
    pub m_element: u32,
}

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    pub fn AudioObjectGetPropertyDataSize(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        out_data_size: *mut u32,
    ) -> i32;
    pub fn AudioObjectGetPropertyData(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> i32;
    pub fn AudioObjectSetPropertyData(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        in_data_size: u32,
        in_data: *const c_void,
    ) -> i32;
    pub fn AudioObjectHasProperty(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
    ) -> u8;
}

#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {}

