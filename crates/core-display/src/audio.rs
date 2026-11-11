//! CoreAudio volume control and fast output/input device switching (AirPods / Speakers / Headphones / USB DAC).

use crate::ffi::*;
use crate::models::{AudioDeviceInfo, AudioSystemState};
use std::process::Command;

const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;
const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT: u32 = u32::from_be_bytes(*b"outp");
const K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT: u32 = u32::from_be_bytes(*b"inpt");
const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: u32 = 0;

const K_AUDIO_HARDWARE_PROPERTY_DEVICES: u32 = u32::from_be_bytes(*b"dev#");
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: u32 = u32::from_be_bytes(*b"dOut");
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE: u32 = u32::from_be_bytes(*b"sOut");
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE: u32 = u32::from_be_bytes(*b"dIn ");
const K_AUDIO_OBJECT_PROPERTY_NAME: u32 = u32::from_be_bytes(*b"lnam");
const K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE: u32 = u32::from_be_bytes(*b"tran");
const K_AUDIO_DEVICE_PROPERTY_STREAMS: u32 = u32::from_be_bytes(*b"stm#");
const K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR: u32 = u32::from_be_bytes(*b"volm");
const K_AUDIO_DEVICE_PROPERTY_MUTE: u32 = u32::from_be_bytes(*b"mute");
const K_AUDIO_HARDWARE_SERVICE_VIRTUAL_MAIN_VOLUME: u32 = u32::from_be_bytes(*b"vmvc");
const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

unsafe fn get_default_audio_device(selector: u32) -> u32 {
    let addr = AudioObjectPropertyAddress {
        m_selector: selector,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut dev_id: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = AudioObjectGetPropertyData(
        K_AUDIO_OBJECT_SYSTEM_OBJECT,
        &addr,
        0,
        std::ptr::null(),
        &mut size,
        &mut dev_id as *mut u32 as *mut std::ffi::c_void,
    );
    if status == 0 {
        dev_id
    } else {
        0
    }
}

unsafe fn get_audio_device_name(dev_id: u32) -> String {
    let addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_OBJECT_PROPERTY_NAME,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut cf_str: *const std::ffi::c_void = std::ptr::null();
    let mut size = std::mem::size_of::<*const std::ffi::c_void>() as u32;
    let status = AudioObjectGetPropertyData(
        dev_id,
        &addr,
        0,
        std::ptr::null(),
        &mut size,
        &mut cf_str as *mut *const std::ffi::c_void as *mut std::ffi::c_void,
    );
    if status != 0 || cf_str.is_null() {
        return format!("Audio Device #{}", dev_id);
    }
    let mut buf = vec![0i8; 256];
    let ok = CFStringGetCString(
        cf_str,
        buf.as_mut_ptr(),
        buf.len() as isize,
        K_CF_STRING_ENCODING_UTF8,
    );
    CFRelease(cf_str);
    if ok {
        std::ffi::CStr::from_ptr(buf.as_ptr())
            .to_string_lossy()
            .into_owned()
    } else {
        format!("Audio Device #{}", dev_id)
    }
}

unsafe fn get_audio_device_transport(dev_id: u32) -> u32 {
    let addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut transport: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    if AudioObjectGetPropertyData(
        dev_id,
        &addr,
        0,
        std::ptr::null(),
        &mut size,
        &mut transport as *mut u32 as *mut std::ffi::c_void,
    ) == 0
    {
        transport
    } else {
        0
    }
}

unsafe fn device_is_alive(dev_id: u32) -> bool {
    const K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE: u32 = 0x6C69_766E; // 'livn'
    const K_AUDIO_DEVICE_PROPERTY_IS_HIDDEN: u32 = 0x6869_646E; // 'hidn'

    let alive_addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    if AudioObjectHasProperty(dev_id, &alive_addr) != 0 {
        let mut alive: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        if AudioObjectGetPropertyData(
            dev_id,
            &alive_addr,
            0,
            std::ptr::null(),
            &mut size,
            &mut alive as *mut u32 as *mut std::ffi::c_void,
        ) == 0
            && alive == 0
        {
            return false;
        }
    }

    let hidden_addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_IS_HIDDEN,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    if AudioObjectHasProperty(dev_id, &hidden_addr) != 0 {
        let mut hidden: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        if AudioObjectGetPropertyData(
            dev_id,
            &hidden_addr,
            0,
            std::ptr::null(),
            &mut size,
            &mut hidden as *mut u32 as *mut std::ffi::c_void,
        ) == 0
            && hidden != 0
        {
            return false;
        }
    }

    true
}

unsafe fn device_has_streams(dev_id: u32, scope: u32) -> bool {
    if !device_is_alive(dev_id) {
        return false;
    }
    let addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_DEVICE_PROPERTY_STREAMS,
        m_scope: scope,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut size: u32 = 0;
    if AudioObjectGetPropertyDataSize(dev_id, &addr, 0, std::ptr::null(), &mut size) == 0 {
        size > 0
    } else {
        false
    }
}

/// Classify a CoreAudio device by name and transport four-char code into `(kind_label, icon)`.
pub fn classify_audio_device(name: &str, transport: u32) -> (String, String) {
    let lower = name.to_lowercase();
    let t_bytes = transport.to_be_bytes();
    let t_str = std::str::from_utf8(&t_bytes).unwrap_or("");

    if lower.contains("airpod") {
        ("AirPods".to_string(), "🎧".to_string())
    } else if t_str == "blut" || t_str == "blea" || lower.contains("bluetooth") {
        ("Tai nghe Bluetooth".to_string(), "🎧".to_string())
    } else if lower.contains("headphone")
        || lower.contains("tai nghe")
        || lower.contains("earphone")
    {
        ("Tai nghe có dây".to_string(), "🎧".to_string())
    } else if lower.contains("speaker") || lower.contains("loa") || t_str == "bltn" {
        ("Loa ngoài MacBook".to_string(), "🔊".to_string())
    } else if t_str == "hdmi"
        || t_str == "dprt"
        || lower.contains("hdmi")
        || lower.contains("display")
    {
        ("Loa Màn hình".to_string(), "🖥️".to_string())
    } else if t_str == "usb " || lower.contains("usb") {
        ("USB Audio / DAC".to_string(), "🎛️".to_string())
    } else {
        ("Thiết bị âm thanh".to_string(), "🔈".to_string())
    }
}

unsafe fn get_device_volume_and_mute(dev_id: u32) -> (f32, bool) {
    if dev_id == 0 {
        return (0.5, false);
    }

    // 1. Try VirtualMainVolume ('vmvc') first
    let vmvc_addr = AudioObjectPropertyAddress {
        m_selector: K_AUDIO_HARDWARE_SERVICE_VIRTUAL_MAIN_VOLUME,
        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
        m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
    };
    let mut vol: f32 = -1.0;
    let mut size = std::mem::size_of::<f32>() as u32;
    if AudioObjectHasProperty(dev_id, &vmvc_addr) != 0 {
        let mut v: f32 = 0.0;
        if AudioObjectGetPropertyData(
            dev_id,
            &vmvc_addr,
            0,
            std::ptr::null(),
            &mut size,
            &mut v as *mut f32 as *mut std::ffi::c_void,
        ) == 0
        {
            vol = v.clamp(0.0, 1.0);
        }
    }

    // 2. Fallback to kAudioDevicePropertyVolumeScalar on element 0, 1, 2
    if vol < 0.0 {
        for elem in [0u32, 1u32, 2u32] {
            let addr = AudioObjectPropertyAddress {
                m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
                m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
                m_element: elem,
            };
            if AudioObjectHasProperty(dev_id, &addr) != 0 {
                let mut v: f32 = 0.0;
                let mut sz = std::mem::size_of::<f32>() as u32;
                if AudioObjectGetPropertyData(
                    dev_id,
                    &addr,
                    0,
                    std::ptr::null(),
                    &mut sz,
                    &mut v as *mut f32 as *mut std::ffi::c_void,
                ) == 0
                {
                    vol = v.clamp(0.0, 1.0);
                    break;
                }
            }
        }
    }

    if vol < 0.0 {
        vol = 1.0;
    }

    // Check mute state
    let mut muted = false;
    for elem in [0u32, 1u32] {
        let mute_addr = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
            m_element: elem,
        };
        if AudioObjectHasProperty(dev_id, &mute_addr) != 0 {
            let mut m: u32 = 0;
            let mut sz = std::mem::size_of::<u32>() as u32;
            if AudioObjectGetPropertyData(
                dev_id,
                &mute_addr,
                0,
                std::ptr::null(),
                &mut sz,
                &mut m as *mut u32 as *mut std::ffi::c_void,
            ) == 0
            {
                muted = m != 0;
                break;
            }
        }
    }

    (vol, muted)
}

/// Get all audio output/input devices and current volume/mute state.
pub fn get_audio_system_state() -> AudioSystemState {
    unsafe {
        let default_out =
            get_default_audio_device(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE);
        let default_in = get_default_audio_device(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE);

        let dev_addr = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_HARDWARE_PROPERTY_DEVICES,
            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };

        let mut data_size: u32 = 0;
        let mut output_devices = Vec::new();
        let mut input_devices = Vec::new();

        if AudioObjectGetPropertyDataSize(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &dev_addr,
            0,
            std::ptr::null(),
            &mut data_size,
        ) == 0
            && data_size > 0
        {
            let count = (data_size as usize) / std::mem::size_of::<u32>();
            let mut ids = vec![0u32; count];
            if AudioObjectGetPropertyData(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &dev_addr,
                0,
                std::ptr::null(),
                &mut data_size,
                ids.as_mut_ptr() as *mut std::ffi::c_void,
            ) == 0
            {
                for id in ids {
                    let is_out = device_has_streams(id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT);
                    let is_in = device_has_streams(id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT);
                    if !is_out && !is_in {
                        continue;
                    }
                    let name = get_audio_device_name(id);
                    if name.contains("CADefaultDeviceAggregate") {
                        continue;
                    }
                    let transport = get_audio_device_transport(id);
                    let (kind_label, icon) = classify_audio_device(&name, transport);

                    let info = AudioDeviceInfo {
                        id,
                        name,
                        kind_label,
                        icon,
                        is_output: is_out,
                        is_input: is_in,
                        is_default_output: id == default_out,
                        is_default_input: id == default_in,
                    };

                    if is_out {
                        output_devices.push(info.clone());
                    }
                    if is_in {
                        input_devices.push(info);
                    }
                }
            }
        }

        let (volume, muted) = get_device_volume_and_mute(default_out);
        let current_output_name = output_devices
            .iter()
            .find(|d| d.id == default_out)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| get_audio_device_name(default_out));

        AudioSystemState {
            volume,
            muted,
            current_output_id: default_out,
            current_output_name,
            output_devices,
            input_devices,
        }
    }
}

/// Switch active audio output (or input) device (e.g. AirPods <-> Loa ngoài MacBook <-> Tai nghe).
pub fn set_audio_device(device_id: u32, is_output: bool) -> Result<AudioSystemState, String> {
    unsafe {
        if is_output {
            for selector in [
                K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
                K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE,
            ] {
                let addr = AudioObjectPropertyAddress {
                    m_selector: selector,
                    m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                    m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
                };
                let _ = AudioObjectSetPropertyData(
                    K_AUDIO_OBJECT_SYSTEM_OBJECT,
                    &addr,
                    0,
                    std::ptr::null(),
                    std::mem::size_of::<u32>() as u32,
                    &device_id as *const u32 as *const std::ffi::c_void,
                );
            }
        } else {
            let addr = AudioObjectPropertyAddress {
                m_selector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
                m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
                m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
            };
            let _ = AudioObjectSetPropertyData(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &addr,
                0,
                std::ptr::null(),
                std::mem::size_of::<u32>() as u32,
                &device_id as *const u32 as *const std::ffi::c_void,
            );
        }
    }
    Ok(get_audio_system_state())
}

/// Set system output volume (`0.0 .. 1.0`) and/or mute state.
pub fn set_audio_volume(volume: Option<f32>, muted: Option<bool>) -> AudioSystemState {
    unsafe {
        let default_out =
            get_default_audio_device(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE);
        if default_out != 0 {
            if let Some(v) = volume {
                let clamped = v.clamp(0.0, 1.0);
                let mut set_ok = false;

                let vmvc_addr = AudioObjectPropertyAddress {
                    m_selector: K_AUDIO_HARDWARE_SERVICE_VIRTUAL_MAIN_VOLUME,
                    m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
                    m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
                };
                if AudioObjectHasProperty(default_out, &vmvc_addr) != 0 {
                    if AudioObjectSetPropertyData(
                        default_out,
                        &vmvc_addr,
                        0,
                        std::ptr::null(),
                        std::mem::size_of::<f32>() as u32,
                        &clamped as *const f32 as *const std::ffi::c_void,
                    ) == 0
                    {
                        set_ok = true;
                    }
                }

                for elem in [0u32, 1u32, 2u32] {
                    let addr = AudioObjectPropertyAddress {
                        m_selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
                        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
                        m_element: elem,
                    };
                    if AudioObjectHasProperty(default_out, &addr) != 0 {
                        if AudioObjectSetPropertyData(
                            default_out,
                            &addr,
                            0,
                            std::ptr::null(),
                            std::mem::size_of::<f32>() as u32,
                            &clamped as *const f32 as *const std::ffi::c_void,
                        ) == 0
                        {
                            set_ok = true;
                        }
                    }
                }

                if !set_ok {
                    let pct = (clamped * 100.0).round() as u32;
                    let _ = Command::new("osascript")
                        .args(["-e", &format!("set volume output volume {}", pct)])
                        .status();
                }

                if clamped > 0.01 && muted.is_none() {
                    let unmute_val: u32 = 0;
                    for elem in [0u32, 1u32] {
                        let mute_addr = AudioObjectPropertyAddress {
                            m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
                            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
                            m_element: elem,
                        };
                        if AudioObjectHasProperty(default_out, &mute_addr) != 0 {
                            let _ = AudioObjectSetPropertyData(
                                default_out,
                                &mute_addr,
                                0,
                                std::ptr::null(),
                                std::mem::size_of::<u32>() as u32,
                                &unmute_val as *const u32 as *const std::ffi::c_void,
                            );
                        }
                    }
                }
            }

            if let Some(m) = muted {
                let m_val: u32 = if m { 1 } else { 0 };
                let mut mute_ok = false;
                for elem in [0u32, 1u32] {
                    let mute_addr = AudioObjectPropertyAddress {
                        m_selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
                        m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT,
                        m_element: elem,
                    };
                    if AudioObjectHasProperty(default_out, &mute_addr) != 0 {
                        if AudioObjectSetPropertyData(
                            default_out,
                            &mute_addr,
                            0,
                            std::ptr::null(),
                            std::mem::size_of::<u32>() as u32,
                            &m_val as *const u32 as *const std::ffi::c_void,
                        ) == 0
                        {
                            mute_ok = true;
                        }
                    }
                }
                if !mute_ok {
                    let _ = Command::new("osascript")
                        .args([
                            "-e",
                            &format!(
                                "set volume output muted {}",
                                if m { "true" } else { "false" }
                            ),
                        ])
                        .status();
                }
            }
        }
    }
    get_audio_system_state()
}

