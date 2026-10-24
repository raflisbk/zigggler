pub trait MouseInjector: Send + Sync {
    fn jiggle(&self);
}

#[cfg(target_os = "linux")]
pub use linux::UinputInjector;

#[cfg(target_os = "linux")]
mod linux {
    use super::MouseInjector;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn log_ts(msg: &str) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        eprintln!("zzzzziglerrr[mouse_injector] t={secs} {msg}");
    }

    pub struct UinputInjector {
        device: Mutex<uinput::Device>,
    }

    impl UinputInjector {
        pub fn new() -> Result<Self, uinput::Error> {
            let device = uinput::default()?
                .name("zzzzziglerrr-jiggler")?
                .event(uinput::event::relative::Relative::Position(
                    uinput::event::relative::Position::X,
                ))?
                .event(uinput::event::relative::Relative::Position(
                    uinput::event::relative::Position::Y,
                ))?
                .create()?;
            Ok(Self {
                device: Mutex::new(device),
            })
        }
    }

    impl MouseInjector for UinputInjector {
        fn jiggle(&self) {
            let mut device = match self.device.lock() {
                Ok(device) => device,
                Err(_) => {
                    log_ts("jiggle SKIPPED, device mutex poisoned");
                    return;
                }
            };
            // Nudge 2px then back — enough for idle/presence detection, not visible on screen.
            let result = (|| -> Result<(), uinput::Error> {
                device.send(uinput::event::relative::Position::X, 2)?;
                device.send(uinput::event::relative::Position::Y, 2)?;
                device.synchronize()?;
                device.send(uinput::event::relative::Position::X, -2)?;
                device.send(uinput::event::relative::Position::Y, -2)?;
                device.synchronize()?;
                Ok(())
            })();
            match result {
                Ok(()) => log_ts("jiggle sent ok"),
                Err(err) => log_ts(&format!("jiggle FAILED: {err}")),
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::SendInputInjector;

#[cfg(target_os = "windows")]
mod windows {
    use super::MouseInjector;
    use ::windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn log_ts(msg: &str) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        eprintln!("zzzzziglerrr[mouse_injector] t={secs} {msg}");
    }

    fn relative_move(dx: i32, dy: i32) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// `SendInput` relative-move nudge — same ±2px-then-back pattern as the
    /// Linux `UinputInjector`. Stateless (no persistent device handle like
    /// uinput needs).
    pub struct SendInputInjector;

    impl SendInputInjector {
        pub fn new() -> Self {
            Self
        }
    }

    impl MouseInjector for SendInputInjector {
        fn jiggle(&self) {
            let inputs = [relative_move(2, 2), relative_move(-2, -2)];
            let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
            if sent as usize == inputs.len() {
                log_ts("jiggle sent ok");
            } else {
                log_ts("jiggle FAILED: SendInput reported fewer events sent than expected");
            }
        }
    }
}
