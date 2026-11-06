//! Hardware brightness (`DisplayServices` / `CoreDisplay`) and software gamma dimming / warmth filters.

use crate::appearance::is_dark_mode;
use crate::ffi::*;
use crate::models::{DisplayColorState, DisplayInfo, DisplaySystemState};
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

static DISPLAY_STATES: OnceLock<Mutex<HashMap<u32, DisplayColorState>>> = OnceLock::new();

type DsGetBrightnessFn = unsafe extern "C" fn(u32, *mut f32) -> i32;
type DsSetBrightnessFn = unsafe extern "C" fn(u32, f32) -> i32;
type CdGetBrightnessFn = unsafe extern "C" fn(u32) -> f64;
type CdSetBrightnessFn = unsafe extern "C" fn(u32, f64) -> i32;

struct BrightnessSymbols {
    ds_get: Option<DsGetBrightnessFn>,
    ds_set: Option<DsSetBrightnessFn>,
    cd_get: Option<CdGetBrightnessFn>,
    cd_set: Option<CdSetBrightnessFn>,
}

static BRIGHTNESS_SYMBOLS: OnceLock<BrightnessSymbols> = OnceLock::new();

fn brightness_symbols() -> &'static BrightnessSymbols {
    BRIGHTNESS_SYMBOLS.get_or_init(|| unsafe {
        let mut ds_get = None;
        let mut ds_set = None;
        let mut cd_get = None;
        let mut cd_set = None;

        if let Ok(path) = CString::new(
            "/System/Library/PrivateFrameworks/DisplayServices.framework/DisplayServices",
        ) {
            let handle = libc::dlopen(path.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                if let Ok(sym) = CString::new("DisplayServicesGetBrightness") {
                    let ptr = libc::dlsym(handle, sym.as_ptr());
                    if !ptr.is_null() {
                        ds_get = Some(std::mem::transmute::<*mut std::ffi::c_void, DsGetBrightnessFn>(ptr));
                    }
                }
                if let Ok(sym) = CString::new("DisplayServicesSetBrightness") {
                    let ptr = libc::dlsym(handle, sym.as_ptr());
                    if !ptr.is_null() {
                        ds_set = Some(std::mem::transmute::<*mut std::ffi::c_void, DsSetBrightnessFn>(ptr));
                    }
                }
            }
        }

        if let Ok(cd_path) =
            CString::new("/System/Library/Frameworks/CoreDisplay.framework/CoreDisplay")
        {
            let cd_handle = libc::dlopen(cd_path.as_ptr(), libc::RTLD_LAZY);
            if !cd_handle.is_null() {
                if let Ok(sym) = CString::new("CoreDisplay_Display_GetUserBrightness") {
                    let ptr = libc::dlsym(cd_handle, sym.as_ptr());
                    if !ptr.is_null() {
                        cd_get = Some(std::mem::transmute::<*mut std::ffi::c_void, CdGetBrightnessFn>(ptr));
                    }
                }
                if let Ok(sym) = CString::new("CoreDisplay_Display_SetUserBrightness") {
                    let ptr = libc::dlsym(cd_handle, sym.as_ptr());
                    if !ptr.is_null() {
                        cd_set = Some(std::mem::transmute::<*mut std::ffi::c_void, CdSetBrightnessFn>(ptr));
                    }
                }
            }
        }

        BrightnessSymbols {
            ds_get,
            ds_set,
            cd_get,
            cd_set,
        }
    })
}

fn state_map() -> &'static Mutex<HashMap<u32, DisplayColorState>> {
    DISPLAY_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compute RGB gamma maximum channels `(r_max, g_max, b_max)` from software dimming and Night Shift warmth.
pub fn compute_gamma_channels(software_dimming: f32, warmth: f32) -> (f32, f32, f32) {
    let dim = software_dimming.clamp(0.15, 1.0);
    let w = warmth.clamp(0.0, 1.0);

    let r_max = dim;
    let g_max = (dim * (1.0 - w * 0.22)).clamp(0.1, 1.0);
    let b_max = (dim * (1.0 - w * 0.50)).clamp(0.05, 1.0);
    (r_max, g_max, b_max)
}

/// Try reading hardware brightness via cached `DisplayServices` or `CoreDisplay` symbols.
fn get_hardware_brightness(display_id: u32) -> Option<f32> {
    let syms = brightness_symbols();
    unsafe {
        if let Some(func) = syms.ds_get {
            let mut val: f32 = 1.0;
            if func(display_id, &mut val) == 0 && (0.0..=1.0).contains(&val) {
                return Some(val);
            }
        }
        if let Some(func) = syms.cd_get {
            let val = func(display_id) as f32;
            if (0.0..=1.0).contains(&val) {
                return Some(val);
            }
        }
    }
    None
}

/// Try setting hardware brightness via cached `DisplayServices` or `CoreDisplay` symbols.
fn set_hardware_brightness_raw(display_id: u32, brightness: f32) -> bool {
    let clamped = brightness.clamp(0.0, 1.0);
    let syms = brightness_symbols();
    unsafe {
        if let Some(func) = syms.ds_set {
            if func(display_id, clamped) == 0 {
                return true;
            }
        }
        if let Some(func) = syms.cd_set {
            if func(display_id, clamped as f64) == 0 {
                return true;
            }
        }
    }
    false
}

/// Apply gamma formula for software dimming & Night Shift warmth on a specific display.
fn apply_gamma_state(display_id: u32, state: DisplayColorState) {
    let (r_max, g_max, b_max) = compute_gamma_channels(state.software_dimming, state.warmth);
    unsafe {
        let _ = CGSetDisplayTransferByFormula(
            display_id, 0.0, r_max, 1.0, 0.0, g_max, 1.0, 0.0, b_max, 1.0,
        );
    }
}

/// List all connected displays and their current brightness / dimming / warmth state.
pub fn list_displays() -> Vec<DisplayInfo> {
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    let err = unsafe { CGGetActiveDisplayList(16, ids.as_mut_ptr(), &mut count) };
    if err != 0 {
        return Vec::new();
    }

    let mut map = state_map().lock().unwrap();
    let mut result = Vec::with_capacity(count as usize);

    for (idx, &id) in ids[..count as usize].iter().enumerate() {
        let width = unsafe { CGDisplayPixelsWide(id) };
        let height = unsafe { CGDisplayPixelsHigh(id) };
        let bounds = unsafe { CGDisplayBounds(id) };
        let is_main = unsafe { CGDisplayIsMain(id) != 0 };
        let is_builtin = unsafe { CGDisplayIsBuiltin(id) != 0 };
        let vendor_id = unsafe { CGDisplayVendorNumber(id) };
        let model_id = unsafe { CGDisplayModelNumber(id) };

        let state = *map.entry(id).or_default();
        let hw_brightness = get_hardware_brightness(id);
        let supports_hw = hw_brightness.is_some();

        let brightness = hw_brightness.unwrap_or(state.software_dimming);

        let name = if is_builtin {
            "Màn hình MacBook (Built-in Retina)".to_string()
        } else {
            format!("Màn hình rời #{} ({}x{})", idx + 1, width, height)
        };

        result.push(DisplayInfo {
            id,
            name,
            width,
            height,
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            is_main,
            is_builtin,
            vendor_id,
            model_id,
            brightness,
            software_dimming: state.software_dimming,
            warmth: state.warmth,
            supports_hardware_brightness: supports_hw,
        });
    }

    result
}

/// Set brightness for a specific display (uses hardware brightness when supported, falls back to software gamma dimming).
pub fn set_display_brightness(display_id: u32, brightness: f32) -> Result<DisplayInfo, String> {
    let clamped = brightness.clamp(0.05, 1.0);
    let hw_ok = set_hardware_brightness_raw(display_id, clamped);

    {
        let mut map = state_map().lock().unwrap();
        let entry = map.entry(display_id).or_default();
        if !hw_ok {
            entry.software_dimming = clamped.clamp(0.15, 1.0);
            apply_gamma_state(display_id, *entry);
        }
    }

    list_displays()
        .into_iter()
        .find(|d| d.id == display_id)
        .ok_or_else(|| format!("Không tìm thấy màn hình ID {}", display_id))
}

/// Explicitly set software dimming (0.15 .. 1.0) and color warmth (0.0 .. 1.0) for a display.
pub fn set_display_color_filter(
    display_id: u32,
    software_dimming: f32,
    warmth: f32,
) -> Result<DisplayInfo, String> {
    {
        let mut map = state_map().lock().unwrap();
        let entry = map.entry(display_id).or_default();
        entry.software_dimming = software_dimming.clamp(0.15, 1.0);
        entry.warmth = warmth.clamp(0.0, 1.0);
        apply_gamma_state(display_id, *entry);
    }

    list_displays()
        .into_iter()
        .find(|d| d.id == display_id)
        .ok_or_else(|| format!("Không tìm thấy màn hình ID {}", display_id))
}

/// Reset all software gamma filters back to macOS ColorSync defaults.
pub fn reset_all_display_filters() {
    {
        let mut map = state_map().lock().unwrap();
        for state in map.values_mut() {
            state.software_dimming = 1.0;
            state.warmth = 0.0;
        }
    }
    unsafe {
        CGDisplayRestoreColorSyncSettings();
    }
}

pub fn get_display_system_state() -> DisplaySystemState {
    DisplaySystemState {
        displays: list_displays(),
        dark_mode: is_dark_mode(),
    }
}

