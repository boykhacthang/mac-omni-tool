//! CoreGraphics (`CGEventTap`), CoreFoundation (`CFRunLoop`), Carbon (`RegisterEventHotKey`), and Objective-C FFI declarations.

use std::ffi::{c_char, c_void};

pub type CFMachPortRef = *mut c_void;
pub type CFRunLoopSourceRef = *mut c_void;
pub type CFRunLoopRef = *mut c_void;
pub type CGEventRef = *mut c_void;
pub type CGEventTapProxy = *mut c_void;

// kCGHIDEventTap = 0 intercepts Cmd+Tab and Option+Tab BEFORE macOS Dock/WindowServer
pub const K_CG_HID_EVENT_TAP: u32 = 0;
pub const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
pub const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;

pub const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
pub const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
pub const K_CG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
pub const K_CG_EVENT_RIGHT_MOUSE_UP: u32 = 4;
pub const K_CG_EVENT_KEY_DOWN: u32 = 10;
pub const K_CG_EVENT_KEY_UP: u32 = 11;
pub const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
pub const K_CG_EVENT_SCROLL_WHEEL: u32 = 22;
pub const K_CG_EVENT_OTHER_MOUSE_DOWN: u32 = 25;
pub const K_CG_EVENT_OTHER_MOUSE_UP: u32 = 26;
pub const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;

pub const K_CG_MOUSE_EVENT_BUTTON_NUMBER: u32 = 3;
pub const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
pub const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;

// ScrollWheel CGEventField constants
pub const K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_1: u32 = 11;
pub const K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_2: u32 = 12;
pub const K_CG_SCROLL_WHEEL_EVENT_IS_CONTINUOUS: u32 = 88;
pub const K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_1: u32 = 93;
pub const K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_2: u32 = 94;
pub const K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1: u32 = 96;
pub const K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2: u32 = 97;
pub const K_CG_SCROLL_WHEEL_EVENT_SCROLL_PHASE: u32 = 99;
pub const K_CG_SCROLL_WHEEL_EVENT_MOMENTUM_PHASE: u32 = 123;

// CGScrollEventUnit: 0 = Pixel, 1 = Line
pub const K_CG_SCROLL_EVENT_UNIT_PIXEL: u32 = 0;
pub const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;

/// Magic tag set on synthetic CGEvents created by Mac Omni Tool so our own CGEventTap passes them through untouched.
pub const OMNI_SYNTHETIC_EVENT_TAG: i64 = 0x4F4D_4E49_5359_4E54; // "OMNISYNT"

pub const FLAG_SHIFT: u64 = 0x0002_0000;
pub const FLAG_CONTROL: u64 = 0x0004_0000;
pub const FLAG_OPTION: u64 = 0x0008_0000;
pub const FLAG_COMMAND: u64 = 0x0010_0000;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(
            CGEventTapProxy,
            u32,
            CGEventRef,
            *mut c_void,
        ) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    pub fn CGEventGetDoubleValueField(event: CGEventRef, field: u32) -> f64;
    pub fn CGEventSetDoubleValueField(event: CGEventRef, field: u32, value: f64);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventSetFlags(event: CGEventRef, flags: u64);
    pub fn CGEventSourceCreate(state_id: i32) -> *mut c_void;
    pub fn CGEventCreateKeyboardEvent(
        source: *mut c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> CGEventRef;
    pub fn CGEventCreateScrollWheelEvent(
        source: *mut c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        ...
    ) -> CGEventRef;
    pub fn CGEventPost(tap: u32, event: CGEventRef);
    pub fn CGSMainConnectionID() -> u32;
    pub fn CGSCopyManagedDisplaySpaces(conn: u32) -> *mut c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFRunLoopCommonModes: *const c_void;
    pub fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    pub fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    pub fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
    pub fn CFRunLoopRun();
    pub fn CFRelease(cf: *const c_void);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct EventHotKeyID {
    pub signature: u32,
    pub id: u32,
}

pub const K_EVENT_CLASS_KEYBOARD: u32 = 0x6b65_7962; // 'keyb'
pub const K_EVENT_HOT_KEY_PRESSED: u32 = 5;
pub const K_EVENT_PARAM_DIRECT_OBJECT: u32 = 0x2d2d_2d2d; // '----'
pub const TYPE_EVENT_HOT_KEY_ID: u32 = 0x686b_6964; // 'hkid'

pub const CARBON_CMD_KEY: u32 = 1 << 8;
pub const CARBON_SHIFT_KEY: u32 = 1 << 9;
pub const CARBON_OPTION_KEY: u32 = 1 << 11;
pub const CARBON_CONTROL_KEY: u32 = 1 << 12;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn GetApplicationEventTarget() -> *mut c_void;
    pub fn GetEventDispatcherTarget() -> *mut c_void;
    pub fn InstallEventHandler(
        in_target: *mut c_void,
        in_handler: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32,
        in_num_types: u32,
        in_list: *const EventTypeSpec,
        in_user_data: *mut c_void,
        out_ref: *mut *mut c_void,
    ) -> i32;
    pub fn RegisterEventHotKey(
        in_hot_key_code: u32,
        in_hot_key_modifiers: u32,
        in_hot_key_id: EventHotKeyID,
        in_target: *mut c_void,
        in_options: u32,
        out_ref: *mut *mut c_void,
    ) -> i32;
    pub fn UnregisterEventHotKey(in_hot_key: *mut c_void) -> i32;
    pub fn GetEventParameter(
        in_event: *mut c_void,
        in_name: u32,
        in_desired_type: u32,
        out_actual_type: *mut u32,
        in_buffer_size: usize,
        out_actual_size: *mut usize,
        out_data: *mut c_void,
    ) -> i32;
}

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

