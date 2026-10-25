#[cfg(target_os = "windows")]
pub use windows::WindowsInhibitor;

#[cfg(target_os = "windows")]
mod windows {
    use ::windows::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    /// `SetThreadExecutionState` while the jiggler is on — parallel to
    /// Linux's `ScreenSaverInhibitor`. IMPORTANT: this does NOT prevent a
    /// user-configured screensaver from kicking in — `ES_DISPLAY_REQUIRED`/
    /// `ES_SYSTEM_REQUIRED` only stop the display/system from sleeping.
    /// Screensaver and sleep are genuinely separate mechanisms on Windows,
    /// same class of limitation as `org.freedesktop.ScreenSaver.Inhibit` on
    /// Linux not resetting the idle counter.
    pub struct WindowsInhibitor;

    impl WindowsInhibitor {
        pub fn new() -> Self {
            Self
        }

        pub fn inhibit(&self) {
            let flags = ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED;
            if unsafe { SetThreadExecutionState(flags) }.0 == 0 {
                eprintln!("zzzzziglerrr[windows_inhibit] SetThreadExecutionState FAILED (inhibit)");
            } else {
                eprintln!("zzzzziglerrr[windows_inhibit] inhibit ok");
            }
        }

        pub fn uninhibit(&self) {
            if unsafe { SetThreadExecutionState(ES_CONTINUOUS) }.0 == 0 {
                eprintln!(
                    "zzzzziglerrr[windows_inhibit] SetThreadExecutionState FAILED (uninhibit)"
                );
            } else {
                eprintln!("zzzzziglerrr[windows_inhibit] uninhibit ok");
            }
        }
    }

    impl Drop for WindowsInhibitor {
        fn drop(&mut self) {
            self.uninhibit();
        }
    }
}
