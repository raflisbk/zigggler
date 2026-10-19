mod config;
mod mouse_injector;

use config::Config;
use mouse_injector::MouseInjector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;

struct AppState {
    config: Arc<Mutex<Config>>,
    jiggling: Arc<AtomicBool>,
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Config {
    state.config.lock().expect("config mutex poisoned").clone()
}

#[tauri::command]
fn set_config(state: tauri::State<AppState>, config: Config) -> Result<(), String> {
    config.save().map_err(|err| err.to_string())?;
    *state.config.lock().expect("config mutex poisoned") = config;
    Ok(())
}

#[tauri::command]
fn toggle_jiggle(state: tauri::State<AppState>) -> bool {
    let now = !state.jiggling.load(Ordering::SeqCst);
    state.jiggling.store(now, Ordering::SeqCst);
    now
}

#[cfg(target_os = "linux")]
fn spawn_jiggle_loop(jiggling: Arc<AtomicBool>, config: Arc<Mutex<Config>>) {
    use mouse_injector::UinputInjector;

    std::thread::spawn(move || {
        let injector = match UinputInjector::new() {
            Ok(injector) => injector,
            Err(err) => {
                eprintln!("zzzzziglerrr: gagal bikin uinput device: {err}");
                return;
            }
        };
        loop {
            let interval_secs = config.lock().expect("config mutex poisoned").interval_secs;
            std::thread::sleep(Duration::from_secs(interval_secs.max(1)));
            if jiggling.load(Ordering::SeqCst) {
                injector.jiggle();
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

            #[cfg(target_os = "linux")]
            spawn_jiggle_loop(jiggling.clone(), config.clone());

            app.manage(AppState { config, jiggling });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_config, set_config, toggle_jiggle])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
