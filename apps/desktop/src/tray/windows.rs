//! The Windows notification-area icon. Shell_NotifyIcon reports clicks to a
//! window, so the icon owns a hidden window and message loop on its own thread.
use super::Event;
use anyhow::{Result, bail};
use std::{
    cell::{Cell, RefCell},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Sender},
    },
    thread,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
            NIN_SELECT, NINF_KEY, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            DestroyWindow, DispatchMessageW, GetMessageW, GetSystemMetrics, HICON, IDI_APPLICATION,
            IMAGE_ICON, LR_DEFAULTCOLOR, LoadIconW, LoadImageW, MF_SEPARATOR, MF_STRING, MSG,
            PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SM_CXSMICON,
            SM_CYSMICON, SendMessageW, SetForegroundWindow, SetMenuDefaultItem, TPM_NONOTIFY,
            TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, TranslateMessage, WM_APP, WM_CLOSE,
            WM_CONTEXTMENU, WM_DESTROY, WM_NULL, WNDCLASSW, WS_OVERLAPPED,
        },
    },
};

const CALLBACK: u32 = WM_APP + 1;
const OPEN: usize = 1;
const QUIT: usize = 2;

static WINDOW: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static SENDER: RefCell<Option<Sender<Event>>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
}

pub async fn spawn(sender: Sender<Event>) -> Result<()> {
    let (ready, result) = mpsc::channel();
    thread::Builder::new()
        .name("CatDo tray".into())
        .spawn(move || run(sender, ready))?;
    result.recv()?
}

/// Removes the icon, which Windows otherwise leaves behind until the pointer
/// passes over it.
pub fn remove() {
    let window = WINDOW.swap(0, Ordering::SeqCst);
    if window != 0 {
        unsafe { SendMessageW(window as HWND, WM_CLOSE, 0, 0) };
    }
}

fn run(sender: Sender<Event>, ready: mpsc::Sender<Result<()>>) {
    SENDER.set(Some(sender));
    let window = match unsafe { create() } {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    WINDOW.store(window as usize, Ordering::SeqCst);
    let _ = ready.send(Ok(()));
    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

unsafe fn create() -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide("CatDoTray");
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&window_class) == 0 {
            bail!(std::io::Error::last_os_error());
        }
        TASKBAR_CREATED.set(RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()));
        // Unlike a message-only window, a hidden top-level window hears when
        // Explorer restarts and the icon must be added again.
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("CatDo").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if window.is_null() {
            bail!(std::io::Error::last_os_error());
        }
        if !add(window) {
            DestroyWindow(window);
            bail!("the notification area is unavailable");
        }
        Ok(window)
    }
}

fn icon_data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: 1,
        ..Default::default()
    }
}

unsafe fn add(window: HWND) -> bool {
    let mut data = icon_data(window);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    data.uCallbackMessage = CALLBACK;
    data.hIcon = unsafe { icon() };
    for (slot, unit) in data.szTip.iter_mut().zip("CatDo".encode_utf16()) {
        *slot = unit;
    }
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &data) != 0 && Shell_NotifyIconW(NIM_SETVERSION, &data) != 0
    }
}

unsafe fn icon() -> HICON {
    unsafe {
        // Resource 1 is the application icon embedded by build.rs.
        let icon = LoadImageW(
            GetModuleHandleW(null()),
            1 as _,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_DEFAULTCOLOR,
        );
        if icon.is_null() {
            LoadIconW(null_mut(), IDI_APPLICATION)
        } else {
            icon
        }
    }
}

fn send(event: Event) {
    SENDER.with_borrow(|sender| {
        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    });
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            CALLBACK => {
                // With version 4, the event is in the low word of lparam and
                // the menu's screen position is in wparam.
                match lparam as u32 & 0xffff {
                    NIN_SELECT => send(Event::Show),
                    event if event == NIN_SELECT | NINF_KEY => send(Event::Show),
                    WM_CONTEXTMENU => {
                        menu(window, wparam as i16 as i32, (wparam >> 16) as i16 as i32)
                    }
                    _ => (),
                }
                0
            }
            WM_DESTROY => {
                Shell_NotifyIconW(NIM_DELETE, &icon_data(window));
                PostQuitMessage(0);
                0
            }
            _ if message == TASKBAR_CREATED.get() => {
                // Without an icon, a closed window must be shown again.
                send(if add(window) {
                    Event::Online
                } else {
                    Event::Offline
                });
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

unsafe fn menu(window: HWND, x: i32, y: i32) {
    unsafe {
        let menu = CreatePopupMenu();
        let open = wide("Open CatDo");
        let quit = wide("Quit CatDo");
        AppendMenuW(menu, MF_STRING, OPEN, open.as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, QUIT, quit.as_ptr());
        SetMenuDefaultItem(menu, OPEN as u32, 0);
        // Without this, the menu stays open after clicking elsewhere.
        SetForegroundWindow(window);
        let command = TrackPopupMenuEx(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            x,
            y,
            window,
            null(),
        );
        PostMessageW(window, WM_NULL, 0, 0);
        DestroyMenu(menu);
        match command as usize {
            OPEN => send(Event::Show),
            QUIT => send(Event::Quit),
            _ => (),
        }
    }
}
