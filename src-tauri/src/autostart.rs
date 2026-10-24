#[cfg(target_os = "linux")]
pub use linux::{disable, enable};

#[cfg(target_os = "linux")]
mod linux {
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    const DESKTOP_FILE_NAME: &str = "zzzzziglerrr.desktop";

    fn autostart_path() -> io::Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| io::Error::other("no config dir for this platform"))?
            .join("autostart");
        Ok(dir.join(DESKTOP_FILE_NAME))
    }

    fn exec_path() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.to_str().map(str::to_string))
            .unwrap_or_else(|| "zzzzziglerrr".into())
    }

    /// Writes `~/.config/autostart/zzzzziglerrr.desktop` so the session
    /// starts the app on login — standard freedesktop.org autostart spec,
    /// read by KDE/GNOME/most other DEs.
    pub fn enable() -> io::Result<()> {
        let path = autostart_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=zzzzziglerrr\n\
             Exec={}\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            exec_path()
        );
        fs::write(path, contents)
    }

    /// Removes the autostart entry. Missing file is not an error — toggling
    /// off twice (or off on a fresh install) is a no-op, not a failure.
    pub fn disable() -> io::Result<()> {
        match fs::remove_file(autostart_path()?) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::{disable, enable};

#[cfg(target_os = "windows")]
mod windows {
    use std::io;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    const RUN_KEY_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "zzzzziglerrr";

    /// Quoted so Windows doesn't try to resolve an unquoted path containing
    /// spaces (e.g. `C:\Program Files\...`) as several candidate binaries
    /// (classic autostart/service "binary planting" vector) — propagates
    /// an error instead of silently falling back to a bare unqualified
    /// name if `current_exe()` fails.
    fn exec_path() -> io::Result<String> {
        let path = std::env::current_exe()?;
        let path = path
            .to_str()
            .ok_or_else(|| io::Error::other("executable path isn't valid UTF-8"))?;
        Ok(format!("\"{path}\""))
    }

    /// Same interface as the Linux `.desktop` arm, backed by the standard
    /// per-user autostart registry key instead of a file.
    pub fn enable() -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run_key, _) = hkcu.create_subkey(RUN_KEY_PATH)?;
        run_key.set_value(VALUE_NAME, &exec_path()?)
    }

    /// Missing key/value is not an error, same reasoning as the Linux arm.
    pub fn disable() -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_SET_VALUE) {
            Ok(run_key) => match run_key.delete_value(VALUE_NAME) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(err),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}
