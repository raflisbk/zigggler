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
