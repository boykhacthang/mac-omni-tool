//! CoreFoundation, IOKit (`IOHIDEventSystemClient`), and Objective-C runtime FFI bindings
//! for reading Apple Silicon hardware thermal sensors without memory leaks.

use std::ffi::{c_void, CStr};

pub type CFAllocatorRef = *const c_void;
pub type CFArrayRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFTypeRef = *const c_void;
pub type CFIndex = isize;
pub type CFStringEncoding = u32;

pub const K_CF_STRING_ENCODING_UTF8: CFStringEncoding = 0x08000100;
pub const K_CF_NUMBER_SINT64_TYPE: CFIndex = 4;
pub const K_IOHID_EVENT_TYPE_TEMPERATURE: i64 = 15;
pub const K_IOHID_EVENT_FIELD_TEMPERATURE_LEVEL: i32 = 15 << 16;

#[link(name = "objc")]
extern "C" {
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFArrayGetCount(the_array: CFArrayRef) -> CFIndex;
    pub fn CFArrayGetValueAtIndex(the_array: CFArrayRef, idx: CFIndex) -> *const c_void;
    pub fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        c_str: *const i8,
        encoding: CFStringEncoding,
    ) -> CFStringRef;
    pub fn CFStringGetCString(
        the_string: CFStringRef,
        buffer: *mut i8,
        buffer_size: CFIndex,
        encoding: CFStringEncoding,
    ) -> bool;
    pub fn CFDictionaryGetValue(the_dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
    pub fn CFNumberGetValue(
        number: CFTypeRef,
        the_type: CFIndex,
        value_ptr: *mut c_void,
    ) -> bool;
    pub fn CFRelease(cf: CFTypeRef);
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    pub fn IOHIDEventSystemClientCreate(allocator: CFAllocatorRef) -> CFTypeRef;
    pub fn IOHIDEventSystemClientCopyServices(client: CFTypeRef) -> CFArrayRef;
    pub fn IOHIDServiceClientCopyProperty(service: CFTypeRef, key: CFStringRef) -> CFTypeRef;
    pub fn IOHIDServiceClientCopyEvent(
        service: CFTypeRef,
        event_type: i64,
        options: i32,
        timeout: i64,
    ) -> CFTypeRef;
    pub fn IOHIDEventGetFloatValue(event: CFTypeRef, field: i32) -> f64;
    pub fn IOServiceMatching(name: *const i8) -> CFDictionaryRef;
    pub fn IOServiceGetMatchingServices(
        main_port: u32,
        matching: CFDictionaryRef,
        existing: *mut u32,
    ) -> i32;
    pub fn IOIteratorNext(iterator: u32) -> u32;
    pub fn IOObjectRelease(object: u32) -> i32;
    pub fn IORegistryEntryCreateCFProperties(
        entry: u32,
        properties: *mut CFDictionaryRef,
        allocator: CFAllocatorRef,
        options: u32,
    ) -> i32;
}

/// Convert a `CFStringRef` into a Rust `String`.
pub unsafe fn cfstring_to_rust(cf_str: CFStringRef) -> Option<String> {
    if cf_str.is_null() {
        return None;
    }
    let mut buf = [0i8; 256];
    if CFStringGetCString(
        cf_str,
        buf.as_mut_ptr(),
        buf.len() as CFIndex,
        K_CF_STRING_ENCODING_UTF8,
    ) {
        let c_str = CStr::from_ptr(buf.as_ptr());
        Some(c_str.to_string_lossy().into_owned())
    } else {
        None
    }
}

