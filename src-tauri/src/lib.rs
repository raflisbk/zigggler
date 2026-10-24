mod autostart;
mod config;
mod idle_watcher;
mod kwin_idle_simulate;
mod mouse_injector;
mod scheduler;
mod screensaver_inhibit;
mod windows_inhibit;

use config::Config;
use mouse_injector::MouseInjector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ID: &str = "main-tray";
const MAIN_WINDOW: &str = "main";

#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Off,
    Waiting,
    Jiggling,
    /// On a random break — distinct from `Off` (toggle is still on, this is
    /// temporary) and from `Waiting` (not gated on idle, gated on the break
    /// schedule).
    Break,
}

struct AppState {
    config: Arc<Mutex<Config>>,
    jiggling: Arc<AtomicBool>,
    /// Set just before the "Keluar" tray item calls `app.exit(0)` — lets
    /// `RunEvent::ExitRequested` tell a genuine quit apart from Tauri's
    /// default "no window visible, exit the process" behavior that we
    /// otherwise suppress to hide-to-tray instead (see `.run()` below). On
    /// at least Windows, `app.exit(0)` still routes through
    /// `ExitRequested` — without this flag, the old unconditional
    /// `prevent_exit()` blocked genuine quits too.
    quitting: Arc<AtomicBool>,
    tray_state: Arc<Mutex<TrayState>>,
    app_handle: AppHandle,
    jiggle_menu_item: CheckMenuItem<tauri::Wry>,
    autostart_menu_item: CheckMenuItem<tauri::Wry>,
    random_breaks_menu_item: CheckMenuItem<tauri::Wry>,
    #[cfg(target_os = "linux")]
    screensaver_inhibitor: Arc<screensaver_inhibit::ScreenSaverInhibitor>,
    #[cfg(target_os = "windows")]
    windows_inhibitor: Arc<windows_inhibit::WindowsInhibitor>,
}

#[derive(serde::Serialize)]
struct Status {
    jiggling: bool,
    autostart: bool,
    random_breaks: bool,
}

fn log_toggle(jiggling: bool) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    eprintln!("zzzzziglerrr[toggle] t={secs} jiggling={jiggling}");
}

/// Shared by the `toggle_jiggle` command and the tray menu's "Toggle Jiggle"
/// checkbox so both paths flip the same state the same way.
fn do_toggle_jiggle(state: &AppState) -> bool {
    let now = !state.jiggling.load(Ordering::SeqCst);
    state.jiggling.store(now, Ordering::SeqCst);
    log_toggle(now);
    let next_state = if now { TrayState::Waiting } else { TrayState::Off };
    *state.tray_state.lock().expect("tray state mutex poisoned") = next_state;
    let theme = state.config.lock().expect("config mutex poisoned").theme.clone();
    refresh_tray(&state.app_handle, next_state, &theme);
    let _ = state.jiggle_menu_item.set_checked(now);

    #[cfg(target_os = "linux")]
    {
        if now {
            state.screensaver_inhibitor.inhibit();
        } else {
            state.screensaver_inhibitor.uninhibit();
        }
    }
    #[cfg(target_os = "windows")]
    {
        if now {
            state.windows_inhibitor.inhibit();
        } else {
            state.windows_inhibitor.uninhibit();
        }
    }

    // So any open window's switch updates live regardless of which path
    // (command or tray menu) triggered the toggle — fixes tray/dashboard
    // getting out of sync.
    let _ = state.app_handle.emit("jiggle-changed", now);

    now
}

/// Shared by the `toggle_autostart` command and the tray menu's "Autostart"
/// checkbox — writes/removes the platform autostart entry (`.desktop` file
/// on Linux, registry Run key on Windows) to match the new state, persists
/// it in `Config`, and keeps the tray checkbox + any open window's switch
/// in sync.
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn do_toggle_autostart(state: &AppState) -> bool {
    let now = {
        let mut config = state.config.lock().expect("config mutex poisoned");
        config.autostart = !config.autostart;
        let now = config.autostart;
        let _ = config.save();
        now
    };

    let result = if now {
        autostart::enable()
    } else {
        autostart::disable()
    };
    if let Err(err) = result {
        eprintln!("zzzzziglerrr[autostart] gagal update .desktop: {err}");
    }

    let _ = state.autostart_menu_item.set_checked(now);
    let _ = state.app_handle.emit("autostart-changed", now);
    now
}

#[tauri::command]
fn toggle_autostart(state: tauri::State<AppState>) -> bool {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    return do_toggle_autostart(&state);
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = state;
        false
    }
}

/// Shared by the `toggle_random_breaks` command and the tray menu's "Jeda
/// Acak" checkbox. Only flips the config flag + UI state — the jiggle loop
/// thread reads `Config.random_breaks` each tick and owns the actual
/// `RandomBreakScheduler` instance, same split as `idle_only`.
fn do_toggle_random_breaks(state: &AppState) -> bool {
    let now = {
        let mut config = state.config.lock().expect("config mutex poisoned");
        config.random_breaks = !config.random_breaks;
        let now = config.random_breaks;
        let _ = config.save();
        now
    };
    let _ = state.random_breaks_menu_item.set_checked(now);
    let _ = state.app_handle.emit("random-breaks-changed", now);
    now
}

#[tauri::command]
fn toggle_random_breaks(state: tauri::State<AppState>) -> bool {
    do_toggle_random_breaks(&state)
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Config {
    state.config.lock().expect("config mutex poisoned").clone()
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> Status {
    let config = state.config.lock().expect("config mutex poisoned");
    Status {
        jiggling: state.jiggling.load(Ordering::SeqCst),
        autostart: config.autostart,
        random_breaks: config.random_breaks,
    }
}

#[tauri::command]
fn set_config(state: tauri::State<AppState>, config: Config) -> Result<(), String> {
    config.save().map_err(|err| err.to_string())?;
    let theme = config.theme.clone();
    *state.config.lock().expect("config mutex poisoned") = config;
    let current = *state.tray_state.lock().expect("tray state mutex poisoned");
    refresh_tray(&state.app_handle, current, &theme);
    Ok(())
}

#[tauri::command]
fn toggle_jiggle(state: tauri::State<AppState>) -> bool {
    do_toggle_jiggle(&state)
}

// Swatch per tema — duplikat dari THEMES di src/index.html karena tray icon
// di-render Rust-side, gak bisa import JS. Kalau nambah/ubah tema di frontend,
// update juga tabel ini biar warna tray nyambung.
fn theme_swatch(theme: &str) -> (u8, u8, u8) {
    match theme {
        "game-boy" => (0x0F, 0x38, 0x0F),
        "nes-arcade" => (0xE6, 0x00, 0x12),
        "dos-phosphor" => (0xFF, 0xB0, 0x00),
        "mercury" => (0x4F, 0xC3, 0xF7),
        "gold" => (0xD4, 0xAF, 0x6A),
        "obsidian-glass" => (0x7C, 0x5C, 0xFF),
        "liquid-glass" => (0x2B, 0xA3, 0x93),
        "holographic" => (0xB6, 0xA4, 0xF7),
        "sketchbook" => (0xD1, 0x49, 0x5B),
        "pastel-blob" => (0xE9, 0x80, 0x74),
        "comic-outline" => (0xFF, 0xD2, 0x3F),
        _ => (0x5F, 0xD3, 0xC4),
    }
}

static BRAND_ICON_BYTES: &[u8] = include_bytes!("../icons/32x32.png");

/// Badge color/alpha per state, drawn as a small dot in the bottom-right
/// corner of the brand icon — off/waiting/jiggling is communicated by the
/// badge, not by replacing the whole icon with a plain dot.
fn badge_for(state: TrayState, theme: &str) -> Option<(u8, u8, u8, u8)> {
    match state {
        TrayState::Off => None,
        TrayState::Waiting => {
            let (r, g, b) = theme_swatch(theme);
            Some((r, g, b, 150))
        }
        TrayState::Jiggling => {
            let (r, g, b) = theme_swatch(theme);
            Some((r, g, b, 255))
        }
        // Neutral gray, independent of theme color, so a break reads as
        // visually distinct from both "off" (dimmed whole icon) and
        // "jiggling"/"waiting" (theme-colored badge).
        TrayState::Break => Some((0x90, 0x90, 0x90, 220)),
    }
}

fn tray_icon_for(state: TrayState, theme: &str) -> Image<'static> {
    let mut icon = image::load_from_memory(BRAND_ICON_BYTES)
        .expect("bundled tray icon is a valid PNG")
        .to_rgba8();
    let (width, height) = icon.dimensions();

    if state == TrayState::Off {
        // Dim the whole icon so "off" reads as inactive at a glance.
        for pixel in icon.pixels_mut() {
            pixel[3] = (pixel[3] as u16 * 140 / 255) as u8;
        }
    }

    if let Some((r, g, b, a)) = badge_for(state, theme) {
        let badge_radius = (width.min(height) as f32 * 0.22).max(3.0);
        let cx = width as f32 - badge_radius - 1.0;
        let cy = height as f32 - badge_radius - 1.0;
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= badge_radius * badge_radius {
                    icon.put_pixel(x, y, image::Rgba([r, g, b, a]));
                }
            }
        }
    }

    Image::new_owned(icon.into_raw(), width, height)
}

fn refresh_tray(app_handle: &AppHandle, state: TrayState, theme: &str) {
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_icon_for(state, theme)));
    }
}

fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn log_tick(idle_only: bool, idle_seconds: u64, gate_open: bool) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    eprintln!(
        "zzzzziglerrr[jiggle_loop] t={secs} tick idle_only={idle_only} idle_seconds={idle_seconds} gate_open={gate_open}"
    );
}

/// Linux: `UinputInjector`, infallible-ish (only fails if `/dev/uinput`
/// isn't accessible — udev rule missing). Windows: `SendInputInjector`,
/// stateless and always succeeds, no device handle to create.
#[cfg(target_os = "linux")]
fn build_injector() -> Option<Box<dyn MouseInjector>> {
    use mouse_injector::UinputInjector;
    match UinputInjector::new() {
        Ok(injector) => Some(Box::new(injector)),
        Err(err) => {
            eprintln!("zzzzziglerrr: gagal bikin uinput device: {err}");
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn build_injector() -> Option<Box<dyn MouseInjector>> {
    use mouse_injector::SendInputInjector;
    Some(Box::new(SendInputInjector::new()))
}

#[cfg(target_os = "linux")]
fn build_idle_watcher() -> Option<Box<dyn idle_watcher::IdleWatcher>> {
    use idle_watcher::WaylandIdleWatcher;
    let watcher = WaylandIdleWatcher::spawn();
    if watcher.is_none() {
        eprintln!("zzzzziglerrr: compositor gak dukung ext-idle-notify-v1, idle_only diabaikan");
    }
    watcher.map(|w| Box::new(w) as Box<dyn idle_watcher::IdleWatcher>)
}

#[cfg(target_os = "windows")]
fn build_idle_watcher() -> Option<Box<dyn idle_watcher::IdleWatcher>> {
    use idle_watcher::WindowsIdleWatcher;
    Some(Box::new(WindowsIdleWatcher::new()))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn spawn_jiggle_loop(
    app_handle: AppHandle,
    jiggling: Arc<AtomicBool>,
    config: Arc<Mutex<Config>>,
    tray_state: Arc<Mutex<TrayState>>,
) {
    std::thread::spawn(move || {
        let Some(injector) = build_injector() else {
            return;
        };
        let idle_watcher = build_idle_watcher();

        // KDE/KWin-specific second signal: confirmed resets the seat
        // activity signal every idle-tracking consumer sees (ActivityWatch
        // included), unlike plain uinput which the compositor doesn't
        // count as real activity. No equivalent exists/is needed on
        // Windows — see idle_watcher::WindowsIdleWatcher's doc comment.
        #[cfg(target_os = "linux")]
        let kwin_idle_simulator = kwin_idle_simulate::KwinIdleSimulator::connect();
        #[cfg(target_os = "linux")]
        if kwin_idle_simulator.is_none() {
            eprintln!("zzzzziglerrr: compositor gak dukung org_kde_kwin_idle, pake uinput doang");
        }

        let mut consecutive_closed_ticks: u64 = 0;
        // Created lazily the first time `random_breaks` is on, then kept
        // alive for the rest of the process — quota history and in-flight
        // breaks persist across the toggle being flipped off and back on
        // (per docs/goals.md), resetting only on process restart. `tick()`
        // simply isn't called while the toggle is off, so time doesn't
        // advance for it during that window either.
        let mut random_break_scheduler: Option<scheduler::RandomBreakScheduler> = None;

        loop {
            let (interval_secs, idle_only, random_breaks, theme) = {
                let config = config.lock().expect("config mutex poisoned");
                (
                    config.interval_secs,
                    config.idle_only,
                    config.random_breaks,
                    config.theme.clone(),
                )
            };
            std::thread::sleep(Duration::from_secs(interval_secs.max(1)));

            if !jiggling.load(Ordering::SeqCst) {
                continue;
            }
            let idle_seconds = idle_watcher.as_ref().map_or(0, |w| w.idle_seconds());
            let gate_open = scheduler::should_jiggle(idle_only, idle_seconds);
            log_tick(idle_only, idle_seconds, gate_open);

            if gate_open {
                consecutive_closed_ticks = 0;
            } else {
                consecutive_closed_ticks += 1;
                // Every ~2 minutes the gate stays closed, say so loudly —
                // this is the exact signal that would've caught platform-
                // compat's ~40min stuck-gate report sooner.
                let ticks_per_log = (120 / interval_secs.max(1)).max(1);
                if consecutive_closed_ticks % ticks_per_log == 0 {
                    let closed_secs = consecutive_closed_ticks * interval_secs.max(1);
                    eprintln!(
                        "zzzzziglerrr[jiggle_loop] gate CLOSED terus {closed_secs}s berturut-turut (idle_only={idle_only})"
                    );
                }
            }

            // Random-break gate is additional to (not instead of) idle_only
            // — it can suspend jiggling even while the main toggle and
            // idle_only gate both say go.
            let on_break = if random_breaks {
                let now = std::time::Instant::now();
                let sched = random_break_scheduler
                    .get_or_insert_with(|| scheduler::RandomBreakScheduler::new(now));
                sched.tick(now);
                sched.state() == scheduler::BreakState::Break
            } else {
                false
            };

            if gate_open && !on_break {
                *tray_state.lock().expect("tray state mutex poisoned") = TrayState::Jiggling;
                refresh_tray(&app_handle, TrayState::Jiggling, &theme);
                injector.jiggle();
                #[cfg(target_os = "linux")]
                if let Some(simulator) = kwin_idle_simulator.as_ref() {
                    simulator.simulate();
                }
                if jiggling.load(Ordering::SeqCst) {
                    *tray_state.lock().expect("tray state mutex poisoned") = TrayState::Waiting;
                    refresh_tray(&app_handle, TrayState::Waiting, &theme);
                }
            } else if gate_open && on_break {
                *tray_state.lock().expect("tray state mutex poisoned") = TrayState::Break;
                refresh_tray(&app_handle, TrayState::Break, &theme);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = Arc::new(Mutex::new(Config::load()));
            let jiggling = Arc::new(AtomicBool::new(false));
            let quitting = Arc::new(AtomicBool::new(false));
            let tray_state = Arc::new(Mutex::new(TrayState::Off));
            let app_handle = app.handle().clone();

            let jiggle_menu_item =
                CheckMenuItem::with_id(app, "toggle_jiggle", "Toggle Jiggle", true, false, None::<&str>)?;
            let autostart_menu_item = CheckMenuItem::with_id(
                app,
                "toggle_autostart",
                "Autostart",
                true,
                config.lock().expect("config mutex poisoned").autostart,
                None::<&str>,
            )?;
            let random_breaks_menu_item = CheckMenuItem::with_id(
                app,
                "toggle_random_breaks",
                "Jeda Acak",
                true,
                config.lock().expect("config mutex poisoned").random_breaks,
                None::<&str>,
            )?;
            let settings_item =
                MenuItem::with_id(app, "open_settings", "Buka Settings", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &jiggle_menu_item,
                    &autostart_menu_item,
                    &random_breaks_menu_item,
                    &settings_item,
                    &separator,
                    &quit_item,
                ],
            )?;

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_icon_for(TrayState::Off, &config.lock().expect("config mutex poisoned").theme))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle_jiggle" => {
                        do_toggle_jiggle(&app.state::<AppState>());
                    }
                    #[cfg(any(target_os = "linux", target_os = "windows"))]
                    "toggle_autostart" => {
                        do_toggle_autostart(&app.state::<AppState>());
                    }
                    "toggle_random_breaks" => {
                        do_toggle_random_breaks(&app.state::<AppState>());
                    }
                    "open_settings" => focus_main_window(app),
                    "quit" => {
                        #[cfg(target_os = "linux")]
                        app.state::<AppState>().screensaver_inhibitor.uninhibit();
                        #[cfg(target_os = "windows")]
                        app.state::<AppState>().windows_inhibitor.uninhibit();
                        app.state::<AppState>()
                            .quitting
                            .store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window(MAIN_WINDOW) {
                            let visible = window.is_visible().unwrap_or(false);
                            if visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // `tauri.conf.json`'s window config has no `icon` field in this
            // Tauri version — without this the running window/taskbar icon
            // falls back to Tauri's default logo even though the bundled
            // launcher icon (desktop file) is correct. Embedded at compile
            // time so it doesn't depend on resource-dir bundling behavior.
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                if let Ok(icon) = Image::from_bytes(include_bytes!("../icons/icon.png")) {
                    let _ = window.set_icon(icon);
                }
            }

            #[cfg(any(target_os = "linux", target_os = "windows"))]
            spawn_jiggle_loop(app_handle.clone(), jiggling.clone(), config.clone(), tray_state.clone());

            #[cfg(target_os = "linux")]
            let screensaver_inhibitor = Arc::new(screensaver_inhibit::ScreenSaverInhibitor::new());
            #[cfg(target_os = "windows")]
            let windows_inhibitor = Arc::new(windows_inhibit::WindowsInhibitor::new());

            // Re-sync the autostart entry (.desktop on Linux, registry Run
            // key on Windows) to match saved config on every launch — covers
            // reinstalls (binary path changed) or an entry that got
            // deleted/edited outside the app.
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                let result = if config.lock().expect("config mutex poisoned").autostart {
                    autostart::enable()
                } else {
                    autostart::disable()
                };
                if let Err(err) = result {
                    eprintln!("zzzzziglerrr[autostart] gagal sync autostart pas startup: {err}");
                }
            }

            app.manage(AppState {
                config,
                jiggling,
                quitting,
                tray_state,
                app_handle,
                jiggle_menu_item,
                autostart_menu_item,
                random_breaks_menu_item,
                #[cfg(target_os = "linux")]
                screensaver_inhibitor,
                #[cfg(target_os = "windows")]
                windows_inhibitor,
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            toggle_jiggle,
            toggle_autostart,
            toggle_random_breaks,
            get_status
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Default Tauri/tao behavior on Linux (and, it turns out,
            // Windows — this app.exit(0) still routes through
            // ExitRequested there too) exits the whole process once no
            // window is visible — hiding to tray on minimize/close would
            // otherwise kill the app outright. Only suppress the exit when
            // it's NOT the genuine "Keluar" quit (tracked via
            // AppState.quitting, set right before that `app.exit(0)` call)
            // — unconditionally calling prevent_exit() here blocked the
            // real quit path too on Windows.
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let quitting = app_handle
                    .state::<AppState>()
                    .quitting
                    .load(Ordering::SeqCst);
                if !quitting {
                    api.prevent_exit();
                }
            }
        });
}
