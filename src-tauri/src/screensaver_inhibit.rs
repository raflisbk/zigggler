#[cfg(target_os = "linux")]
pub use linux::ScreenSaverInhibitor;

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::Mutex;
    use zbus::blocking::{Connection, Proxy};

    const APP_NAME: &str = "zzzzziglerrr";
    const REASON: &str = "jiggler nyala, nahan screensaver/DPMS biar gak mati sendiri";

    /// `org.freedesktop.ScreenSaver.Inhibit` while the jiggler is on — belt-and-
    /// suspenders alongside the synthetic uinput activity, since a long real
    /// idle session showed KWin's idle tracking (ext-idle-notify-v1 *and*
    /// DPMS) stops counting synthetic input as activity after a while.
    pub struct ScreenSaverInhibitor {
        state: Mutex<Option<(Connection, u32)>>,
    }

    impl ScreenSaverInhibitor {
        pub fn new() -> Self {
            Self {
                state: Mutex::new(None),
            }
        }

        fn proxy(conn: &Connection) -> zbus::Result<Proxy<'_>> {
            Proxy::new(
                conn,
                "org.freedesktop.ScreenSaver",
                "/ScreenSaver",
                "org.freedesktop.ScreenSaver",
            )
        }

        pub fn inhibit(&self) {
            let mut state = self.state.lock().expect("inhibitor mutex poisoned");
            if state.is_some() {
                return; // already inhibiting
            }
            let result = (|| -> zbus::Result<(Connection, u32)> {
                let conn = Connection::session()?;
                let cookie: u32 = {
                    let proxy = Self::proxy(&conn)?;
                    proxy.call("Inhibit", &(APP_NAME, REASON))?
                };
                Ok((conn, cookie))
            })();
            match result {
                Ok((conn, cookie)) => {
                    eprintln!("zzzzziglerrr[screensaver_inhibit] Inhibit ok, cookie={cookie}");
                    *state = Some((conn, cookie));
                }
                Err(err) => {
                    eprintln!("zzzzziglerrr[screensaver_inhibit] Inhibit FAILED: {err}");
                }
            }
        }

        pub fn uninhibit(&self) {
            let mut state = self.state.lock().expect("inhibitor mutex poisoned");
            if let Some((conn, cookie)) = state.take() {
                match Self::proxy(&conn)
                    .and_then(|proxy| proxy.call::<_, _, ()>("UnInhibit", &(cookie,)))
                {
                    Ok(()) => {
                        eprintln!("zzzzziglerrr[screensaver_inhibit] UnInhibit ok, cookie={cookie}")
                    }
                    Err(err) => {
                        eprintln!("zzzzziglerrr[screensaver_inhibit] UnInhibit FAILED: {err}")
                    }
                }
            }
        }
    }

    impl Drop for ScreenSaverInhibitor {
        fn drop(&mut self) {
            self.uninhibit();
        }
    }
}
