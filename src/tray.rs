use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::error::{AppError, AppResult};

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Show,
    Quit,
}

pub struct TrayHandle {
    _icon: TrayIcon,
}

pub fn load_egui_app_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn load_tray_icon() -> AppResult<Icon> {
    let img = image::load_from_memory(APP_ICON_PNG)
        .map_err(|e| AppError::Internal(format!("Failed to decode tray icon: {e}")))?
        .to_rgba8();
    let (width, height) = img.dimensions();
    Icon::from_rgba(img.into_raw(), width, height)
        .map_err(|e| AppError::Internal(format!("Failed to create tray icon: {e}")))
}

pub fn setup_tray(
    egui_ctx: egui::Context,
    tx: Sender<TrayCommand>,
    force_quit: Arc<AtomicBool>,
    win32_hwnd: Option<isize>,
) -> AppResult<TrayHandle> {
    let menu = Menu::new();
    let show_item = MenuItem::with_id("show", "Open OpenLauncher", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);

    menu.append_items(&[&show_item, &separator, &quit_item])
        .map_err(|e| AppError::Internal(format!("Failed to create tray menu: {e}")))?;

    let icon = load_tray_icon()?;
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("OpenLauncher")
        .with_icon(icon)
        .with_menu_on_left_click(false)
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build tray icon: {e}")))?;

    let click_tx = tx.clone();
    let click_ctx = egui_ctx.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            if let Some(hwnd) = win32_hwnd {
                restore_win32_window(hwnd);
            }
            let _ = click_tx.send(TrayCommand::Show);
            click_ctx.request_repaint();
        }
    }));

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        match event.id().as_ref() {
            "show" => {
                if let Some(hwnd) = win32_hwnd {
                    restore_win32_window(hwnd);
                }
                let _ = tx.send(TrayCommand::Show);
                egui_ctx.request_repaint();
            }
            "quit" => {
                force_quit.store(true, Ordering::SeqCst);
                let _ = tx.send(TrayCommand::Quit);
                if let Some(hwnd) = win32_hwnd {
                    post_close_win32_window(hwnd);
                }
                egui_ctx.request_repaint();
            }
            _ => {}
        }
    }));

    Ok(TrayHandle { _icon: tray_icon })
}

#[cfg(target_os = "windows")]
pub fn extract_win32_hwnd(cc: &eframe::CreationContext<'_>) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = cc.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn extract_win32_hwnd(_cc: &eframe::CreationContext<'_>) -> Option<isize> {
    None
}

#[cfg(target_os = "windows")]
pub fn configure_windows_frame(hwnd: isize) {
    use core::ffi::c_void;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWCP_ROUND, DWMWA_BORDER_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    let hwnd_ptr = hwnd as HWND;
    let dark_mode: i32 = 1;
    let corner_pref: i32 = DWMWCP_ROUND;
    let border_color_bgr: u32 = 0x0032_3238;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd_ptr,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&raw const dark_mode).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd_ptr,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&raw const corner_pref).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd_ptr,
            DWMWA_BORDER_COLOR as u32,
            (&raw const border_color_bgr).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn configure_windows_frame(_hwnd: isize) {}

#[cfg(target_os = "windows")]
pub fn restore_win32_window(hwnd: isize) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let hwnd_ptr = hwnd as HWND;

    unsafe {
        let _ = ShowWindow(hwnd_ptr, SW_SHOW);
        if IsIconic(hwnd_ptr) != 0 {
            let _ = ShowWindow(hwnd_ptr, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd_ptr);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restore_win32_window(_hwnd: isize) {}

#[cfg(target_os = "windows")]
pub fn post_close_win32_window(hwnd: isize) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    let hwnd_ptr = hwnd as HWND;

    unsafe {
        let _ = PostMessageW(hwnd_ptr, WM_CLOSE, 0, 0);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn post_close_win32_window(_hwnd: isize) {}
