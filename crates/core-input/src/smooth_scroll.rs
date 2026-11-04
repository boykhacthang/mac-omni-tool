//! 120Hz Smooth Scroll Engine & Independent Scroll Direction Inverter for External Mice.
//! Automatically ignores Apple Trackpad & Magic Mouse continuous gesture events (`IsContinuous != 0`).

use crate::actions::now_ms;
use crate::config::InputAutomationConfig;
use crate::ffi::*;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

struct SmoothScrollState {
    remaining_y: f64,
    remaining_x: f64,
    subpixel_y: f64,
    subpixel_x: f64,
    duration_ms: u32,
    flags: u64,
    last_notch_at_ms: i64,
    fast_streak: u32,
}

impl Default for SmoothScrollState {
    fn default() -> Self {
        Self {
            remaining_y: 0.0,
            remaining_x: 0.0,
            subpixel_y: 0.0,
            subpixel_x: 0.0,
            duration_ms: 260,
            flags: 0,
            last_notch_at_ms: 0,
            fast_streak: 0,
        }
    }
}

static SCROLL_ENGINE: OnceLock<(Mutex<SmoothScrollState>, Condvar)> = OnceLock::new();

/// Immediately clear any in-flight smooth scroll momentum when Smooth Scroll or External Mouse is turned off.
pub fn stop_smooth_scroll_momentum() {
    if let Some((lock, _)) = SCROLL_ENGINE.get() {
        if let Ok(mut st) = lock.lock() {
            st.remaining_y = 0.0;
            st.remaining_x = 0.0;
            st.subpixel_y = 0.0;
            st.subpixel_x = 0.0;
            st.fast_streak = 0;
        }
    }
}

fn engine_pair() -> &'static (Mutex<SmoothScrollState>, Condvar) {
    SCROLL_ENGINE.get_or_init(|| {
        let pair = (Mutex::new(SmoothScrollState::default()), Condvar::new());
        spawn_smooth_scroll_worker();
        pair
    })
}

fn spawn_smooth_scroll_worker() {
    let _ = std::thread::Builder::new()
        .name("omni-smooth-scroll".to_string())
        .spawn(|| loop {
            let (lock, cvar) = engine_pair();
            let (px_y, px_x, flags) = {
                let mut st = lock.lock().unwrap();
                while st.remaining_y.abs() < 0.35 && st.remaining_x.abs() < 0.35 {
                    st.remaining_y = 0.0;
                    st.remaining_x = 0.0;
                    st.subpixel_y = 0.0;
                    st.subpixel_x = 0.0;
                    st = cvar.wait(st).unwrap();
                }

                // 8ms tick (~120Hz ProMotion frame pacing)
                let tau_ms = (st.duration_ms.clamp(80, 600) as f64 / 3.2).max(20.0);
                let alpha = (1.0 - (-8.0 / tau_ms).exp()).clamp(0.08, 0.45);

                let dy = st.remaining_y * alpha;
                let dx = st.remaining_x * alpha;
                st.remaining_y -= dy;
                st.remaining_x -= dx;

                st.subpixel_y += dy;
                st.subpixel_x += dx;

                let emit_y = st.subpixel_y.trunc() as i32;
                let emit_x = st.subpixel_x.trunc() as i32;
                st.subpixel_y -= emit_y as f64;
                st.subpixel_x -= emit_x as f64;

                (emit_y, emit_x, st.flags)
            };

            if px_y != 0 || px_x != 0 {
                post_smooth_pixel_scroll(px_y, px_x, flags);
            }

            std::thread::sleep(Duration::from_millis(8));
        });
}

fn post_smooth_pixel_scroll(px_y: i32, px_x: i32, flags: u64) {
    unsafe {
        let src = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE);
        let ev = CGEventCreateScrollWheelEvent(src, K_CG_SCROLL_EVENT_UNIT_PIXEL, 2, px_y, px_x);
        if !ev.is_null() {
            CGEventSetIntegerValueField(ev, K_CG_SCROLL_WHEEL_EVENT_IS_CONTINUOUS, 1);
            CGEventSetIntegerValueField(
                ev,
                K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
                px_y as i64,
            );
            CGEventSetIntegerValueField(
                ev,
                K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
                px_x as i64,
            );
            CGEventSetIntegerValueField(ev, K_CG_EVENT_SOURCE_USER_DATA, OMNI_SYNTHETIC_EVENT_TAG);
            // Strip Shift flag if we already converted Shift+Vertical into Horizontal scroll
            let clean_flags = if px_x != 0 && px_y == 0 {
                flags & !FLAG_SHIFT
            } else {
                flags
            };
            CGEventSetFlags(ev, clean_flags);
            CGEventPost(K_CG_HID_EVENT_TAP, ev);
            CFRelease(ev);
        }
        if !src.is_null() {
            CFRelease(src);
        }
    }
}

/// Process a `kCGEventScrollWheel` event inside `CGEventTap`.
/// Returns `event` to pass through, or `null_mut()` when consumed by the 120Hz Smooth Scroll animator.
pub unsafe fn handle_scroll_wheel_event(
    event: CGEventRef,
    cfg: &InputAutomationConfig,
    frontmost_bundle_id: &str,
) -> CGEventRef {
    if !cfg.external_mouse_enabled {
        return event;
    }

    // 1. Ignore Apple Trackpad / Magic Mouse continuous gesture & momentum events
    let is_continuous = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_IS_CONTINUOUS);
    let scroll_phase = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_SCROLL_PHASE);
    let momentum_phase = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_MOMENTUM_PHASE);
    if is_continuous != 0 || scroll_phase != 0 || momentum_phase != 0 {
        return event;
    }

    // 2. Read discrete wheel notch deltas from the external mouse
    let mut dy_lines = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_1);
    let mut dx_lines = CGEventGetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_2);
    if dy_lines == 0 && dx_lines == 0 {
        return event;
    }

    // 3. Independent External Mouse Scroll Inversion (does not touch Trackpad!)
    if cfg.invert_external_mouse_scroll {
        dy_lines = -dy_lines;
        dx_lines = -dx_lines;
    }

    // 4. Shift + Vertical Scroll => Horizontal Scroll
    let flags = CGEventGetFlags(event);
    let shifted_horiz =
        cfg.shift_horizontal_scroll && (flags & FLAG_SHIFT) != 0 && dx_lines == 0 && dy_lines != 0;
    if shifted_horiz {
        dx_lines = dy_lines;
        dy_lines = 0;
    }

    // 5. Check whether Smooth Scroll is active globally and for the frontmost app
    let smooth_active = cfg.is_smooth_scroll_enabled_for_app(frontmost_bundle_id);
    if !smooth_active {
        if cfg.invert_external_mouse_scroll || shifted_horiz {
            let pt_y = CGEventGetIntegerValueField(
                event,
                K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
            );
            let pt_x = CGEventGetIntegerValueField(
                event,
                K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
            );
            let fp_y = CGEventGetDoubleValueField(
                event,
                K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_1,
            );
            let fp_x = CGEventGetDoubleValueField(
                event,
                K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_2,
            );

            let sign = if cfg.invert_external_mouse_scroll {
                -1
            } else {
                1
            };
            let fsign = sign as f64;

            if shifted_horiz {
                CGEventSetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_1, 0);
                CGEventSetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_2, dx_lines);
                CGEventSetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1, 0);
                CGEventSetIntegerValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
                    pt_y * sign,
                );
                CGEventSetDoubleValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_1,
                    0.0,
                );
                CGEventSetDoubleValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_2,
                    fp_y * fsign,
                );
                CGEventSetFlags(event, flags & !FLAG_SHIFT);
            } else {
                CGEventSetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_1, dy_lines);
                CGEventSetIntegerValueField(event, K_CG_SCROLL_WHEEL_EVENT_DELTA_AXIS_2, dx_lines);
                CGEventSetIntegerValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
                    -pt_y,
                );
                CGEventSetIntegerValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
                    -pt_x,
                );
                CGEventSetDoubleValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_1,
                    -fp_y,
                );
                CGEventSetDoubleValueField(
                    event,
                    K_CG_SCROLL_WHEEL_EVENT_FIXED_PT_DELTA_AXIS_2,
                    -fp_x,
                );
            }
        }
        return event;
    }

    // 6. Enqueue smooth pixel impulse into the 120Hz physics animator
    let (lock, cvar) = engine_pair();
    if let Ok(mut st) = lock.try_lock() {
        let now = now_ms();
        let dt = (now - st.last_notch_at_ms).abs();
        st.last_notch_at_ms = now;

        if dt < 95 {
            st.fast_streak = (st.fast_streak + 1).min(8);
        } else {
            st.fast_streak = 0;
        }

        let max_accel = (cfg.scroll_acceleration as f64).clamp(1.0, 3.0);
        let accel_factor = if st.fast_streak == 0 {
            1.0
        } else {
            1.0 + (max_accel - 1.0) * (st.fast_streak as f64 / 6.0).min(1.0)
        };

        let step_px = (cfg.scroll_step_px.clamp(15, 120) as f64) * accel_factor;
        let impulse_y = (dy_lines.clamp(-6, 6) as f64) * step_px;
        let impulse_x = (dx_lines.clamp(-6, 6) as f64) * step_px;

        // Instant stop & reverse when changing scroll direction
        if impulse_y != 0.0 && st.remaining_y * impulse_y < 0.0 {
            st.remaining_y = 0.0;
            st.subpixel_y = 0.0;
        }
        if impulse_x != 0.0 && st.remaining_x * impulse_x < 0.0 {
            st.remaining_x = 0.0;
            st.subpixel_x = 0.0;
        }

        st.remaining_y = (st.remaining_y + impulse_y).clamp(-2400.0, 2400.0);
        st.remaining_x = (st.remaining_x + impulse_x).clamp(-2400.0, 2400.0);
        st.duration_ms = cfg.scroll_duration_ms.clamp(80, 600);
        st.flags = flags;
        cvar.notify_one();
        return std::ptr::null_mut();
    }

    event
}
