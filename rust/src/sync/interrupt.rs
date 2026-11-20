//! Ctrl+C for `fa sync`, with no new crate: a small handler that sets a flag. The run reads the flag between two items and
//! the request queues read it before each request, so the run ends at the next request with all files in a correct
//! state (each cache file is written in one step: the old file stays or the new file is complete).
//!
//! Unix (Linux, macOS): `signal(SIGINT)` of the C library. Windows (PowerShell, cmd): `SetConsoleCtrlHandler` of kernel32.
//! Other systems: no handler (Ctrl+C ends the program at once, as before).

use std::sync::atomic::{AtomicBool, Ordering};

static FLAG: AtomicBool = AtomicBool::new(false);

/// The flag that the handler sets.
pub fn flag() -> &'static AtomicBool {
    &FLAG
}

#[cfg(unix)]
mod os {
    use super::*;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;

    extern "C" fn on_signal(_: i32) {
        // Only an atomic store: this is allowed in a signal handler.
        FLAG.store(true, Ordering::SeqCst);
        crate::hostnet::request_stop();
    }

    pub fn install() {
        // SAFETY: `signal` is the function of the C library; the handler does an atomic store only.
        unsafe {
            signal(SIGINT, on_signal as extern "C" fn(i32) as usize);
            signal(SIGTERM, on_signal as extern "C" fn(i32) as usize);
        }
    }
}

#[cfg(windows)]
mod os {
    use super::*;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }

    unsafe extern "system" fn on_ctrl(kind: u32) -> i32 {
        // CTRL_C_EVENT 0, CTRL_BREAK_EVENT 1, CTRL_CLOSE_EVENT 2.
        if kind <= 2 {
            FLAG.store(true, Ordering::SeqCst);
            crate::hostnet::request_stop();
            1
        } else {
            0
        }
    }

    pub fn install() {
        // SAFETY: the handler does an atomic store only.
        unsafe {
            SetConsoleCtrlHandler(Some(on_ctrl), 1);
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod os {
    pub fn install() {}
}

/// Install the handler (once is enough; a second call does the same again).
pub fn install() {
    os::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_starts_clear() {
        assert!(!flag().load(Ordering::SeqCst));
    }
}
