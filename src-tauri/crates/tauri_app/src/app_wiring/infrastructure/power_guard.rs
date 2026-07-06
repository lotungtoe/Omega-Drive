use std::sync::atomic::{AtomicUsize, Ordering};

pub struct PowerGuard {
    count: AtomicUsize,
}

impl PowerGuard {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn acquire(&self) {
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            set_sleep_block(true);
        }
    }

    pub fn release(&self) {
        let prev = self.count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            self.count.store(0, Ordering::SeqCst);
            return;
        }
        if prev == 1 {
            set_sleep_block(false);
        }
    }
}

impl Default for PowerGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
fn set_sleep_block(block: bool) {
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
    };
    // SAFETY: SetThreadExecutionState is a process-local Win32 API with no raw
    // pointers here. We only toggle the documented execution-state flags.
    unsafe {
        if block {
            let _ = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        } else {
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_sleep_block(_block: bool) {}
