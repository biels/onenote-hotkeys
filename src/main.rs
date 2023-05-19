use winapi::um::winuser::{WH_KEYBOARD_LL, KBDLLHOOKSTRUCT, RegisterClassExW, VK_MENU, VK_RIGHT, VK_DOWN, VK_RETURN, VK_LEFT, VK_ESCAPE, VK_LMENU, VK_OEM_PERIOD, VK_LSHIFT, VK_RSHIFT, VK_RCONTROL, VK_SHIFT, VK_OEM_MINUS, VK_OEM_COMMA};
use std::ptr;
use winapi::um::winuser::{SetCursorPos, mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP};
use winapi::um::winuser::{GetCursorPos, ScreenToClient};
use std::mem;
use std::os::windows::ffi::OsStringExt;
use std::ffi::OsString;
use std::thread::sleep;
use std::time::Duration;
use winapi::shared::minwindef::{BYTE, DWORD, LPARAM};
use winapi::um::{winuser};

extern crate winapi;

use winapi::shared::windef::{HBRUSH, HWND, RECT};
use winapi::um::winuser::{CreateWindowExW, FindWindowW, GetClientRect, GetDC, MessageBoxW, ReleaseDC, RegisterClassW, CW_USEDEFAULT, MB_OK, MSG, WM_PAINT, WS_EX_TOPMOST, WS_OVERLAPPED, WS_SYSMENU};


static mut tool: Tools = Tools::Arrow;
static mut last_timestamp: DWORD = 0;
static mut last_shift_timestamp: DWORD = 0;
static mut last_shift_timestamp_prev: DWORD = 0;
static default_dw_amount: i32 = 120 * 14;
static mut zoomed_in: bool = false;
static mut zoom_manual: bool = false;
static mut zoom_manual_value: bool = false;

static mut base_tool: Tools = Tools::Arrow;

unsafe extern "system" fn keyboard_proc(n_code: i32, wparam: usize, lparam: isize) -> isize {
    if n_code >= 0 && wparam as u32 == 0x100 {
        let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
        println!("Key pressed: 0x{:x}", kbd.vkCode);

        if kbd.vkCode == VK_ESCAPE as DWORD {
            // escape
            tool = Tools::Arrow;
        }
        if kbd.vkCode == VK_OEM_PERIOD as DWORD {
            // Period, eraser
            if tool == Tools::Eraser {
                activate_tool(Tools::Pen);
                return 1;
            }
            if tool != Tools::Arrow {
                activate_tool(Tools::Eraser);
                return 1;
            }
        }
        if kbd.vkCode == VK_OEM_COMMA as DWORD {
            // Comma, small arrow to drag
            if tool == Tools::ArrowSm {
                activate_tool(Tools::Pen);
                return 1;
            }
            if tool != Tools::ArrowSm {
                activate_tool(Tools::ArrowSm);
                return 1;
            }
        }
        if kbd.vkCode == VK_LEFT as DWORD {
            // Left arrow
            if tool != Tools::Arrow {
                issue_undo(false);
                return 1;
            }
        }
        let enable_shift_zoom = true;
        if (kbd.vkCode == VK_RIGHT as DWORD) && enable_shift_zoom.clone() {
            // Right arrow
            if tool != Tools::Arrow {
                issue_undo(true);
                return 1;
            }
        }
        if (kbd.vkCode == VK_LSHIFT as DWORD) && enable_shift_zoom.clone() {
            // Left shift
            let delta = kbd.clone().time - &last_shift_timestamp;
            let delta_prev = kbd.clone().time - &last_shift_timestamp_prev;
            last_shift_timestamp_prev = last_shift_timestamp.clone();
            last_shift_timestamp = kbd.clone().time;
            if delta_prev < 400 {
                // println!("Triple shift");
                // set_zoom_to_default();
                // return 1;
            } else if delta < 200 {
                zoom_manual_value = !zoom_manual_value.clone();
                ensure_zoom(zoom_manual_value.clone());
                return 1;
            }
            tool = Tools::Arrow;
        }
        if kbd.vkCode == VK_RSHIFT as DWORD {
            // Right shift
            let delta = kbd.clone().time - &last_shift_timestamp;
            last_shift_timestamp = kbd.clone().time;
            if delta < 200 {
                zoom_manual = !zoom_manual.clone();
                if !zoom_manual.clone() {
                    ensure_zoom(is_tool_zoomed_in(&tool));
                }
                let mut str = format!("{}", if zoom_manual { "manual" } else { "auto" });
                // pad with spaces
                str.push_str("  ".repeat(60 - str.len()).as_str());
                show_overlay_toast(str.as_str());
                return 1;
            }
            tool = Tools::Arrow;
        }
        if kbd.vkCode == VK_OEM_MINUS as DWORD {
            if tool != Tools::Arrow {
                activate_tool(Tools::Select);
                return 1;
            }
        }
        if kbd.vkCode == VK_RCONTROL as DWORD {
            // activate_tool(get_next_tool());
            let delta = kbd.clone().time - &last_timestamp;
            if delta < 200 {
                // let shift = kbd.clone().flags & VK_SHIFT as DWORD == VK_SHIFT as DWORD;
                activate_tool(Tools::SelectLg);
            } else {
                activate_tool(get_next_tool());
            }
            last_timestamp = kbd.clone().time;
            // println!("Delta: {}", delta);
            // return 1;
        }
    }
    return winuser::CallNextHookEx(ptr::null_mut(), n_code, wparam.clone(), lparam.clone());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tools {
    Arrow,
    ArrowSm,
    Select,
    Eraser,
    Pen,
    SelectLg,
}

fn activate_tool(t: Tools) {
    println!("Activating tool: {:?}", t);
    let pos = get_cursor_pos_absolute().unwrap();
    // println!("Cursor position: ({}, {})", pos.0, pos.1);
    match t {
        Tools::Arrow | Tools::ArrowSm => {
            // send keypress esc
            send_keypress(0x1b);
        }
        Tools::Select | Tools::SelectLg => {
            click(147, 78);
            move_cursor(pos);
        }
        Tools::Eraser => {
            click(228, 78);
            move_cursor(pos);
        }
        Tools::Pen => {
            click(267, 78);
            move_cursor(pos);
        }
    }
    if !unsafe { zoom_manual.clone() } {
        ensure_zoom(is_tool_zoomed_in(&t));
    }
    unsafe {
        if is_tool_base(&t) {
            base_tool = tool;
        }
        tool = t;

    }
}

// Not working
fn set_zoom_to_default() {
    unsafe {
        // The following keypresses:
        // Alt ArrowRight ArrowDown ArrowRight (x5) Enter Alt ArrowLeft Esc
        // Vec using key enum values
        sleep(Duration::from_millis(500));

        let key_presses = vec![
            VK_LMENU, // Lmenu not working
            // VK_RIGHT, VK_DOWN, VK_RIGHT, VK_RIGHT, VK_RIGHT, VK_RIGHT, VK_RIGHT, VK_RETURN, VK_MENU, VK_LEFT, VK_ESCAPE
        ];
        for key in key_presses {
            println!("Sending keypress: 0x{:x}", key);
            send_keypress(key as u8);
            sleep(Duration::from_millis(50));
        }
    }
}

fn is_tool_zoomed_in(t: &Tools) -> bool {
    match t {
        Tools::Arrow => false,
        Tools::ArrowSm => true,
        Tools::SelectLg => false,
        Tools::Select => true,
        Tools::Eraser => true,
        Tools::Pen => true,
    }
}

fn is_tool_base(t: &Tools) -> bool {
    match t {
        Tools::Arrow => false,
        Tools::ArrowSm => false,
        Tools::SelectLg => true,
        Tools::Select => true,
        Tools::Eraser => false,
        Tools::Pen => true,
    }
}

fn ensure_zoom(new_value: bool) {
    println!("Zoom: {}", new_value);
    unsafe {
        if zoomed_in != new_value {
            let m = if new_value { 1 } else { -1 };
            send_ctrl_scroll_zoom((default_dw_amount.clone() * m) as DWORD);
            zoomed_in = new_value.clone();
            zoom_manual_value = new_value.clone();
        }
    }
}

fn move_cursor(pos: (i32, i32)) {
    unsafe {
        SetCursorPos(pos.0, pos.1);
    }
}

fn get_next_tool() -> Tools {
    unsafe {
        match tool {
            Tools::Arrow => Tools::Pen,
            Tools::Pen => Tools::Arrow,
            Tools::Eraser => Tools::Pen,
            Tools::Select => Tools::Pen,
            Tools::SelectLg => Tools::Arrow,
            Tools::ArrowSm => Tools::Pen,
        }
    }
}

unsafe extern "system" fn mouse_proc(n_code: i32, wparam: usize, lparam: isize) -> isize {
    // println!("Mouse event {} {} {}", n_code, wparam, lparam);
    let filter = vec![
        winuser::WM_LBUTTONDBLCLK,
        winuser::WM_LBUTTONDOWN,
        winuser::WM_LBUTTONUP,
        winuser::WM_MBUTTONDBLCLK,
        winuser::WM_MBUTTONDOWN,
        winuser::WM_MBUTTONUP,
        winuser::WM_MOUSEACTIVATE,
        winuser::WM_MOUSEHOVER,
        winuser::WM_MOUSEHWHEEL,
        winuser::WM_MOUSELEAVE,
        winuser::WM_MOUSEMOVE,
        winuser::WM_MOUSEWHEEL,
        winuser::WM_NCHITTEST,
        winuser::WM_NCLBUTTONDBLCLK,
        winuser::WM_NCLBUTTONDOWN,
        winuser::WM_NCLBUTTONUP,
    ];
    if n_code >= 0 && filter.contains(&(wparam as u32)) {
        println!("Event: {}", wparam);
    }
    return winapi::um::winuser::CallNextHookEx(ptr::null_mut(), n_code, wparam.clone(), lparam.clone());
}

fn click(x: i32, y: i32) {
    // Set cursor position
    unsafe { SetCursorPos(x, y) };

    // Simulate left click
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
}

fn send_keypress(key: u8) {
    unsafe {
        winuser::keybd_event(key.clone(), 0xb8, winuser::KEYEVENTF_EXTENDEDKEY, 0);
        winuser::keybd_event(key.clone(), 0xb8, winuser::KEYEVENTF_EXTENDEDKEY | winuser::KEYEVENTF_KEYUP, 0);
    }
}

fn send_ctrl_scroll_zoom(dw_amount: DWORD) {
    unsafe {
        winapi::um::winuser::keybd_event(winapi::um::winuser::VK_LCONTROL as BYTE, 0, 0, 0);
        winapi::um::winuser::mouse_event(winapi::um::winuser::MOUSEEVENTF_WHEEL, 0, 0, dw_amount, 0);
        winapi::um::winuser::keybd_event(winapi::um::winuser::VK_LCONTROL as BYTE, 0, winapi::um::winuser::KEYEVENTF_KEYUP, 0);
    }
}

fn issue_undo(redo: bool) {
    println!("Issuing {}", if redo { "redo" } else { "undo" });
    let key = if redo { 0x59 } else { 0x5A };
    unsafe {
        winapi::um::winuser::keybd_event(winapi::um::winuser::VK_LCONTROL as BYTE, 0, winuser::KEYEVENTF_EXTENDEDKEY, 0);
        winapi::um::winuser::keybd_event(key.clone(), 0, 0, 0);
        winapi::um::winuser::keybd_event(key.clone(), 0, winuser::KEYEVENTF_KEYUP, 0);
        winapi::um::winuser::keybd_event(winapi::um::winuser::VK_LCONTROL as BYTE, 0, winuser::KEYEVENTF_EXTENDEDKEY | winuser::KEYEVENTF_KEYUP, 0);
    }
}


fn get_cursor_pos(hwnd: HWND) -> Option<(i32, i32)> {
    let mut point = winapi::shared::windef::POINT { x: 0, y: 0 };

    unsafe {
        if GetCursorPos(&mut point) == 0 {
            return None;
        }

        if ScreenToClient(hwnd, &mut point) == 0 {
            return None;
        }
    }

    Some((point.x, point.y))
}

fn get_cursor_pos_absolute() -> Option<(i32, i32)> {
    let mut point = winapi::shared::windef::POINT { x: 0, y: 0 };

    unsafe {
        if GetCursorPos(&mut point) == 0 {
            return None;
        }
    }

    Some((point.x, point.y))
}


fn overlay_toast_register() {}

fn show_overlay_toast(window_text: &str) {
    unsafe {
        let class_name = "MyOverlayClass\0".encode_utf16().collect::<Vec<u16>>();
        let window_text = window_text.encode_utf16().collect::<Vec<u16>>();

        let wnd_class = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null_mut());
        let wcex = winapi::um::winuser::WNDCLASSEXW {
            cbSize: std::mem::size_of::<winapi::um::winuser::WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(winapi::um::winuser::DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: wnd_class,
            hIcon: winapi::um::winuser::LoadIconW(std::ptr::null_mut(), winapi::um::winuser::IDI_APPLICATION),
            hCursor: winapi::um::winuser::LoadCursorW(std::ptr::null_mut(), winapi::um::winuser::IDC_ARROW),
            hbrBackground: winapi::um::winuser::COLOR_WINDOW as HBRUSH,
            lpszMenuName: std::ptr::null_mut(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: winapi::um::winuser::LoadIconW(std::ptr::null_mut(), winapi::um::winuser::IDI_APPLICATION),
        };

        RegisterClassExW(&wcex);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            window_text.as_ptr() as *const u16,
            WS_OVERLAPPED | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            150,
            90,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            wnd_class,
            std::ptr::null_mut(),
        );
        // disable_window_animation(hwnd);

        let mut rect = winapi::shared::windef::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetClientRect(hwnd, &mut rect);

        let screen_width = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN);
        let screen_height = winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN);

        let window_width = rect.right - rect.left;
        let window_height = rect.bottom - rect.top;

        let x = (screen_width - window_width) / 2;
        let y = (screen_height - window_height) / 2;

        winapi::um::winuser::SetWindowPos(hwnd, std::ptr::null_mut(), x, y, 0, 0, winapi::um::winuser::SWP_NOSIZE | winapi::um::winuser::SWP_NOZORDER);

        winapi::um::winuser::ShowWindow(hwnd, winapi::um::winuser::SW_SHOW);

        std::thread::sleep(std::time::Duration::from_millis(400));

        winapi::um::winuser::DestroyWindow(hwnd);
    }
}

use winapi::um::winuser::{SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TOOLWINDOW, WS_EX_LAYERED};

fn disable_window_animation(hwnd: HWND) {
    unsafe {
        let extended_style = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, WS_EX_TOOLWINDOW as isize);
        // Add WS_EX_LAYERED style to make the window layered
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, extended_style | WS_EX_LAYERED as isize);
    }
}

fn print_instructions() {
    let instructions = r"
    Hotkeys:
      - RCtrl: Toggle between Arrow and Pen
      - RCtrl (x2): Draw selection (zoomed out)
      - Dash (-): Draw selection (zoomed in) (except in Arrow mode)
      - Dot: Toggle between Pen and Eraser, in any mode except Arrow
      - LArrow and RArrow: Undo and Redo (except in Arrow mode)
      - LShift (x2): Manual zoom in and out
      - RShift (x2): Toggle manual / automatic zoom
      - Esc: Arrow mode (reset to known state)
";
    println!("{}", instructions);
}


fn main() {
    print_instructions();
    unsafe {
        // List all windows with titles and handles
        // let hwnd = unsafe { winapi::um::winuser::FindWindowA(ptr::null_mut(), "One Note\0".as_ptr() as *const i8) };
        match get_cursor_pos_absolute() {
            Some((x, y)) => println!("Cursor position: ({}, {})", x, y),
            None => println!("Error getting cursor position"),
        }

        let hook = winapi::um::winuser::SetWindowsHookExA(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            winapi::um::libloaderapi::GetModuleHandleA(ptr::null()),
            0,
        );
        if hook.is_null() {
            println!("Failed to set hook");
            return;
        }
        // Set hook for all mouse events
        // let mouse_hook = winapi::um::winuser::SetWindowsHookExA(
        //     winapi::um::winuser::WH_MOUSE_LL,
        //     Some(mouse_proc),
        //     winapi::um::libloaderapi::GetModuleHandleA(ptr::null()),
        //     0,
        // );
        println!("Hook set, waiting for events...");
        loop {
            let mut msg = MSG {
                hwnd: ptr::null_mut(),
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: winapi::shared::windef::POINT { x: 0, y: 0 },
            };
            let res = winapi::um::winuser::GetMessageW(&mut msg, ptr::null_mut(), 0, 0);
            if res == -1 {
                println!("Error getting message");
                break;
            } else if res == 0 {
                println!("Message loop exited");
                break;
            } else {
                winapi::um::winuser::TranslateMessage(&msg);
                winapi::um::winuser::DispatchMessageW(&msg);
            }
            // listen for q
            if msg.message == 0x51 {
                println!("Q pressed, exiting");
                break;
            }
        }
        winapi::um::winuser::UnhookWindowsHookEx(hook);
    }
}
