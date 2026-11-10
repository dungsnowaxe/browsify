use crate::config_loader::BrowsifyEngine;
use crate::launcher::execute_launch_plan;
use crate::ui_server::{
    is_ui_server_active, log_event, notify_config_reloaded, record_routed_url, LogLevel,
};
use browsify_core::{AppType, ModifierKeys, OpenerInfo, PowerInfo, ProfileResolver, SystemInfo};
use browsify_js::{register_host_callbacks, HostQueryCallbacks};
use std::fs::{DirBuilder, File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const BROWSIFY_BUNDLE_ID: &str = "com.lavenxsnow.browsify";

static LAST_DAEMON_ACTIVITY: OnceLock<Mutex<Instant>> = OnceLock::new();

fn touch_daemon_activity() {
    let lock = LAST_DAEMON_ACTIVITY.get_or_init(|| Mutex::new(Instant::now()));
    if let Ok(mut t) = lock.lock() {
        *t = Instant::now();
    }
}

fn daemon_idle_elapsed() -> Duration {
    LAST_DAEMON_ACTIVITY
        .get_or_init(|| Mutex::new(Instant::now()))
        .lock()
        .map(|t| t.elapsed())
        .unwrap_or(Duration::ZERO)
}

/// Converts a local filesystem path to a percent-encoded `file://` URL
/// (ensuring characters like `#`, `?`, and spaces are properly escaped).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn file_path_to_url(path_str: &str) -> Result<String, String> {
    Url::from_file_path(path_str)
        .map(|u| u.into())
        .map_err(|()| format!("Invalid file path for file URL: {}", path_str))
}

/// Posts a macOS user notification for `finicky.notify(title, subtitle)`
/// (invoked lazily from QuickJS via `__browsifyHostNotify`).
pub fn post_notification(title: &str, subtitle: &str) {
    #[cfg(target_os = "macos")]
    {
        let escape = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
        let script = format!(
            "display notification {} with title {}",
            escape(subtitle),
            escape(title)
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, subtitle);
    }
}

/// Initializes lazy OS host callbacks for QuickJS.
pub fn init_host_hooks() {
    register_host_callbacks(HostQueryCallbacks {
        get_modifier_keys: current_modifier_keys,
        get_system_info: current_system_info,
        get_power_info: current_power_info,
        is_app_running,
        notify: post_notification,
        get_window_title: lookup_window_title,
    });
}

/// Acquires an exclusive non-blocking `flock` in `/tmp/browsify-startup-<uid>/startup.lock`
/// to prevent duplicate concurrent daemon instances (fixes Finicky Issue #515).
pub fn acquire_singleton_lock() -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

        let uid = unsafe { libc::geteuid() };
        let lock_dir = PathBuf::from(format!("/tmp/browsify-startup-{}", uid));
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&lock_dir)?;
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .open(lock_dir.join("startup.lock"))?;
        let rc = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(lock_file)
    }
    #[cfg(not(unix))]
    {
        File::open(".")
    }
}

/// Reads global session modifier key state via `CGEventSourceFlagsState` on macOS.
pub fn current_modifier_keys() -> ModifierKeys {
    #[cfg(target_os = "macos")]
    {
        const KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE: i32 = 0;
        const KCG_EVENT_FLAG_MASK_ALPHA_SHIFT: u64 = 1 << 16;
        const KCG_EVENT_FLAG_MASK_SHIFT: u64 = 1 << 17;
        const KCG_EVENT_FLAG_MASK_CONTROL: u64 = 1 << 18;
        const KCG_EVENT_FLAG_MASK_ALTERNATE: u64 = 1 << 19;
        const KCG_EVENT_FLAG_MASK_COMMAND: u64 = 1 << 20;
        const KCG_EVENT_FLAG_MASK_SECONDARY_FN: u64 = 1 << 23;

        #[link(name = "CoreGraphics", kind = "framework")]
        unsafe extern "C" {
            fn CGEventSourceFlagsState(state_id: i32) -> u64;
        }

        let flags =
            unsafe { CGEventSourceFlagsState(KCG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE) };
        let fn_down = flags & KCG_EVENT_FLAG_MASK_SECONDARY_FN != 0;
        ModifierKeys {
            shift: flags & KCG_EVENT_FLAG_MASK_SHIFT != 0,
            option: flags & KCG_EVENT_FLAG_MASK_ALTERNATE != 0,
            command: flags & KCG_EVENT_FLAG_MASK_COMMAND != 0,
            control: flags & KCG_EVENT_FLAG_MASK_CONTROL != 0,
            caps_lock: flags & KCG_EVENT_FLAG_MASK_ALPHA_SHIFT != 0,
            fn_key: fn_down,
            function: fn_down,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ModifierKeys::default()
    }
}

/// Reads local computer name and hostname lazily when `finicky.getSystemInfo()` is called.
pub fn current_system_info() -> SystemInfo {
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        let name = unsafe {
            if libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) == 0 {
                let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                String::from_utf8_lossy(&buf[..len]).into_owned()
            } else {
                "Mac".to_string()
            }
        };
        SystemInfo {
            localized_name: name.clone(),
            name,
        }
    }
    #[cfg(not(unix))]
    {
        SystemInfo {
            localized_name: "Mac".to_string(),
            name: "mac".to_string(),
        }
    }
}

/// Reads battery / AC power state lazily when `finicky.getPowerInfo()` is called.
pub fn current_power_info() -> PowerInfo {
    #[cfg(target_os = "macos")]
    {
        macos_impl::read_power_info_iokit()
    }
    #[cfg(not(target_os = "macos"))]
    {
        PowerInfo {
            is_charging: false,
            is_connected: true,
            percentage: Some(100),
        }
    }
}

/// Checks if an application (by bundle ID or application name) is currently running.
pub fn is_app_running(identifier: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_impl::is_app_running_nsworkspace(identifier)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = identifier;
        false
    }
}

/// Checks if `com.lavenxsnow.browsify` is currently the default handler for `http` and `https`.
pub fn is_default_browser() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_impl::check_is_default_browser()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Requests macOS LaunchServices to set `Browsify.app` as the default handler for `http` and `https`.
pub fn set_as_default_browser() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_impl::request_set_default_browser()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Lazily reads an opener's focused window title via the macOS AX API
/// (invoked from QuickJS only when a JS callback actually reads `options.opener.windowTitle`).
pub fn lookup_window_title(pid: i32) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos_impl::lookup_window_title_ax(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// Activates a running browser application across macOS Spaces after profile helper handoff.
pub fn activate_running_browser(app_type: AppType, browser_name: &str) {
    #[cfg(target_os = "macos")]
    {
        macos_impl::activate_running_app(app_type, browser_name);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app_type, browser_name);
    }
}

/// Starts the background config hot-reload watcher and optional GitHub Releases update checker,
/// then runs the macOS AppKit `NSApplication` daemon loop.
pub fn run_daemon(
    engine: Arc<Mutex<BrowsifyEngine>>,
    resolver: Arc<ProfileResolver>,
    force_window: bool,
    dry_run: bool,
) {
    let _lock = match acquire_singleton_lock() {
        Ok(file) => file,
        Err(err) => {
            eprintln!("Browsify is already running ({}). Exiting.", err);
            return;
        }
    };

    touch_daemon_activity();
    let home_dir = resolver.home_dir().to_path_buf();

    // 1. Background Config Hot-Reload Watcher & `keepRunning: false` Idle Lifecycle Monitor
    {
        let engine = Arc::clone(&engine);
        let home_dir = home_dir.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(1000));
            let (should_reload, keep_running) = engine
                .lock()
                .map(|eng| (eng.check_files_changed(), eng.summary().options.keep_running))
                .unwrap_or((false, true));

            if should_reload {
                touch_daemon_activity();
                if let Ok(mut eng) = engine.lock() {
                    match eng.reload() {
                        Ok(()) => {
                            log_event(LogLevel::Info, "Configuration hot-reloaded from disk");
                            notify_config_reloaded(&eng);
                        }
                        Err(err) => {
                            log_event(LogLevel::Error, format!("Config reload failed: {}", err));
                        }
                    }
                }
                crate::update::trigger_update_check_if_enabled(&engine, &home_dir);
            }

            if !keep_running
                && !is_ui_server_active()
                && daemon_idle_elapsed() >= Duration::from_secs(2)
            {
                log_event(LogLevel::Info, "Exiting daemon (keepRunning is false and idle timeout elapsed)");
                #[cfg(target_os = "macos")]
                macos_impl::terminate_nsapp();
                #[cfg(not(target_os = "macos"))]
                std::process::exit(0);
            }
        });
    }

    // 2. Periodic 24-hour GitHub Releases Update Scheduler (respects dynamic config changes)
    crate::update::trigger_update_check_if_enabled(&engine, &home_dir);
    {
        let engine = Arc::clone(&engine);
        let home_dir = home_dir.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(86_400));
            crate::update::trigger_update_check_if_enabled(&engine, &home_dir);
        });
    }

    #[cfg(target_os = "macos")]
    {
        macos_impl::run_nsapplication_loop(engine, resolver, force_window, dry_run);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = dry_run;
        if force_window {
            let _ = crate::ui_server::run_ui_server(engine, resolver, 0, true);
        } else {
            let sum = engine.lock().unwrap().summary().clone();
            println!(
                "Browsify v{} ({}) — Configuration valid ({} handlers, {} rewrites).",
                env!("CARGO_PKG_VERSION"),
                sum.format,
                sum.handlers_count,
                sum.rewrites_count
            );
        }
    }
}

/// Routes a URL received by the daemon (via `kAEGetURL`, `openFile:`, Handoff, or SSO).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn handle_daemon_url_event(
    engine: &Arc<Mutex<BrowsifyEngine>>,
    resolver: &Arc<ProfileResolver>,
    raw_url: &str,
    opener: Option<OpenerInfo>,
    open_in_background: bool,
    dry_run: bool,
) {
    touch_daemon_activity();
    log_event(LogLevel::Info, format!("Routing URL: {}", raw_url));
    let (resolved_res, missing_policy) = {
        let Ok(eng) = engine.lock() else {
            return;
        };
        (
            eng.resolve(raw_url, opener.as_ref(), open_in_background),
            eng.summary().options.on_missing_profile,
        )
    };

    match resolved_res {
        Ok(cfg) => {
            let plan = resolver.build_launch_plan(&cfg, open_in_background, missing_policy);
            if let Some(ref w) = plan.warning {
                log_event(LogLevel::Warn, w.clone());
            }
            record_routed_url(raw_url, &plan.final_url, &plan.browser_name, &cfg.profile);
            if let Err(err) = execute_launch_plan(&plan, dry_run) {
                log_event(LogLevel::Error, err);
            }
        }
        Err(err) => {
            log_event(LogLevel::Error, format!("Failed evaluating URL: {}", err));
        }
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos_impl {
    use super::*;
    use crate::ui_server::{get_recent_routed_urls, spawn_or_open_ui_server};
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::ptr;

    type Id = *mut c_void;
    type Class = *mut c_void;
    type Sel = *mut c_void;
    type Imp = *const c_void;

    #[cfg_attr(target_os = "macos", link(name = "objc", kind = "dylib"))]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Class;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra_bytes: usize) -> Class;
        fn class_addMethod(cls: Class, name: Sel, imp: Imp, types: *const c_char) -> bool;
        fn objc_registerClassPair(cls: Class);
        fn objc_msgSend();
    }

    #[cfg_attr(target_os = "macos", link(name = "Cocoa", kind = "framework"))]
    unsafe extern "C" {}

    #[cfg_attr(
        target_os = "macos",
        link(name = "ApplicationServices", kind = "framework")
    )]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> *mut c_void;
        fn AXUIElementCopyAttributeValue(
            element: *mut c_void,
            attribute: *const c_void,
            value: *mut *mut c_void,
        ) -> i32;
        fn CFRelease(cf: *const c_void);
    }

    #[cfg_attr(target_os = "macos", link(name = "IOKit", kind = "framework"))]
    unsafe extern "C" {
        fn IOPSCopyPowerSourcesInfo() -> *mut c_void;
        fn IOPSCopyPowerSourcesList(blob: *mut c_void) -> *mut c_void;
        fn IOPSGetPowerSourceDescription(blob: *mut c_void, ps: *const c_void) -> *mut c_void;
    }

    struct DaemonContext {
        engine: Arc<Mutex<BrowsifyEngine>>,
        resolver: Arc<ProfileResolver>,
        force_window: bool,
        dry_run: bool,
        status_item: Mutex<usize>,
    }

    static DAEMON_CTX: OnceLock<DaemonContext> = OnceLock::new();

    unsafe fn cls(name: &str) -> Class {
        let c = CString::new(name).unwrap();
        unsafe { objc_getClass(c.as_ptr()) }
    }

    unsafe fn sel(name: &str) -> Sel {
        let c = CString::new(name).unwrap();
        unsafe { sel_registerName(c.as_ptr()) }
    }

    unsafe fn nsstring(s: &str) -> Id {
        let c = CString::new(s).unwrap_or_default();
        let f: unsafe extern "C" fn(Class, Sel, *const c_char) -> Id =
            unsafe { std::mem::transmute(objc_msgSend as *const c_void) };
        unsafe { f(cls("NSString"), sel("stringWithUTF8String:"), c.as_ptr()) }
    }

    unsafe fn nsstring_to_rust(ns: Id) -> Option<String> {
        if ns.is_null() {
            return None;
        }
        let f: unsafe extern "C" fn(Id, Sel) -> *const c_char =
            unsafe { std::mem::transmute(objc_msgSend as *const c_void) };
        let ptr = unsafe { f(ns, sel("UTF8String")) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() })
        }
    }

    pub fn terminate_nsapp() {
        std::process::exit(0);
    }

    pub fn read_power_info_iokit() -> PowerInfo {
        unsafe {
            let blob = IOPSCopyPowerSourcesInfo();
            if blob.is_null() {
                return PowerInfo {
                    is_charging: false,
                    is_connected: true,
                    percentage: Some(100),
                };
            }
            let list = IOPSCopyPowerSourcesList(blob);
            if list.is_null() {
                CFRelease(blob);
                return PowerInfo {
                    is_charging: false,
                    is_connected: true,
                    percentage: Some(100),
                };
            }
            let count_fn: unsafe extern "C" fn(Id, Sel) -> usize =
                std::mem::transmute(objc_msgSend as *const c_void);
            let count = count_fn(list, sel("count"));
            if count == 0 {
                CFRelease(list);
                CFRelease(blob);
                return PowerInfo {
                    is_charging: false,
                    is_connected: true,
                    percentage: Some(100),
                };
            }
            let obj_at: unsafe extern "C" fn(Id, Sel, usize) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let ps = obj_at(list, sel("objectAtIndex:"), 0);
            let desc = IOPSGetPowerSourceDescription(blob, ps);
            if desc.is_null() {
                CFRelease(list);
                CFRelease(blob);
                return PowerInfo {
                    is_charging: false,
                    is_connected: true,
                    percentage: Some(100),
                };
            }
            let dict_get: unsafe extern "C" fn(Id, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let int_val: unsafe extern "C" fn(Id, Sel) -> i32 =
                std::mem::transmute(objc_msgSend as *const c_void);
            let bool_val: unsafe extern "C" fn(Id, Sel) -> bool =
                std::mem::transmute(objc_msgSend as *const c_void);

            let cap_obj = dict_get(desc, sel("objectForKey:"), nsstring("Current Capacity"));
            let charging_obj = dict_get(desc, sel("objectForKey:"), nsstring("Is Charging"));
            let state_obj = dict_get(desc, sel("objectForKey:"), nsstring("Power Source State"));

            let percentage = if !cap_obj.is_null() {
                Some(int_val(cap_obj, sel("intValue")))
            } else {
                None
            };
            let is_charging = if !charging_obj.is_null() {
                bool_val(charging_obj, sel("boolValue"))
            } else {
                false
            };
            let is_connected = if let Some(s) = nsstring_to_rust(state_obj) {
                s == "AC Power"
            } else {
                is_charging
            };

            CFRelease(list);
            CFRelease(blob);
            PowerInfo {
                is_charging,
                is_connected,
                percentage,
            }
        }
    }

    pub fn is_app_running_nsworkspace(identifier: &str) -> bool {
        unsafe {
            let msg_id: unsafe extern "C" fn(Class, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let count_fn: unsafe extern "C" fn(Id, Sel) -> usize =
                std::mem::transmute(objc_msgSend as *const c_void);

            let running_cls = cls("NSRunningApplication");
            let apps = msg_id(
                running_cls,
                sel("runningApplicationsWithBundleIdentifier:"),
                nsstring(identifier),
            );
            if !apps.is_null() && count_fn(apps, sel("count")) > 0 {
                return true;
            }

            let ws_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let ws = ws_fn(cls("NSWorkspace"), sel("sharedWorkspace"));
            let all_apps = ws_fn(ws, sel("runningApplications"));
            if all_apps.is_null() {
                return false;
            }
            let total = count_fn(all_apps, sel("count"));
            let obj_at: unsafe extern "C" fn(Id, Sel, usize) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            for i in 0..total {
                let app = obj_at(all_apps, sel("objectAtIndex:"), i);
                let name_obj = ws_fn(app, sel("localizedName"));
                if let Some(name) = nsstring_to_rust(name_obj) {
                    if name.eq_ignore_ascii_case(identifier) {
                        return true;
                    }
                }
            }
            false
        }
    }

    pub fn activate_running_app(app_type: AppType, browser_name: &str) {
        unsafe {
            let count_fn: unsafe extern "C" fn(Id, Sel) -> usize =
                std::mem::transmute(objc_msgSend as *const c_void);
            let obj_at: unsafe extern "C" fn(Id, Sel, usize) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let activate_fn: unsafe extern "C" fn(Id, Sel, usize) -> bool =
                std::mem::transmute(objc_msgSend as *const c_void);
            const ACTIVATE_OPTS: usize = 1 | 2;

            let target_bundle_id = if app_type == AppType::BundleId {
                Some(browser_name)
            } else {
                browsify_core::lookup_browser_meta(browser_name).map(|m| m.id)
            };

            if let Some(bid) = target_bundle_id {
                let msg_id: unsafe extern "C" fn(Class, Sel, Id) -> Id =
                    std::mem::transmute(objc_msgSend as *const c_void);
                let apps = msg_id(
                    cls("NSRunningApplication"),
                    sel("runningApplicationsWithBundleIdentifier:"),
                    nsstring(bid),
                );
                if !apps.is_null() {
                    let n = count_fn(apps, sel("count"));
                    for i in 0..n {
                        let app = obj_at(apps, sel("objectAtIndex:"), i);
                        let _ = activate_fn(app, sel("activateWithOptions:"), ACTIVATE_OPTS);
                    }
                    if n > 0 {
                        return;
                    }
                }
            }

            let ws_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let ws = ws_fn(cls("NSWorkspace"), sel("sharedWorkspace"));
            let all_apps = ws_fn(ws, sel("runningApplications"));
            if all_apps.is_null() {
                return;
            }
            let total = count_fn(all_apps, sel("count"));
            for i in 0..total {
                let app = obj_at(all_apps, sel("objectAtIndex:"), i);
                let name_obj = ws_fn(app, sel("localizedName"));
                if let Some(name) = nsstring_to_rust(name_obj) {
                    if name.eq_ignore_ascii_case(browser_name) {
                        let _ = activate_fn(app, sel("activateWithOptions:"), ACTIVATE_OPTS);
                    }
                }
            }
        }
    }

    pub fn check_is_default_browser() -> bool {
        unsafe {
            let ws_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let url_fn: unsafe extern "C" fn(Class, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let app_for_url: unsafe extern "C" fn(Id, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);

            let ws = ws_fn(cls("NSWorkspace"), sel("sharedWorkspace"));
            let test_url = url_fn(cls("NSURL"), sel("URLWithString:"), nsstring("https://"));
            if test_url.is_null() {
                return false;
            }
            let app_url = app_for_url(ws, sel("URLForApplicationToOpenURL:"), test_url);
            if app_url.is_null() {
                return false;
            }
            let bundle = url_fn(cls("NSBundle"), sel("bundleWithURL:"), app_url);
            if bundle.is_null() {
                return false;
            }
            let bid_obj = ws_fn(bundle, sel("bundleIdentifier"));
            if let Some(bid) = nsstring_to_rust(bid_obj) {
                bid.eq_ignore_ascii_case(BROWSIFY_BUNDLE_ID)
            } else {
                false
            }
        }
    }

    pub fn request_set_default_browser() -> bool {
        unsafe {
            let ws_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let main_bundle = ws_fn(cls("NSBundle"), sel("mainBundle"));
            if main_bundle.is_null() {
                return false;
            }
            let app_url = ws_fn(main_bundle, sel("bundleURL"));
            if app_url.is_null() {
                return false;
            }
            let path_obj = ws_fn(app_url, sel("path"));
            let Some(path_str) = nsstring_to_rust(path_obj) else {
                return false;
            };
            if !path_str.contains(".app") {
                return false;
            }

            let ws = ws_fn(cls("NSWorkspace"), sel("sharedWorkspace"));
            let set_def: unsafe extern "C" fn(Id, Sel, Id, Id, *const c_void) =
                std::mem::transmute(objc_msgSend as *const c_void);
            set_def(
                ws,
                sel("setDefaultApplicationAtURL:toOpenURLsWithScheme:completionHandler:"),
                app_url,
                nsstring("http"),
                ptr::null(),
            );
            set_def(
                ws,
                sel("setDefaultApplicationAtURL:toOpenURLsWithScheme:completionHandler:"),
                app_url,
                nsstring("https"),
                ptr::null(),
            );
            true
        }
    }

    pub fn lookup_window_title_ax(pid: i32) -> Option<String> {
        unsafe {
            let app_el = AXUIElementCreateApplication(pid);
            if app_el.is_null() {
                return None;
            }
            let mut focused_win: *mut c_void = ptr::null_mut();
            let err = AXUIElementCopyAttributeValue(
                app_el,
                nsstring("AXFocusedWindow") as *const c_void,
                &mut focused_win,
            );
            let mut result = None;
            if err == 0 && !focused_win.is_null() {
                let mut title_val: *mut c_void = ptr::null_mut();
                let err2 = AXUIElementCopyAttributeValue(
                    focused_win,
                    nsstring("AXTitle") as *const c_void,
                    &mut title_val,
                );
                if err2 == 0 && !title_val.is_null() {
                    result = nsstring_to_rust(title_val);
                    CFRelease(title_val);
                }
                CFRelease(focused_win);
            }
            CFRelease(app_el);
            result
        }
    }

    extern "C" fn handle_get_url_event(
        _self: Id,
        _cmd: Sel,
        event: Id,
        _reply_event: Id,
    ) {
        let Some(ctx) = DAEMON_CTX.get() else {
            return;
        };
        unsafe {
            let desc_for_kw: unsafe extern "C" fn(Id, Sel, u32) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let str_val: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let int32_val: unsafe extern "C" fn(Id, Sel) -> i32 =
                std::mem::transmute(objc_msgSend as *const c_void);

            // keyDirectObject = '----' (0x2d2d2d2d), keySenderPIDAttr = 'spid' (0x73706964)
            let direct_desc = desc_for_kw(event, sel("paramDescriptorForKeyword:"), 0x2d2d2d2d);
            if direct_desc.is_null() {
                return;
            }
            let Some(url_str) = nsstring_to_rust(str_val(direct_desc, sel("stringValue"))) else {
                return;
            };

            let pid_desc = desc_for_kw(event, sel("attributeDescriptorForKeyword:"), 0x73706964);
            let pid = if !pid_desc.is_null() {
                int32_val(pid_desc, sel("int32Value"))
            } else {
                0
            };

            // Native `opener_window_title` matchers need the title eagerly; dynamic
            // JS callbacks fetch it lazily via `__browsifyHostGetWindowTitle` instead.
            let needs_native_title = ctx
                .engine
                .lock()
                .map(|e| e.summary().uses_window_title)
                .unwrap_or(false);

            let opener = if pid > 0 {
                let run_app_fn: unsafe extern "C" fn(Class, Sel, i32) -> Id =
                    std::mem::transmute(objc_msgSend as *const c_void);
                let app = run_app_fn(
                    cls("NSRunningApplication"),
                    sel("runningApplicationWithProcessIdentifier:"),
                    pid,
                );
                if !app.is_null() {
                    let name =
                        nsstring_to_rust(str_val(app, sel("localizedName"))).unwrap_or_default();
                    let bundle_id =
                        nsstring_to_rust(str_val(app, sel("bundleIdentifier"))).unwrap_or_default();
                    let bundle_url = str_val(app, sel("bundleURL"));
                    let path = if !bundle_url.is_null() {
                        nsstring_to_rust(str_val(bundle_url, sel("path"))).unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let window_title = if needs_native_title {
                        lookup_window_title_ax(pid)
                    } else {
                        None
                    };
                    Some(OpenerInfo {
                        name,
                        bundle_id,
                        path,
                        window_title,
                        pid,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            handle_daemon_url_event(
                &ctx.engine,
                &ctx.resolver,
                &url_str,
                opener,
                false,
                ctx.dry_run,
            );
            rebuild_status_menu();
        }
    }

    extern "C" fn app_open_file(_self: Id, _cmd: Sel, _app: Id, filename: Id) -> bool {
        let Some(ctx) = DAEMON_CTX.get() else {
            return false;
        };
        unsafe {
            let Some(path_str) = nsstring_to_rust(filename) else {
                return false;
            };
            let Ok(file_url) = file_path_to_url(&path_str) else {
                return false;
            };
            handle_daemon_url_event(&ctx.engine, &ctx.resolver, &file_url, None, false, ctx.dry_run);
            rebuild_status_menu();
            true
        }
    }

    extern "C" fn app_will_continue_user_activity(
        _self: Id,
        _cmd: Sel,
        _app: Id,
        activity_type: Id,
    ) -> bool {
        unsafe {
            nsstring_to_rust(activity_type)
                .map(|t| t == "NSUserActivityTypeBrowsingWeb")
                .unwrap_or(false)
        }
    }

    extern "C" fn app_continue_user_activity(
        _self: Id,
        _cmd: Sel,
        _app: Id,
        user_activity: Id,
        _restoration_handler: Id,
    ) -> bool {
        let Some(ctx) = DAEMON_CTX.get() else {
            return false;
        };
        unsafe {
            let get_id: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            if user_activity.is_null() {
                return false;
            }
            let act_type = get_id(user_activity, sel("activityType"));
            if nsstring_to_rust(act_type).as_deref() != Some("NSUserActivityTypeBrowsingWeb") {
                return false;
            }
            let webpage_url = get_id(user_activity, sel("webpageURL"));
            if webpage_url.is_null() {
                return false;
            }
            let abs_str = get_id(webpage_url, sel("absoluteString"));
            let Some(url_str) = nsstring_to_rust(abs_str) else {
                return false;
            };
            handle_daemon_url_event(&ctx.engine, &ctx.resolver, &url_str, None, false, ctx.dry_run);
            rebuild_status_menu();
            true
        }
    }

    extern "C" fn show_ui_action(_self: Id, _cmd: Sel, _sender: Id) {
        if let Some(ctx) = DAEMON_CTX.get() {
            spawn_or_open_ui_server(Arc::clone(&ctx.engine), Arc::clone(&ctx.resolver));
        }
    }

    extern "C" fn set_default_browser_action(_self: Id, _cmd: Sel, _sender: Id) {
        let _ = request_set_default_browser();
    }

    extern "C" fn edit_config_action(_self: Id, _cmd: Sel, _sender: Id) {
        if let Some(ctx) = DAEMON_CTX.get() {
            if let Ok(eng) = ctx.engine.lock() {
                if let Some(ref p) = eng.summary().config_path {
                    let _ = std::process::Command::new("open").arg(p).spawn();
                }
            }
        }
    }

    extern "C" fn reload_config_action(_self: Id, _cmd: Sel, _sender: Id) {
        if let Some(ctx) = DAEMON_CTX.get() {
            if let Ok(mut eng) = ctx.engine.lock() {
                match eng.reload() {
                    Ok(()) => {
                        log_event(LogLevel::Info, "Configuration reloaded via menu bar");
                        notify_config_reloaded(&eng);
                    }
                    Err(e) => {
                        log_event(LogLevel::Error, format!("Reload error: {}", e));
                    }
                }
            }
            crate::update::trigger_update_check_if_enabled(&ctx.engine, ctx.resolver.home_dir());
            rebuild_status_menu();
        }
    }

    extern "C" fn app_will_finish_launching(_self: Id, _cmd: Sel, _notif: Id) {
        unsafe {
            let shared_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let set_handler: unsafe extern "C" fn(Id, Sel, Id, Sel, u32, u32) =
                std::mem::transmute(objc_msgSend as *const c_void);

            let aem = shared_fn(cls("NSAppleEventManager"), sel("sharedAppleEventManager"));
            // kInternetEventClass = 'GURL' (0x4755524c), kAEGetURL = 'GURL' (0x4755524c)
            set_handler(
                aem,
                sel("setEventHandler:andSelector:forEventClass:andEventID:"),
                _self,
                sel("handleGetURLEvent:withReplyEvent:"),
                0x4755524c,
                0x4755524c,
            );
        }
    }

    extern "C" fn app_did_finish_launching(_self: Id, _cmd: Sel, _notif: Id) {
        let Some(ctx) = DAEMON_CTX.get() else {
            return;
        };
        if !check_is_default_browser() {
            let _ = request_set_default_browser();
        }
        let hide_icon = ctx
            .engine
            .lock()
            .map(|e| e.summary().options.hide_icon)
            .unwrap_or(false);
        if !hide_icon {
            create_status_item(_self);
        }
        if ctx.force_window {
            spawn_or_open_ui_server(Arc::clone(&ctx.engine), Arc::clone(&ctx.resolver));
        }
    }

    fn create_status_item(delegate: Id) {
        let Some(ctx) = DAEMON_CTX.get() else {
            return;
        };
        unsafe {
            let shared_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let status_item_fn: unsafe extern "C" fn(Id, Sel, f64) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let retain_fn: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let set_id: unsafe extern "C" fn(Id, Sel, Id) =
                std::mem::transmute(objc_msgSend as *const c_void);

            let bar = shared_fn(cls("NSStatusBar"), sel("systemStatusBar"));
            // NSSquareStatusItemLength = -2.0
            let item = status_item_fn(bar, sel("statusItemWithLength:"), -2.0);
            let _ = retain_fn(item, sel("retain"));

            let button = shared_fn(item, sel("button"));
            if !button.is_null() {
                set_id(button, sel("setTitle:"), nsstring("B"));
                set_id(button, sel("setToolTip:"), nsstring("Browsify"));
            }

            let _ = delegate;
            if let Ok(mut guard) = ctx.status_item.lock() {
                *guard = item as usize;
            }
        }
        rebuild_status_menu();
    }

    fn rebuild_status_menu() {
        let Some(ctx) = DAEMON_CTX.get() else {
            return;
        };
        let item_ptr = ctx.status_item.lock().map(|g| *g).unwrap_or(0) as Id;
        if item_ptr.is_null() {
            return;
        }

        unsafe {
            let alloc_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let init_fn: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let void_fn: unsafe extern "C" fn(Id, Sel) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let add_item_fn: unsafe extern "C" fn(Id, Sel, Id, Sel, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let add_menu_item: unsafe extern "C" fn(Id, Sel, Id) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let set_bool: unsafe extern "C" fn(Id, Sel, bool) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let set_id: unsafe extern "C" fn(Id, Sel, Id) =
                std::mem::transmute(objc_msgSend as *const c_void);

            let menu = init_fn(alloc_fn(cls("NSMenu"), sel("alloc")), sel("init"));

            let mode_label = ctx
                .engine
                .lock()
                .map(|e| {
                    format!(
                        "Browsify v{} — {}",
                        env!("CARGO_PKG_VERSION"),
                        if e.summary().pure_native_fast_path {
                            "Native Fast Path"
                        } else {
                            "Hybrid QuickJS"
                        }
                    )
                })
                .unwrap_or_else(|_| "Browsify".to_string());

            let header = add_item_fn(
                menu,
                sel("addItemWithTitle:action:keyEquivalent:"),
                nsstring(&mode_label),
                ptr::null_mut(),
                nsstring(""),
            );
            set_bool(header, sel("setEnabled:"), false);

            let _ = add_item_fn(
                menu,
                sel("addItemWithTitle:action:keyEquivalent:"),
                nsstring("Rules & URL Tester…"),
                sel("showUiAction:"),
                nsstring(","),
            );

            // Recent URLs submenu (up to 10 items)
            let recent = get_recent_routed_urls();
            if !recent.is_empty() {
                let recent_item = add_item_fn(
                    menu,
                    sel("addItemWithTitle:action:keyEquivalent:"),
                    nsstring("Recent URLs"),
                    ptr::null_mut(),
                    nsstring(""),
                );
                let sub = init_fn(alloc_fn(cls("NSMenu"), sel("alloc")), sel("init"));
                for entry in recent {
                    let target_desc = if entry.profile.is_empty() {
                        entry.browser.clone()
                    } else {
                        format!("{} ({})", entry.browser, entry.profile)
                    };
                    let label = format!("{} → {}", entry.input_url, target_desc);
                    let row = add_item_fn(
                        sub,
                        sel("addItemWithTitle:action:keyEquivalent:"),
                        nsstring(&label),
                        ptr::null_mut(),
                        nsstring(""),
                    );
                    set_bool(row, sel("setEnabled:"), false);
                }
                set_id(recent_item, sel("setSubmenu:"), sub);
                void_fn(sub, sel("release"));
            }

            if !check_is_default_browser() {
                let _ = add_item_fn(
                    menu,
                    sel("addItemWithTitle:action:keyEquivalent:"),
                    nsstring("Set as Default Browser"),
                    sel("setDefaultBrowserAction:"),
                    nsstring(""),
                );
            }

            let _ = add_item_fn(
                menu,
                sel("addItemWithTitle:action:keyEquivalent:"),
                nsstring("Edit Config…"),
                sel("editConfigAction:"),
                nsstring("e"),
            );
            let _ = add_item_fn(
                menu,
                sel("addItemWithTitle:action:keyEquivalent:"),
                nsstring("Reload Config"),
                sel("reloadConfigAction:"),
                nsstring("r"),
            );

            let sep = init_fn(cls("NSMenuItem"), sel("separatorItem"));
            add_menu_item(menu, sel("addItem:"), sep);

            let _ = add_item_fn(
                menu,
                sel("addItemWithTitle:action:keyEquivalent:"),
                nsstring("Quit Browsify"),
                sel("terminate:"),
                nsstring("q"),
            );

            set_id(item_ptr, sel("setMenu:"), menu);
            void_fn(menu, sel("release"));
        }
    }

    pub fn run_nsapplication_loop(
        engine: Arc<Mutex<BrowsifyEngine>>,
        resolver: Arc<ProfileResolver>,
        force_window: bool,
        dry_run: bool,
    ) {
        let _ = DAEMON_CTX.set(DaemonContext {
            engine,
            resolver,
            force_window,
            dry_run,
            status_item: Mutex::new(0),
        });

        unsafe {
            let ns_object = cls("NSObject");
            let delegate_name = CString::new("BrowsifyAppDelegate").unwrap();
            let delegate_cls = objc_allocateClassPair(ns_object, delegate_name.as_ptr(), 0);
            if !delegate_cls.is_null() {
                class_addMethod(
                    delegate_cls,
                    sel("applicationWillFinishLaunching:"),
                    app_will_finish_launching as Imp,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("applicationDidFinishLaunching:"),
                    app_did_finish_launching as Imp,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("handleGetURLEvent:withReplyEvent:"),
                    handle_get_url_event as Imp,
                    c"v@:@@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("application:openFile:"),
                    app_open_file as Imp,
                    c"B@:@@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("application:willContinueUserActivityWithType:"),
                    app_will_continue_user_activity as Imp,
                    c"B@:@@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("application:continueUserActivity:restorationHandler:"),
                    app_continue_user_activity as Imp,
                    c"B@:@@@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("showUiAction:"),
                    show_ui_action as Imp,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("setDefaultBrowserAction:"),
                    set_default_browser_action as Imp,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("editConfigAction:"),
                    edit_config_action as Imp,
                    c"v@:@".as_ptr(),
                );
                class_addMethod(
                    delegate_cls,
                    sel("reloadConfigAction:"),
                    reload_config_action as Imp,
                    c"v@:@".as_ptr(),
                );
                objc_registerClassPair(delegate_cls);
            }

            let shared_fn: unsafe extern "C" fn(Class, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let init_fn: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const c_void);
            let set_id: unsafe extern "C" fn(Id, Sel, Id) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let void_fn: unsafe extern "C" fn(Id, Sel) =
                std::mem::transmute(objc_msgSend as *const c_void);

            let app = shared_fn(cls("NSApplication"), sel("sharedApplication"));
            let del_cls = cls("BrowsifyAppDelegate");
            let delegate = init_fn(shared_fn(del_cls, sel("alloc")), sel("init"));
            set_id(app, sel("setDelegate:"), delegate);
            void_fn(app, sel("finishLaunching"));
            void_fn(app, sel("run"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_path_to_url_percent_encodes_special_characters() {
        let url = file_path_to_url("/tmp/Report #1?.html").unwrap();
        assert_eq!(url, "file:///tmp/Report%20%231%3F.html");
    }
}
