pub trait MouseInjector: Send + Sync {
    fn jiggle(&self);
}

#[cfg(target_os = "linux")]
pub use linux::UinputInjector;

#[cfg(target_os = "linux")]
mod linux {
    use super::MouseInjector;
    use std::sync::Mutex;

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
                Err(_) => return,
            };
            // Nudge 2px then back — enough for idle/presence detection, not visible on screen.
            let _ = device.send(uinput::event::relative::Position::X, 2);
            let _ = device.send(uinput::event::relative::Position::Y, 2);
            let _ = device.synchronize();
            let _ = device.send(uinput::event::relative::Position::X, -2);
            let _ = device.send(uinput::event::relative::Position::Y, -2);
            let _ = device.synchronize();
        }
    }
}
