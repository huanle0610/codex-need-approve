#![windows_subsystem = "windows"]

use codex_need_approve::{parse_approval_event, should_suppress_duplicate_alert};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::c_void;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type Bool = i32;
type Dword = u32;
type Uint = u32;
type Wparam = usize;
type Lparam = isize;
type Lresult = isize;
type Hwnd = *mut c_void;
type Hinstance = *mut c_void;
type Hicon = *mut c_void;
type Hcursor = *mut c_void;
type Hbrush = *mut c_void;
type Lpcwstr = *const u16;

const WM_DESTROY: Uint = 0x0002;
const WM_COMMAND: Uint = 0x0111;
const WM_RBUTTONDOWN: Uint = 0x0204;
const WM_RBUTTONUP: Uint = 0x0205;
const WM_CONTEXTMENU: Uint = 0x007B;
const WM_USER: Uint = 0x0400;
const TRAY_CALLBACK_MESSAGE: Uint = WM_USER + 1;
const ABOUT_MENU_ID: usize = 1000;
const EXIT_MENU_ID: usize = 1001;
const PM_REMOVE: Uint = 0x0001;
const NIM_ADD: Dword = 0x0000;
const NIM_MODIFY: Dword = 0x0001;
const NIM_DELETE: Dword = 0x0002;
const NIF_MESSAGE: Uint = 0x0001;
const NIF_ICON: Uint = 0x0002;
const NIF_TIP: Uint = 0x0004;
const NIF_INFO: Uint = 0x0010;
const NIIF_WARNING: Dword = 0x00000002;
const SND_ASYNC: Dword = 0x0001;
const SND_NODEFAULT: Dword = 0x0002;
const SND_FILENAME: Dword = 0x00020000;
const MB_OK: Uint = 0x00000000;
const MB_ICONINFORMATION: Uint = 0x00000040;
const MB_ICONEXCLAMATION: Uint = 0x00000030;
const MF_SEPARATOR: Uint = 0x00000800;
const IDI_APPLICATION: usize = 32512;
const IMAGE_ICON: Uint = 1;
const LR_LOADFROMFILE: Uint = 0x0010;
const LR_DEFAULTSIZE: Uint = 0x0040;
const APP_NAME: &str = "Codex Need Approve";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
static SHOULD_EXIT: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct WndClassW {
    style: Uint,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: Hinstance,
    h_icon: Hicon,
    h_cursor: Hcursor,
    hbr_background: Hbrush,
    lpsz_menu_name: Lpcwstr,
    lpsz_class_name: Lpcwstr,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Msg {
    hwnd: Hwnd,
    message: Uint,
    w_param: Wparam,
    l_param: Lparam,
    time: Dword,
    pt: Point,
}

#[repr(C)]
struct NotifyIconDataW {
    cb_size: Dword,
    hwnd: Hwnd,
    uid: Uint,
    u_flags: Uint,
    u_callback_message: Uint,
    h_icon: Hicon,
    sz_tip: [u16; 128],
    dw_state: Dword,
    dw_state_mask: Dword,
    sz_info: [u16; 256],
    u_timeout_or_version: Uint,
    sz_info_title: [u16; 64],
    dw_info_flags: Dword,
    guid_item: [u8; 16],
    h_balloon_icon: Hicon,
}

#[link(name = "user32")]
extern "system" {
    fn DefWindowProcW(hwnd: Hwnd, msg: Uint, wparam: Wparam, lparam: Lparam) -> Lresult;
    fn DispatchMessageW(msg: *const Msg) -> Lresult;
    fn LoadIconW(instance: Hinstance, icon_name: Lpcwstr) -> Hicon;
    fn LoadImageW(
        instance: Hinstance,
        name: Lpcwstr,
        image_type: Uint,
        cx: i32,
        cy: i32,
        flags: Uint,
    ) -> *mut c_void;
    fn MessageBeep(sound_type: Uint) -> Bool;
    fn MessageBoxW(hwnd: Hwnd, text: Lpcwstr, caption: Lpcwstr, flags: Uint) -> i32;
    fn PeekMessageW(msg: *mut Msg, hwnd: Hwnd, min: Uint, max: Uint, remove: Uint) -> Bool;
    fn PostQuitMessage(exit_code: i32);
    fn RegisterClassW(wnd_class: *const WndClassW) -> u16;
    fn CreatePopupMenu() -> *mut c_void;
    fn AppendMenuW(menu: *mut c_void, flags: Uint, id: usize, text: Lpcwstr) -> Bool;
    fn DestroyMenu(menu: *mut c_void) -> Bool;
    fn GetCursorPos(point: *mut Point) -> Bool;
    fn SetForegroundWindow(hwnd: Hwnd) -> Bool;
    fn TrackPopupMenu(
        menu: *mut c_void,
        flags: Uint,
        x: i32,
        y: i32,
        reserved: i32,
        hwnd: Hwnd,
        rect: *const c_void,
    ) -> Bool;
    fn TranslateMessage(msg: *const Msg) -> Bool;
    fn CreateWindowExW(
        ex_style: Dword,
        class_name: Lpcwstr,
        window_name: Lpcwstr,
        style: Dword,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *mut c_void,
        instance: Hinstance,
        param: *mut c_void,
    ) -> Hwnd;
    fn DestroyWindow(hwnd: Hwnd) -> Bool;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: Lpcwstr) -> Hinstance;
}

#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(message: Dword, data: *mut NotifyIconDataW) -> Bool;
}

#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(sound: Lpcwstr, module: Hinstance, flags: Dword) -> Bool;
}

unsafe extern "system" fn wnd_proc(
    hwnd: Hwnd,
    msg: Uint,
    wparam: Wparam,
    lparam: Lparam,
) -> Lresult {
    match msg {
        WM_DESTROY => {
            request_exit();
            PostQuitMessage(0);
            0
        }
        WM_COMMAND => match wparam & 0xffff {
            ABOUT_MENU_ID => {
                show_about(hwnd);
                0
            }
            EXIT_MENU_ID => {
                request_exit();
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        },
        TRAY_CALLBACK_MESSAGE => {
            let event = (lparam & 0xffff) as Uint;
            if event == WM_RBUTTONDOWN || event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                show_tray_menu(hwnd);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn request_exit() {
    SHOULD_EXIT.store(true, Ordering::SeqCst);
}

fn should_exit() -> bool {
    SHOULD_EXIT.load(Ordering::SeqCst)
}

unsafe fn show_tray_menu(hwnd: Hwnd) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let about_text = wide_null("About");
    let exit_text = wide_null("Exit");
    let _ = AppendMenuW(menu, 0, ABOUT_MENU_ID, about_text.as_ptr());
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
    let _ = AppendMenuW(menu, 0, EXIT_MENU_ID, exit_text.as_ptr());
    let mut point = Point { x: 0, y: 0 };
    if GetCursorPos(&mut point) != 0 {
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, 0, point.x, point.y, 0, hwnd, null());
    }
    let _ = DestroyMenu(menu);
}

unsafe fn show_about(hwnd: Hwnd) {
    let body = format!(
        "{APP_NAME} v{APP_VERSION}\n\n当 Codex Desktop 出现需要你确认的 approval/permission 卡片时，播放声音提醒。\n\n右键托盘图标可以查看 About 或退出程序。\n\nGitHub: {APP_REPOSITORY}"
    );
    let body = wide_null(&body);
    let title = wide_null(APP_NAME);
    let _ = MessageBoxW(
        hwnd,
        body.as_ptr(),
        title.as_ptr(),
        MB_OK | MB_ICONINFORMATION,
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let outputs_dir =
        exe_dir().unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let log_path = outputs_dir.join("codex-approval-watcher-rs.log");
    let sound_path = find_sound_path(&outputs_dir);

    if args.iter().any(|a| a == "--test-alert") {
        let mut logger = Logger::new(log_path);
        logger.log("test-alert requested");
        match NativeNotifier::new() {
            Ok(mut notifier) => {
                notifier.alert(
                    APP_NAME,
                    "A confirmation card is waiting in Codex.",
                    sound_path.as_deref(),
                    &mut logger,
                );
                notifier.pump_for(Duration::from_secs(8));
            }
            Err(err) => {
                logger.log(&format!("notifier-init-failed {err}"));
                play_sound(sound_path.as_deref(), &mut logger);
            }
        }
        return;
    }

    let mut logger = Logger::new(log_path);
    logger.log("watcher starting");
    let log_root = codex_log_root();
    logger.log(&format!("codex-log-root {}", log_root.display()));
    logger.log(&format!(
        "sound-path {}",
        sound_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    ));

    let mut notifier = match NativeNotifier::new() {
        Ok(n) => n,
        Err(err) => {
            logger.log(&format!("notifier-init-failed {err}"));
            return;
        }
    };

    let mut watcher = LogWatcher::new(log_root);
    watcher.initialize_positions(&mut logger);
    let mut ui_scanner = UiApprovalScanner::new(&mut logger);
    let mut last_ui_signature: Option<String> = None;
    let mut last_alert_at: Option<Instant> = None;
    while !should_exit() {
        for event in watcher.poll(&mut logger) {
            logger.log(&format!("approval-detected id={}", event.id));
            alert_with_cooldown(
                &mut notifier,
                sound_path.as_deref(),
                &mut logger,
                &mut last_alert_at,
                "log",
                Duration::from_secs(2),
            );
        }

        if let Some(scanner) = ui_scanner.as_mut() {
            match scanner.find_approval_card(&mut logger) {
                Some(signature) => {
                    if last_ui_signature.as_deref() != Some(signature.as_str()) {
                        logger.log(&format!("ui-approval-detected signature={signature}"));
                        alert_with_cooldown(
                            &mut notifier,
                            sound_path.as_deref(),
                            &mut logger,
                            &mut last_alert_at,
                            "ui",
                            Duration::from_secs(2),
                        );
                        last_ui_signature = Some(signature);
                    }
                }
                None => {
                    last_ui_signature = None;
                }
            }
        }

        notifier.pump_for(Duration::from_millis(50));
        thread::sleep(Duration::from_millis(450));
    }
}

fn alert_with_cooldown(
    notifier: &mut NativeNotifier,
    sound_path: Option<&Path>,
    logger: &mut Logger,
    last_alert_at: &mut Option<Instant>,
    source: &str,
    cooldown: Duration,
) {
    let now = Instant::now();
    if should_suppress_duplicate_alert(last_alert_at.map(|t| now.duration_since(t)), cooldown) {
        logger.log(&format!("alert-suppressed source={source}"));
        return;
    }
    logger.log(&format!("alert source={source}"));
    notifier.alert(
        APP_NAME,
        "A confirmation card is waiting in Codex.",
        sound_path,
        logger,
    );
    *last_alert_at = Some(now);
}

struct Logger {
    path: PathBuf,
}

impl Logger {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn log(&mut self, message: &str) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{ts} {message}");
        }
    }
}

struct LogWatcher {
    root: PathBuf,
    positions: HashMap<PathBuf, u64>,
    seen: HashSet<String>,
    initialized: bool,
}

impl LogWatcher {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            positions: HashMap::new(),
            seen: HashSet::new(),
            initialized: false,
        }
    }

    fn initialize_positions(&mut self, logger: &mut Logger) {
        for path in find_log_files(&self.root) {
            if let Ok(meta) = fs::metadata(&path) {
                self.positions.insert(path, meta.len());
            }
        }
        self.initialized = true;
        logger.log(&format!("initialized-files count={}", self.positions.len()));
    }

    fn poll(&mut self, logger: &mut Logger) -> Vec<codex_need_approve::ApprovalEvent> {
        let mut events = Vec::new();
        for path in find_log_files(&self.root) {
            let previous = match self.positions.get(&path).copied() {
                Some(pos) => pos,
                None if self.initialized => 0,
                None => fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
            };

            let mut file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let len = file.metadata().map(|m| m.len()).unwrap_or(previous);
            let start = if len < previous { 0 } else { previous };
            if file.seek(SeekFrom::Start(start)).is_err() {
                continue;
            }
            let mut text = String::new();
            if file.read_to_string(&mut text).is_err() {
                self.positions.insert(path, len);
                continue;
            }
            self.positions.insert(path.clone(), len);
            for line in text.lines() {
                if let Some(event) = parse_approval_event(line, &path.display().to_string()) {
                    if self.seen.insert(event.id.clone()) {
                        logger.log(&format!("matched id={} line={}", event.id, line));
                        events.push(event);
                    }
                }
            }
        }
        events
    }
}

struct NativeNotifier {
    hwnd: Hwnd,
    icon: Hicon,
    nid: NotifyIconDataW,
}

impl NativeNotifier {
    fn new() -> Result<Self, String> {
        unsafe {
            let class_name = wide_null("CodexApprovalWatcherWindow");
            let instance = GetModuleHandleW(null());
            let icon = load_app_icon()
                .unwrap_or_else(|| LoadIconW(null_mut(), IDI_APPLICATION as Lpcwstr));
            let wc = WndClassW {
                style: 0,
                lpfn_wnd_proc: Some(wnd_proc),
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: instance,
                h_icon: icon,
                h_cursor: null_mut(),
                hbr_background: null_mut(),
                lpsz_menu_name: null(),
                lpsz_class_name: class_name.as_ptr(),
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide_null(APP_NAME).as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW returned null".to_string());
            }

            let mut nid = empty_nid();
            nid.cb_size = size_of::<NotifyIconDataW>() as Dword;
            nid.hwnd = hwnd;
            nid.uid = 1;
            nid.u_flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.u_callback_message = TRAY_CALLBACK_MESSAGE;
            nid.h_icon = icon;
            set_tray_tooltip(&mut nid);
            if Shell_NotifyIconW(NIM_ADD, &mut nid) == 0 {
                DestroyWindow(hwnd);
                return Err("Shell_NotifyIconW(NIM_ADD) failed".to_string());
            }
            Ok(Self { hwnd, icon, nid })
        }
    }

    fn alert(&mut self, title: &str, body: &str, sound_path: Option<&Path>, logger: &mut Logger) {
        play_sound(sound_path, logger);
        unsafe {
            self.nid.u_flags = NIF_INFO | NIF_ICON | NIF_TIP | NIF_MESSAGE;
            self.nid.dw_info_flags = NIIF_WARNING;
            set_tray_tooltip(&mut self.nid);
            self.nid.u_timeout_or_version = 10_000;
            self.nid.h_icon = self.icon;
            copy_wide(&mut self.nid.sz_info_title, title);
            copy_wide(&mut self.nid.sz_info, body);
            let ok = Shell_NotifyIconW(NIM_MODIFY, &mut self.nid) != 0;
            logger.log(&format!(
                "tray-balloon {}",
                if ok { "ok" } else { "failed" }
            ));
            let _ = MessageBeep(MB_ICONEXCLAMATION);
        }
    }

    fn pump_for(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline && !should_exit() {
            unsafe {
                let mut msg: Msg = zeroed();
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == 0x0012 {
                        request_exit();
                        return;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for NativeNotifier {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn play_sound(sound_path: Option<&Path>, logger: &mut Logger) {
    unsafe {
        if let Some(path) = sound_path {
            let wide = wide_null(&path.display().to_string());
            let ok = PlaySoundW(
                wide.as_ptr(),
                null_mut(),
                SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
            ) != 0;
            logger.log(&format!(
                "play-sound {} path={}",
                if ok { "ok" } else { "failed" },
                path.display()
            ));
            if ok {
                return;
            }
        }
        let ok = MessageBeep(MB_ICONEXCLAMATION) != 0;
        logger.log(&format!(
            "message-beep {}",
            if ok { "ok" } else { "failed" }
        ));
    }
}

fn find_log_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_log_files(root, &mut out);
    out
}

fn collect_log_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_log_files(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("log"))
        {
            out.push(path);
        }
    }
}

fn codex_log_root() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Codex")
        .join("Logs")
}

fn exe_dir() -> Option<PathBuf> {
    env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

fn find_icon_path(outputs_dir: &Path) -> Option<PathBuf> {
    let local = outputs_dir.join("codex-need-approve.ico");
    if local.exists() {
        return Some(local);
    }
    None
}

fn load_app_icon() -> Option<Hicon> {
    let dir = exe_dir()?;
    let path = find_icon_path(&dir)?;
    let wide = wide_null(&path.display().to_string());
    let icon = unsafe {
        LoadImageW(
            null_mut(),
            wide.as_ptr(),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
    };
    if icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

fn find_sound_path(outputs_dir: &Path) -> Option<PathBuf> {
    let local = outputs_dir.join("approval-alert.wav");
    if local.exists() {
        return Some(local);
    }
    let codex = PathBuf::from(
        r"C:\Program Files\WindowsApps\OpenAI.Codex_26.616.4196.0_x64__2p2nqsd0c76g0\app\resources\codex-notification.wav",
    );
    if codex.exists() {
        return Some(codex);
    }
    None
}

fn empty_nid() -> NotifyIconDataW {
    NotifyIconDataW {
        cb_size: 0,
        hwnd: null_mut(),
        uid: 0,
        u_flags: 0,
        u_callback_message: 0,
        h_icon: null_mut(),
        sz_tip: [0; 128],
        dw_state: 0,
        dw_state_mask: 0,
        sz_info: [0; 256],
        u_timeout_or_version: 0,
        sz_info_title: [0; 64],
        dw_info_flags: 0,
        guid_item: [0; 16],
        h_balloon_icon: null_mut(),
    }
}

fn set_tray_tooltip(nid: &mut NotifyIconDataW) {
    copy_wide(&mut nid.sz_tip, APP_NAME);
}

fn copy_wide<const N: usize>(dest: &mut [u16; N], text: &str) {
    dest.fill(0);
    for (slot, ch) in dest
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(text.encode_utf16())
    {
        *slot = ch;
    }
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

struct UiApprovalScanner {
    automation: windows::Win32::UI::Accessibility::IUIAutomation,
    condition: windows::Win32::UI::Accessibility::IUIAutomationCondition,
}

impl UiApprovalScanner {
    fn new(logger: &mut Logger) -> Option<Self> {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            );
            let automation: windows::Win32::UI::Accessibility::IUIAutomation =
                match windows::Win32::System::Com::CoCreateInstance(
                    &windows::Win32::UI::Accessibility::CUIAutomation,
                    None,
                    windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
                ) {
                    Ok(a) => a,
                    Err(err) => {
                        logger.log(&format!("ui-scanner-init-failed {err:?}"));
                        return None;
                    }
                };
            let condition = match automation.CreateTrueCondition() {
                Ok(c) => c,
                Err(err) => {
                    logger.log(&format!("ui-scanner-condition-failed {err:?}"));
                    return None;
                }
            };
            logger.log("ui-scanner-ready");
            Some(Self {
                automation,
                condition,
            })
        }
    }

    fn find_approval_card(&mut self, _logger: &mut Logger) -> Option<String> {
        let hwnd = find_codex_window()?;
        unsafe {
            let root = self.automation.ElementFromHandle(hwnd).ok()?;
            let elements = root
                .FindAll(
                    windows::Win32::UI::Accessibility::TreeScope_Descendants,
                    &self.condition,
                )
                .ok()?;
            let len = elements.Length().unwrap_or(0).min(1500);
            let mut has_yes = false;
            let mut has_dont_ask = false;
            let mut has_submit = false;
            let mut has_awaiting_approval = false;
            let mut has_running_command = false;
            let mut matched_names: Vec<String> = Vec::new();

            for i in 0..len {
                let el = match elements.GetElement(i) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let control_type = el.CurrentControlType().map(|c| c.0).unwrap_or_default();
                if control_type != 50000 {
                    continue;
                }
                let name = el.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let lower = trimmed.to_ascii_lowercase();
                if trimmed == "Yes" {
                    has_yes = true;
                    matched_names.push(trimmed.to_string());
                } else if lower.starts_with("yes, and don't ask again") {
                    has_dont_ask = true;
                    matched_names.push(trimmed.to_string());
                } else if trimmed == "Submit" {
                    has_submit = true;
                    matched_names.push(trimmed.to_string());
                }
                if lower.contains("awaiting approval") {
                    has_awaiting_approval = true;
                    matched_names.push(trimmed.to_string());
                }
                if lower.starts_with("running ") || lower.contains(" running ") {
                    has_running_command = true;
                    matched_names.push(trimmed.to_string());
                }
            }

            if (has_yes && has_dont_ask && has_submit)
                || (has_awaiting_approval && has_running_command)
            {
                matched_names.sort();
                matched_names.dedup();
                Some(matched_names.join("|"))
            } else {
                None
            }
        }
    }
}

fn find_codex_window() -> Option<windows::Win32::Foundation::HWND> {
    let mut hwnds: Vec<windows::Win32::Foundation::HWND> = Vec::new();
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(enum_codex_window),
            windows::Win32::Foundation::LPARAM(&mut hwnds as *mut _ as isize),
        );
    }
    hwnds.into_iter().next()
}

unsafe extern "system" fn enum_codex_window(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::core::BOOL {
    if !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd).as_bool() {
        return windows::core::BOOL(1);
    }
    let title = window_text_for_hwnd(hwnd);
    if title == "Codex" {
        let hwnds = &mut *(lparam.0 as *mut Vec<windows::Win32::Foundation::HWND>);
        hwnds.push(hwnd);
        return windows::core::BOOL(0);
    }
    windows::core::BOOL(1)
}

fn window_text_for_hwnd(hwnd: windows::Win32::Foundation::HWND) -> String {
    let mut buf = [0u16; 512];
    let len =
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..len])
}
