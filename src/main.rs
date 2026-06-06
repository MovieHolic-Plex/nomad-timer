#![windows_subsystem = "windows"]
#![allow(unsafe_op_in_unsafe_fn)]

use breaktime::cat_assets::{CatBitmap, bitmap_for_variant};
use breaktime::api_contract::BroadcastMessage;
use breaktime::timer::{TimerState, WorkPhase, state_from_epoch_ms};
use breaktime::widget::{cat_variant, latest_chat_line, preset_at_x, visible_presets};
use breaktime::{
    client::ServerClock, client::default_api_base, client::fetch_recent_messages,
    client::fetch_server_clock, client::post_preset_broadcast,
};
use std::env;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, CreatePen, CreateSolidBrush, DIB_RGB_COLORS,
    DeleteObject, DrawTextW, EndPaint, FillRect, GetMonitorInfoW, GetStockObject, HBRUSH, HDC,
    HGDIOBJ, InvalidateRect, LOGFONTW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
    NULL_BRUSH, PAINTSTRUCT, PS_SOLID, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
    StretchDIBits, TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetWindowLongPtrW, HICON, HTCAPTION, IDC_ARROW, IDI_APPLICATION, KillTimer, LoadCursorW,
    LoadIconW, MENU_ITEM_FLAGS, MSG, PostQuitMessage, RegisterClassW,
    SW_HIDE, SW_SHOWNOACTIVATE, SendMessageW, SetForegroundWindow,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCLBUTTONDOWN, WM_PAINT,
    WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_TOPMOST, WS_POPUP, HWND_TOPMOST, SWP_SHOWWINDOW,
};
use windows::core::{PCWSTR, w};

const WINDOW_CLASS: PCWSTR = w!("BreaktimeWidgetWindow");
const TRAY_UID: u32 = 7;
const TIMER_ID: usize = 1;
const WM_TRAY: u32 = WM_APP + 1;
const MENU_TOGGLE: usize = 1001;
const MENU_EXIT: usize = 1002;
const WIDTH: i32 = 320;
const HEIGHT: i32 = 196;
const COLLAPSED_HEIGHT: i32 = 48;
const PRESET_TOP: i32 = 144;
const PRESET_HEIGHT: i32 = 30;
const PRESET_LEFT: i32 = 14;
const PRESET_RIGHT: i32 = WIDTH - 10;
const CLOSE_LEFT: i32 = WIDTH - 36;
const TOGGLE_LEFT: i32 = WIDTH - 68;
const TOGGLE_BOTTOM: i32 = 36;

struct AppState {
    timer: TimerState,
    last_notified_phase: WorkPhase,
    visible: bool,
    collapsed: bool,
    last_broadcast: String,
    latest_preset_id: Option<String>,
    server_clock: Option<ServerClock>,
    schedule_rx: Receiver<ServerClock>,
    messages_rx: Receiver<Vec<BroadcastMessage>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WidgetHitAction {
    Close,
    ToggleCollapse,
    PresetBroadcast(&'static str),
    Drag,
}

impl AppState {
    fn new() -> Self {
        let schedule_rx = spawn_schedule_poller();
        let messages_rx = spawn_message_poller();
        let timer = current_timer_state();
        Self {
            timer,
            last_notified_phase: timer.phase,
            visible: true,
            collapsed: false,
            last_broadcast: breaktime::widget::DEFAULT_CHAT_LINE.to_owned(),
            latest_preset_id: None,
            server_clock: None,
            schedule_rx,
            messages_rx,
        }
    }
}

fn main() -> windows::core::Result<()> {
    unsafe {
        debug_log("start");
        let instance = GetModuleHandleW(None)?;
        debug_log("module");
        let cursor = LoadCursorW(None, IDC_ARROW)?;
        let background = HBRUSH(GetStockObject(WHITE_BRUSH).0);

        let wc = WNDCLASSW {
            hCursor: cursor,
            hInstance: instance.into(),
            lpszClassName: WINDOW_CLASS,
            style: CS_HREDRAW | CS_VREDRAW,
            hbrBackground: background,
            lpfnWndProc: Some(wnd_proc),
            ..Default::default()
        };
        RegisterClassW(&wc);
        debug_log("registered");

        let state = Box::new(AppState::new());
        let state_ptr = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            WINDOW_CLASS,
            w!("쉬는시간"),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIDTH,
            HEIGHT,
            None,
            None,
            Some(instance.into()),
            Some(state_ptr.cast::<c_void>()),
        )?;
        debug_log(&format!("window-created {:?}", hwnd.0));

        set_widget_position(hwnd, HEIGHT);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        set_widget_position(hwnd, HEIGHT);
        let _ = SetForegroundWindow(hwnd);
        let _ = InvalidateRect(Some(hwnd), None, true);
        debug_log("shown");

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                let createstruct =
                    lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
                let state_ptr = (*createstruct).lpCreateParams as *mut AppState;
                SetWindowLongPtrW(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    state_ptr as isize,
                );
                add_tray_icon(hwnd);
                let _ = SetTimer(Some(hwnd), TIMER_ID, 1000, None);
                LRESULT(0)
            }
            WM_TIMER => {
                if wparam.0 == TIMER_ID
                    && let Some(state) = state_mut(hwnd)
                {
                    while let Ok(clock) = state.schedule_rx.try_recv() {
                        state.server_clock = Some(clock);
                    }
                    while let Ok(messages) = state.messages_rx.try_recv() {
                        state.latest_preset_id =
                            messages.first().map(|message| message.preset_id.clone());
                        state.last_broadcast = latest_chat_line(&messages);
                    }
                    let local_now_ms = current_epoch_ms();
                    let next = state
                        .server_clock
                        .map(|clock| clock.timer_state(local_now_ms))
                        .unwrap_or_else(|| state_from_epoch_ms(local_now_ms));
                    if next.phase != state.timer.phase && next.phase != state.last_notified_phase {
                        show_phase_notification(hwnd, next.phase);
                        state.last_notified_phase = next.phase;
                    }
                    state.timer = next;
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }
            WM_PAINT => {
                if let Some(state) = state_mut(hwnd) {
                    debug_log("paint");
                    paint(hwnd, state);
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = state_mut(hwnd) {
                    let x = signed_loword(lparam.0);
                    let y = signed_loword(lparam.0 >> 16);
                    if should_drag(state.collapsed, x, y) {
                        let _ = ReleaseCapture();
                        let _ = SendMessageW(
                            hwnd,
                            WM_NCLBUTTONDOWN,
                            Some(WPARAM(HTCAPTION as usize)),
                            Some(LPARAM(0)),
                        );
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if let Some(state) = state_mut(hwnd) {
                    let x = signed_loword(lparam.0);
                    let y = signed_loword(lparam.0 >> 16);
                    match widget_hit_action(state.collapsed, x, y) {
                        WidgetHitAction::Close => {
                            let _ = DestroyWindow(hwnd);
                        }
                        WidgetHitAction::ToggleCollapse => {
                            state.collapsed = !state.collapsed;
                            let height = if state.collapsed {
                                COLLAPSED_HEIGHT
                            } else {
                                HEIGHT
                            };
                            set_widget_position(hwnd, height);
                            let _ = SetWindowPos(
                                hwnd,
                                Some(HWND_TOPMOST),
                                0,
                                0,
                                WIDTH,
                                height,
                                SWP_SHOWWINDOW,
                            );
                            let _ = InvalidateRect(Some(hwnd), None, true);
                        }
                        WidgetHitAction::PresetBroadcast(preset_id) => {
                            state.last_broadcast = queue_preset_broadcast(preset_id);
                            let _ = InvalidateRect(Some(hwnd), None, true);
                        }
                        WidgetHitAction::Drag => {}
                    }
                }
                LRESULT(0)
            }
            WM_TRAY => match lparam.0 as u32 {
                WM_LBUTTONUP => {
                    toggle_visible(hwnd);
                    LRESULT(0)
                }
                WM_RBUTTONUP => {
                    show_tray_menu(hwnd);
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            },
            WM_COMMAND => {
                match wparam.0 & 0xffff {
                    MENU_TOGGLE => toggle_visible(hwnd),
                    MENU_EXIT => {
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER_ID);
                delete_tray_icon(hwnd);
                if let Some(ptr) = take_state_ptr(hwnd) {
                    drop(Box::from_raw(ptr));
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn current_timer_state() -> TimerState {
    state_from_epoch_ms(current_epoch_ms())
}

fn debug_log(message: &str) {
    let Ok(path) = env::var("BREAKTIME_DEBUG_LOG") else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

fn spawn_schedule_poller() -> Receiver<ServerClock> {
    let (tx, rx) = channel();
    let server = server_base();
    let _ = thread::Builder::new()
        .name("breaktime-schedule".to_owned())
        .spawn(move || {
            loop {
                let local_sample_ms = current_epoch_ms();
                if let Ok(clock) = fetch_server_clock(&server, local_sample_ms) {
                    let _ = tx.send(clock);
                }
                thread::sleep(Duration::from_secs(10));
            }
        });
    rx
}

fn spawn_message_poller() -> Receiver<Vec<BroadcastMessage>> {
    let (tx, rx) = channel();
    let server = server_base();
    let _ = thread::Builder::new()
        .name("breaktime-messages".to_owned())
        .spawn(move || {
            loop {
                if let Ok(messages) = fetch_recent_messages(&server) {
                    let _ = tx.send(messages);
                }
                thread::sleep(Duration::from_secs(3));
            }
        });
    rx
}

fn server_base() -> String {
    env::var("BREAKTIME_SERVER").unwrap_or_else(|_| default_api_base().to_owned())
}

fn current_epoch_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

fn signed_loword(value: isize) -> i32 {
    (value as u16) as i16 as i32
}

fn queue_preset_broadcast(preset_id: &'static str) -> String {
    let preset_label = breaktime::presets::find_preset(preset_id)
        .map(|preset| preset.label)
        .unwrap_or("알림");
    let _ = thread::Builder::new()
        .name("breaktime-broadcast".to_owned())
        .spawn(move || {
            let _ = post_preset_broadcast(&server_base(), preset_id, "windows-widget");
        });
    format!("모두에게 보냈어요: {preset_label}")
}

fn should_drag(collapsed: bool, x: i32, y: i32) -> bool {
    matches!(widget_hit_action(collapsed, x, y), WidgetHitAction::Drag)
}

fn widget_hit_action(collapsed: bool, x: i32, y: i32) -> WidgetHitAction {
    if y <= TOGGLE_BOTTOM && x >= CLOSE_LEFT {
        return WidgetHitAction::Close;
    }
    if y <= TOGGLE_BOTTOM && x >= TOGGLE_LEFT {
        return WidgetHitAction::ToggleCollapse;
    }
    if !collapsed
        && (PRESET_TOP..=PRESET_TOP + PRESET_HEIGHT).contains(&y)
        && (PRESET_LEFT..=PRESET_RIGHT).contains(&x)
    {
        if let Some(preset_id) = preset_at_x(x - PRESET_LEFT, PRESET_RIGHT - PRESET_LEFT) {
            return WidgetHitAction::PresetBroadcast(preset_id);
        }
    }
    WidgetHitAction::Drag
}

#[cfg(test)]
mod widget_input_tests {
    use super::*;

    #[test]
    fn widget_hit_action_selects_close_toggle_and_preset_when_pointer_is_in_control_zones() {
        // Given: the expanded widget exposes top-right window controls and preset buttons.
        let close_x = WIDTH - 12;
        let toggle_x = WIDTH - 42;
        let preset_x = PRESET_LEFT + 2;

        // When: clicks land in each interactive zone.
        let close_action = widget_hit_action(false, close_x, 18);
        let toggle_action = widget_hit_action(false, toggle_x, 18);
        let preset_action = widget_hit_action(false, preset_x, PRESET_TOP + 4);

        // Then: the widget routes each click to the intended behavior.
        assert_eq!(close_action, WidgetHitAction::Close);
        assert_eq!(toggle_action, WidgetHitAction::ToggleCollapse);
        assert_eq!(
            preset_action,
            WidgetHitAction::PresetBroadcast("rest-start")
        );
    }
}

unsafe fn state_mut(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA)
        as *mut AppState;
    ptr.as_mut()
}

unsafe fn take_state_ptr(hwnd: HWND) -> Option<*mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA)
        as *mut AppState;
    SetWindowLongPtrW(
        hwnd,
        windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
        0,
    );
    if ptr.is_null() { None } else { Some(ptr) }
}

unsafe fn set_widget_position(hwnd: HWND, height: i32) {
    let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    let work = info.rcWork;
    let x = work.right - WIDTH - 18;
    let y = work.bottom - height - 18;
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_TOPMOST),
        x,
        y,
        WIDTH,
        height,
        SWP_SHOWWINDOW,
    );
}

unsafe fn toggle_visible(hwnd: HWND) {
    if let Some(state) = state_mut(hwnd) {
        state.visible = !state.visible;
        let _ = ShowWindow(
            hwnd,
            if state.visible {
                SW_SHOWNOACTIVATE
            } else {
                SW_HIDE
            },
        );
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0), MENU_TOGGLE, w!("보이기 / 숨기기"));
    let _ = AppendMenuW(menu, MENU_ITEM_FLAGS(0), MENU_EXIT, w!("종료"));

    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, Some(0), hwnd, None);
    let _ = DestroyMenu(menu);
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let mut data = tray_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap_or(HICON::default());
    write_wide(&mut data.szTip, "쉬는시간");
    let _ = Shell_NotifyIconW(NIM_ADD, &data);
}

unsafe fn delete_tray_icon(hwnd: HWND) {
    let data = tray_data(hwnd);
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}

unsafe fn show_phase_notification(hwnd: HWND, phase: WorkPhase) {
    let mut data = tray_data(hwnd);
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = NIIF_INFO;
    write_wide(&mut data.szInfoTitle, "쉬는시간");
    match phase {
        WorkPhase::Work => write_wide(&mut data.szInfo, "다시 일할 시간이에요"),
        WorkPhase::Break => write_wide(&mut data.szInfo, "쉬는시간이에요"),
    }
    let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
}

fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_UID,
        ..unsafe { zeroed() }
    }
}

fn write_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    for (slot, unit) in target.iter_mut().zip(text.encode_utf16()) {
        *slot = unit;
    }
}

unsafe fn paint(hwnd: HWND, state: &AppState) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let height = if state.collapsed {
        COLLAPSED_HEIGHT
    } else {
        HEIGHT
    };

    let bg = match state.timer.phase {
        WorkPhase::Work => rgb(255, 244, 214),
        WorkPhase::Break => rgb(221, 245, 235),
    };
    let border = match state.timer.phase {
        WorkPhase::Work => rgb(237, 172, 73),
        WorkPhase::Break => rgb(88, 176, 138),
    };
    let text = rgb(55, 50, 44);

    let brush = CreateSolidBrush(bg);
    FillRect(
        hdc,
        &RECT {
            left: 0,
            top: 0,
            right: WIDTH,
            bottom: height,
        },
        brush,
    );
    let _ = DeleteObject(brush.into());

    let pen = CreatePen(PS_SOLID, 2, border);
    let old_pen = SelectObject(hdc, pen.into());
    let old_brush = SelectObject(hdc, HGDIOBJ(GetStockObject(NULL_BRUSH).0));
    let _ = windows::Win32::Graphics::Gdi::RoundRect(hdc, 1, 1, WIDTH - 1, height - 1, 18, 18);
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    let _ = DeleteObject(pen.into());

    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, text);

    if state.collapsed {
        draw_text(
            hdc,
            &format!("{}  {}", state.timer.title(), state.timer.remaining_label()),
            TextBox::new(14, 11, WIDTH - 78, 40, 18, 700),
        );
        draw_window_controls(hdc);
    } else {
        let cat = cat_variant(state.timer, state.latest_preset_id.as_deref());
        draw_cat_bitmap(
            hdc,
            bitmap_for_variant(cat, state.latest_preset_id.as_deref()),
        );
        draw_text(
            hdc,
            state.timer.title(),
            TextBox::new(96, 17, WIDTH - 82, 44, 19, 700),
        );
        draw_text(
            hdc,
            &state.timer.remaining_label(),
            TextBox::new(96, 48, WIDTH - 16, 92, 31, 800),
        );
        draw_text(
            hdc,
            &state.last_broadcast,
            TextBox::new(18, 104, WIDTH - 16, 132, 12, 400),
        );
        draw_preset_buttons(hdc);
        draw_window_controls(hdc);
    }

    let _ = EndPaint(hwnd, &ps);
}

unsafe fn draw_preset_buttons(hdc: HDC) {
    let presets = visible_presets();
    let count = i32::try_from(presets.len()).unwrap_or(1).max(1);
    let button_width = (PRESET_RIGHT - PRESET_LEFT) / count;
    for (index, preset) in presets.iter().enumerate() {
        let left = PRESET_LEFT + (index as i32 * button_width);
        let right = if index + 1 == presets.len() {
            PRESET_RIGHT
        } else {
            left + button_width - 4
        };
        let brush = CreateSolidBrush(rgb(255, 255, 255));
        FillRect(
            hdc,
            &RECT {
                left,
                top: PRESET_TOP,
                right,
                bottom: PRESET_TOP + PRESET_HEIGHT,
            },
            brush,
        );
        let _ = DeleteObject(brush.into());
        draw_text(
            hdc,
            short_preset_label(preset.id),
            TextBox::new(
                left + 3,
                PRESET_TOP + 5,
                right - 2,
                PRESET_TOP + PRESET_HEIGHT,
                11,
                700,
            ),
        );
    }
}

unsafe fn draw_window_controls(hdc: HDC) {
    draw_text(hdc, "-", TextBox::new(TOGGLE_LEFT, 8, CLOSE_LEFT - 4, 30, 15, 800));
    draw_text(hdc, "x", TextBox::new(CLOSE_LEFT, 8, WIDTH - 12, 30, 14, 800));
}

fn short_preset_label(preset_id: &str) -> &'static str {
    match preset_id {
        "rest-start" => "휴식",
        "stretch" => "기지개",
        "water" => "물",
        "wave" => "인사",
        "cheer" => "응원",
        "back-to-work" => "복귀",
        _ => "보내기",
    }
}

unsafe fn draw_cat_bitmap(hdc: HDC, cat: CatBitmap) {
    let Some(info) = cat.info() else {
        return;
    };
    let bytes = cat.bytes();
    let Some(pixels) = bytes.get(info.pixel_offset..) else {
        return;
    };
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: info.width,
            biHeight: info.height,
            biPlanes: 1,
            biBitCount: info.bit_count,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let _ = StretchDIBits(
        hdc,
        18,
        10,
        64,
        64,
        0,
        0,
        info.width,
        info.height,
        Some(pixels.as_ptr().cast::<c_void>()),
        &bitmap,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

#[derive(Clone, Copy)]
struct TextBox {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    size: i32,
    weight: i32,
}

impl TextBox {
    const fn new(left: i32, top: i32, right: i32, bottom: i32, size: i32, weight: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
            size,
            weight,
        }
    }
}

unsafe fn draw_text(hdc: HDC, text: &str, box_: TextBox) {
    let mut font_name = [0u16; 32];
    write_wide(&mut font_name, "Malgun Gothic");
    let font = windows::Win32::Graphics::Gdi::CreateFontIndirectW(&LOGFONTW {
        lfHeight: -box_.size,
        lfWeight: box_.weight,
        lfFaceName: font_name,
        ..Default::default()
    });
    let old_font = SelectObject(hdc, font.into());
    let mut rect = RECT {
        left: box_.left,
        top: box_.top,
        right: box_.right,
        bottom: box_.bottom,
    };
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    DrawTextW(
        hdc,
        &mut wide,
        &mut rect,
        windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(0),
    );
    SelectObject(hdc, old_font);
    let _ = DeleteObject(font.into());
}

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF((red as u32) | ((green as u32) << 8) | ((blue as u32) << 16))
}
