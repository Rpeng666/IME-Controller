use windows::Win32::UI::WindowsAndMessaging::WM_USER;

// Constants
pub const IDI_ICON1: u16 = 101;
pub const IDI_ICON_ACTIVE: u16 = 102;
pub const IDM_EXIT: u32 = 1001;
pub const IDM_AUTOSTART: u32 = 1003;
pub const IDM_MANAGE_EXCLUDES: u32 = 1004;
pub const IDM_IME_CHINESE: u32 = 1005;
pub const IDM_IME_ENGLISH: u32 = 1006;
pub const IDM_MASTER_SWITCH: u32 = 1007;
pub const IDM_TOGGLE_NOTIFICATIONS: u32 = 1008;
pub const IDM_CONFIGURE_HOTKEYS: u32 = 1009;
pub const IDM_OPEN_CONFIG_DIR: u32 = 1010;
pub const IDM_RELOAD_CONFIG: u32 = 1011;
pub const NOTIFYICONMESSAGE: u32 = WM_USER + 100;
pub const HOT_KEY_TOGGLE_ID: i32 = 1; // 切换总开关
pub const HOT_KEY_SWITCH_MODE_ID: i32 = 2; // 切换中英文模式
pub const TIMER_ID_IME_CHECK: usize = 1; // 定时器ID，用于定时检测输入法状态
