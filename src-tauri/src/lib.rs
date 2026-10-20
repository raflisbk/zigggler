mod config;
mod idle_watcher;
mod mouse_injector;
mod scheduler;
mod screensaver_inhibit;

use config::Config;
use mouse_injector::MouseInjector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main-tray";
const MAIN_WINDOW: &str = "main";

#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Off,
    Waiting,
    Jiggling,
}

struct AppState {
    config: Arc<Mutex<Config>>,
    jiggling: Arc<AtomicBool>,
    tray_state: Arc<Mutex<TrayState>>,
    app_handle: AppHandle,
    #[cfg(target_os = "linux")]
    screensaver_inhibitor: Arc<screensaver_inhibit::ScreenSaverInhibitor>,
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Config {
    state.config.lock().expect("config mutex poisoned").clone()
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
    let now = !state.jiggling.load(Ordering::SeqCst);
    state.jiggling.store(now, Ordering::SeqCst);
    let next_state = if now { TrayState::Waiting } else { TrayState::Off };
    *state.tray_state.lock().expect("tray state mutex poisoned") = next_state;
    let theme = state.config.lock().expect("config mutex poisoned").theme.clone();
    refresh_tray(&state.app_handle, next_state, &theme);

    #[cfg(target_os = "linux")]
    {
        if now {
            state.screensaver_inhibitor.inhibit();
        } else {
            state.screensaver_inhibitor.uninhibit();
        }
    }

    now
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

fn dot_icon(rgb: (u8, u8, u8), alpha: u8) -> Image<'static> {
    const SIZE: u32 = 32;
    let (r, g, b) = rgb;
    let center = SIZE as f32 / 2.0;
    let radius = center - 2.0;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            if dx * dx + dy * dy <= radius * radius {
                let idx = ((y * SIZE + x) * 4) as usize;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = alpha;
            }
        }
    }
    Image::new_owned(pixels, SIZE, SIZE)
}

fn tray_icon_for(state: TrayState, theme: &str) -> Image<'static> {
    match state {
        TrayState::Off => dot_icon((0x6B, 0x6B, 0x70), 160),
        TrayState::Waiting => dot_icon(theme_swatch(theme), 130),
        TrayState::Jiggling => dot_icon(theme_swatch(theme), 255),
    }
}

fn refresh_tray(app_handle: &AppHandle, state: TrayState, theme: &str) {
    if let Some(tray) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(tray_icon_for(state, theme)));
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn spawn_jiggle_loop(
    app_handle: AppHandle,
    jiggling: Arc<AtomicBool>,
    config: Arc<Mutex<Config>>,
    tray_state: Arc<Mutex<TrayState>>,
) {
    use idle_watcher::{IdleWatcher, WaylandIdleWatcher};
    use mouse_injector::UinputInjector;

    std::thread::spawn(move || {
        let injector = match UinputInjector::new() {
            Ok(injector) => injector,
            Err(err) => {
                eprintln!("zzzzziglerrr: gagal bikin uinput device: {err}");
                return;
            }
        };
        let idle_watcher = WaylandIdleWatcher::spawn();
        if idle_watcher.is_none() {
            eprintln!(
                "zzzzziglerrr: compositor gak dukung ext-idle-notify-v1, idle_only diabaikan"
            );
        }

        let mut consecutive_closed_ticks: u64 = 0;
        loop {
            let (interval_secs, idle_only, theme) = {
                let config = config.lock().expect("config mutex poisoned");
                (config.interval_secs, config.idle_only, config.theme.clone())
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

            if gate_open {
                *tray_state.lock().expect("tray state mutex poisoned") = TrayState::Jiggling;
                refresh_tray(&app_handle, TrayState::Jiggling, &theme);
                injector.jiggle();
                if jiggling.load(Ordering::SeqCst) {
                    *tray_state.lock().expect("tray state mutex poisoned") = TrayState::Waiting;
                    refresh_tray(&app_handle, TrayState::Waiting, &theme);
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = Arc::new(Mutex::new(Config::load()));
            let jiggling = Arc::new(AtomicBool::new(false));
            let tray_state = Arc::new(Mutex::new(TrayState::Off));
            let app_handle = app.handle().clone();

            let quit_item = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_icon_for(TrayState::Off, &config.lock().expect("config mutex poisoned").theme))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        #[cfg(target_os = "linux")]
                        app.state::<AppState>().screensaver_inhibitor.uninhibit();
                        app.exit(0);
                    }
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

            #[cfg(target_os = "linux")]
            spawn_jiggle_loop(app_handle.clone(), jiggling.clone(), config.clone(), tray_state.clone());

            #[cfg(target_os = "linux")]
            let screensaver_inhibitor = Arc::new(screensaver_inhibit::ScreenSaverInhibitor::new());

            app.manage(AppState {
                config,
                jiggling,
                tray_state,
                app_handle,
                #[cfg(target_os = "linux")]
                screensaver_inhibitor,
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![get_config, set_config, toggle_jiggle])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
