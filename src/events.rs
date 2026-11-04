use std::sync::{Mutex, OnceLock};

pub const MAX_SCREENSHOT_SLOTS: usize = 4;

#[derive(Clone, Default)]
pub struct ScreenshotSlotData {
    pub display_id: u32,
    pub display_name: String,
    pub png_bytes: Vec<u8>,
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    pub preselect_fullscreen: bool,
}

pub struct ActiveScreenshotSession {
    pub slots: Vec<ScreenshotSlotData>,
    pub capture_seq: u64,
    pub expected_slots: usize,
    pub ready_slots: Vec<bool>,
    pub revealed: bool,
    pub target_slot: usize,
    pub active_slot: Option<usize>,
    pub prev_frontmost_pid: i32,
    pub capturing: bool,
}

static SCREENSHOT_SESSION: OnceLock<Mutex<ActiveScreenshotSession>> = OnceLock::new();

pub fn screenshot_session() -> &'static Mutex<ActiveScreenshotSession> {
    SCREENSHOT_SESSION.get_or_init(|| {
        Mutex::new(ActiveScreenshotSession {
            slots: Vec::new(),
            capture_seq: 0,
            expected_slots: 0,
            ready_slots: Vec::new(),
            revealed: false,
            target_slot: 0,
            active_slot: None,
            prev_frontmost_pid: 0,
            capturing: false,
        })
    })
}

#[derive(Debug, Clone)]
pub enum AppUserEvent {
    ToggleMenuBarPopover,
    SetMiniExpanded(bool),
    SetMiniHeight(f64),
    UpdateMenuBarTitle(String),
    SyncMenuBarConfig,
    SetAppTheme(String),
    SetAppLang(String),
    ToggleClipboardDrawer,
    HideClipboardDrawer,
    ToggleAltTabOverlay,
    CommitAltTabSelection,
    HideAltTabOverlay,
    TriggerScreenshotOverlay,
    TriggerFullscreenScreenshot {
        display_id: u32,
        instant_copy: bool,
    },
    PresentScreenshotSession {
        seq: u64,
        fullscreen_picker_mode: bool,
    },
    RevealScreenshotSlot {
        slot: usize,
        seq: u64,
    },
    ScreenshotCaptureFailed,
    FullscreenInstantCopied {
        display_name: String,
    },
    SetActiveScreenshotSlot(usize),
    HideScreenshotOverlay,
    SnapOwnMainWindow(String),
    HideAllOverlays,
    ShowLauncher,
    ToggleLauncher,
    ShowAboutDialog,
    OpenDashboardTab(String),
    HideLauncher,
    QuitApp,
}

