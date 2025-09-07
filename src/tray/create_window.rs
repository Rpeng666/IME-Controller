use crate::constants;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, LoadCursorW, LoadIconW, RegisterClassW, CW_USEDEFAULT, IDC_ARROW,
    WNDCLASS_STYLES, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::Win32::UI::WindowsAndMessaging::{CS_HREDRAW, CS_VREDRAW, WNDCLASSW};
use windows::{
    core::*,
    Win32::{Foundation::HWND, System::LibraryLoader::GetModuleHandleW},
};

// 创建隐藏窗口
pub fn create_window(
    window_proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
) -> Result<HWND> {
    let instance = unsafe { GetModuleHandleW(None)? };
    let window_class = w!("IME_CONTROL_WINDOW");
    let wc = WNDCLASSW {
        style: WNDCLASS_STYLES(CS_HREDRAW.0 | CS_VREDRAW.0),
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: unsafe { LoadIconW(instance, PCWSTR(constants::IDI_ICON1 as *const u16))? },
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: window_class,
    };

    unsafe {
        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return Err(Error::from_win32());
        }

        // 创建一个隐藏的窗口
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW, // 使用工具窗口样式
            window_class,
            w!("输入法控制"),
            WS_OVERLAPPED, // 不使用 WS_VISIBLE
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1, // 最小化窗口大小
            1,
            None,
            None,
            instance,
            None,
        );

        if hwnd.0 == 0 {
            Err(Error::from_win32())
        } else {
            Ok(hwnd)
        }
    }
}
