//! A native tray/message thread shared by the comparison hosts.
use std::sync::{Arc, mpsc};
use tray_icon::{
    Icon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};
#[derive(Clone, Copy)]
pub enum ShellEvent {
    Show,
    Quit,
}
pub fn is_visible(handle: usize) -> bool {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(handle as _) != 0
            && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(handle as _) == 0
    }
}
pub fn set_visible(handle: usize, visible: bool) {
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow(
            handle as _,
            if visible { 9 } else { 0 },
        );
        if visible {
            windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(handle as _);
        }
    }
}
pub struct TraySession {
    thread: u32,
}
impl Drop for TraySession {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                self.thread,
                0x0012,
                0,
                0,
            )
        };
    }
}
pub fn tray(
    label: &str,
    callback: impl Fn(ShellEvent) + Send + Sync + 'static,
) -> Result<TraySession, String> {
    let callback = Arc::new(callback);
    let label = label.to_owned();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let show = MenuItem::new("表示する", true, None);
        let quit = MenuItem::new("終了", true, None);
        let menu = Menu::new();
        if menu.append(&show).is_err() || menu.append(&quit).is_err() {
            let _ = tx.send(Err("トレイメニューを作成できません。".into()));
            return;
        }
        let show_id = show.id().clone();
        let quit_id = quit.id().clone();
        let cb = callback.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if e.id == show_id {
                cb(ShellEvent::Show);
            } else if e.id == quit_id {
                cb(ShellEvent::Quit);
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e| {
            if matches!(e, TrayIconEvent::DoubleClick { .. }) {
                callback(ShellEvent::Show);
            }
        }));
        let mut rgba = vec![0u8; 16 * 16 * 4];
        for y in 2..14 {
            for x in 2..14 {
                let i = (y * 16 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[138, 170, 255, 255]);
            }
        }
        let icon = Icon::from_rgba(rgba, 16, 16).unwrap();
        let _tray = match TrayIconBuilder::new()
            .with_tooltip(label)
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
        {
            Ok(t) => t,
            Err(_) => {
                let _ = tx.send(Err("トレイを作成できません。×でアプリを終了します。".into()));
                return;
            }
        };
        let id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        let _ = tx.send(Ok(TraySession { thread: id }));
        let mut msg = unsafe { std::mem::zeroed() };
        unsafe {
            while windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW(
                &mut msg,
                std::ptr::null_mut(),
                0,
                0,
            ) > 0
            {
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }
    });
    rx.recv()
        .map_err(|_| "トレイ用スレッドを開始できません。".to_string())?
}

pub const WINDOW_TITLE: &str = "DiToVR";

/// ウィンドウに付けるモード印（通常とプレビュー）。タイトルは両モードで同じため、
/// 再起動時に別モードのウィンドウを復帰しないよう、所有者が窓の属性として持つ。
const MODE_PROP: &str = "DiToVR.Mode";

fn mode_mark(preview: bool) -> isize {
    // 0 は「印なし」と区別できないため、1 起点にする。
    1 + preview as isize
}

fn prop_name() -> Vec<u16> {
    MODE_PROP.encode_utf16().chain(Some(0)).collect()
}

/// 自分のウィンドウにモード印を付ける。
pub fn tag_window(handle: usize, preview: bool) {
    let name = prop_name();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::SetPropW(
            handle as _,
            name.as_ptr(),
            mode_mark(preview) as _,
        );
    }
}

/// 候補（ウィンドウ, 印）から、同じモードのウィンドウを選ぶ。印のないウィンドウ（印を付ける前や旧版）は
/// 同じモードの候補がないときだけ使い、別のモードの印があるものは使わない。
fn pick_window(candidates: &[(usize, isize)], preview: bool) -> Option<usize> {
    let mark = mode_mark(preview);
    candidates
        .iter()
        .find(|(_, m)| *m == mark)
        .or_else(|| candidates.iter().find(|(_, m)| *m == 0))
        .map(|(window, _)| *window)
}

fn find_instance_window(preview: bool) -> Option<usize> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetPropW, GetWindowTextW};
    unsafe extern "system" fn collect(window: HWND, lparam: LPARAM) -> i32 {
        let found = unsafe { &mut *(lparam as *mut Vec<(usize, isize)>) };
        let mut text = [0u16; 64];
        let length = unsafe { GetWindowTextW(window, text.as_mut_ptr(), text.len() as i32) };
        if length > 0 && String::from_utf16_lossy(&text[..length as usize]) == WINDOW_TITLE {
            let mark = unsafe { GetPropW(window, prop_name().as_ptr()) } as isize;
            found.push((window as usize, mark));
        }
        1
    }
    let mut candidates = Vec::<(usize, isize)>::new();
    unsafe { EnumWindows(Some(collect), &mut candidates as *mut _ as LPARAM) };
    pick_window(&candidates, preview)
}

/// 単一起動の保持。Dropで解放する。
pub struct InstanceGuard(usize);
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0 as _) };
    }
}

/// 最初の起動ならガードを返す。すでに起動中なら既存ウィンドウを復帰して None を返す。
/// プレビューは実利用中のアプリと干渉しないよう別名にする。
pub fn single_instance(preview: bool) -> Option<InstanceGuard> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError},
        System::Threading::CreateMutexW,
    };
    let name: Vec<u16> = format!(
        "Local\\DiToVR-App{}\0",
        if preview { "-Preview" } else { "" }
    )
    .encode_utf16()
    .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Some(InstanceGuard(0));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        if let Some(window) = find_instance_window(preview) {
            set_visible(window, true);
        }
        return None;
    }
    Some(InstanceGuard(handle as usize))
}

/// 固定のhttps URLだけをブラウザーで開く。
pub fn open_url(url: &'static str) {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide("open").as_ptr(),
            wide(url).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaunch_restores_only_the_window_of_the_same_mode() {
        let normal = mode_mark(false);
        let preview = mode_mark(true);
        assert_ne!(normal, preview);
        assert!(normal != 0 && preview != 0);
        // どちらの作成順でも、同じモードの窓を選ぶ。
        assert_eq!(pick_window(&[(1, preview), (2, normal)], false), Some(2));
        assert_eq!(pick_window(&[(1, normal), (2, preview)], false), Some(1));
        assert_eq!(pick_window(&[(1, normal), (2, preview)], true), Some(2));
        // 別モードの窓しかなければ、復帰しない。
        assert_eq!(pick_window(&[(1, preview)], false), None);
        assert_eq!(pick_window(&[(1, normal)], true), None);
        // 印のない窓（印を付ける前・旧版）は、同じモードの候補がないときだけ使う。
        assert_eq!(pick_window(&[(1, 0), (2, normal)], false), Some(2));
        assert_eq!(pick_window(&[(1, 0), (2, preview)], false), Some(1));
    }
}
