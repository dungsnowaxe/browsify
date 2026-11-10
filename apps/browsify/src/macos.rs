use browsify_core::ModifierKeys;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io;
use std::path::PathBuf;

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

/// Reads global session modifier key state via `CGEventSourceFlagsState` on macOS
/// (which works inside `LSUIElement` background agents where `[NSEvent modifierFlags]` returns 0).
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
