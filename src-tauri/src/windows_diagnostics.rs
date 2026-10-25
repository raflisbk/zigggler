//! Explicit diagnostic build + runtime opt-in; never an alternate injector.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows::Win32::{
    System::{
        RemoteDesktop::ProcessIdToSessionId,
        SystemInformation::GetTickCount,
        Threading::{GetCurrentProcessId, GetCurrentThreadId},
    },
    UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
};

static OUTPUT: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

fn output() -> Option<&'static Mutex<std::fs::File>> {
    OUTPUT
        .get_or_init(|| {
            let path = std::env::var_os("ZZZZZIGLERRR_DIAGNOSTICS_LOG")?;
            match OpenOptions::new().create(true).append(true).open(path) {
                Ok(file) => Some(Mutex::new(file)),
                Err(error) => {
                    eprintln!("diagnostic log unavailable: {error}");
                    None
                }
            }
        })
        .as_ref()
}

pub fn record(mut value: Value) {
    let Some(output) = output() else { return };
    let pid = unsafe { GetCurrentProcessId() };
    let mut session = 0;
    let session_ok = unsafe { ProcessIdToSessionId(pid, &mut session) }.is_ok();
    value["pid"] = json!(pid);
    value["tid"] = json!(unsafe { GetCurrentThreadId() });
    value["session"] = if session_ok {
        json!(session)
    } else {
        Value::Null
    };
    value["elapsed_ms"] = json!(START.get_or_init(Instant::now).elapsed().as_millis());
    value["unix_ms"] = json!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    if let Ok(mut file) = output.lock() {
        let _ = writeln!(file, "{value}");
    }
}

fn sample(phase: &str, attempt: u64) {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut info) }.as_bool();
    let error = if ok {
        None
    } else {
        Some(unsafe { windows::Win32::Foundation::GetLastError() }.0)
    };
    let tick = unsafe { GetTickCount() };
    record(
        json!({"phase":phase, "attempt":attempt, "tick":tick, "last_input_ok":ok,
        "last_input_error":error, "last_input_tick":if ok { Some(info.dwTime) } else { None },
        "idle_ms":if ok { Some(crate::diagnostic_math::idle_ms(tick, info.dwTime)) } else { None }}),
    );
}

/// Read-only `GetLastInputInfo` sample taken while the main toggle is OFF —
/// no `SendInput` call, no injection, nothing to measure a *reset* for.
/// Needed so the OFF control arm has a raw idle timeline too, instead of
/// only the bare `{"enabled": false}` tick marker. Reuses the real
/// `interval_secs` cadence (called once per loop tick), not a second timer.
pub fn baseline() {
    if output().is_none() {
        return;
    }
    sample("baseline", 0);
}

pub fn before() -> Option<u64> {
    output()?;
    let attempt = ATTEMPT.fetch_add(1, Ordering::Relaxed) + 1;
    sample("before", attempt);
    Some(attempt)
}

/// CADENCE NOTE for analysis: this blocks the jiggle worker for 20×50ms =
/// ~1s after every `SendInput` before the next loop tick can start its own
/// `sleep(interval_secs)`. So the real gap between consecutive "before"
/// samples is `interval_secs + ~1s`, not exactly `interval_secs` — e.g.
/// `intervalSecs=5` reads as ~6s between attempts, `intervalSecs=60` reads
/// as ~61s. Don't read that +1s as a missed tick or a stall.
pub fn after(attempt: Option<u64>, requested: usize, sent: u32, error: Option<u32>) {
    let Some(attempt) = attempt else { return };
    record(
        json!({"phase":"send_input", "attempt":attempt, "requested":requested, "sent":sent, "error":error}),
    );
    sample("after", attempt);
    // Bounded observation window, not an API guarantee. Remains on the normal
    // jiggle worker so no other injection from this loop overlaps the samples.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(50));
        sample("after_poll", attempt);
    }
}

/// RUNBOOK WARNING: `ZZZZZIGLERRR_DIAGNOSTICS_CONFIG` applies the trial
/// config through the real `do_set_config` handler — same path the UI
/// uses — which means it OVERWRITES the user's actual persisted
/// `config.json` on disk, not an in-memory-only override. Whoever runs a
/// trial (platform-compat) must back up `config.json` byte-for-byte
/// before activating and restore it after, for every trial run — this is
/// known/accepted (not something this PR works around), but it must not
/// be skipped operationally.
pub fn activate(state: &crate::AppState) -> Result<(), Box<dyn std::error::Error>> {
    // Explicit path required even for feature builds; malformed opt-ins fail
    // startup instead of silently running an unintended trial.
    if std::env::var_os("ZZZZZIGLERRR_DIAGNOSTICS_LOG").is_none() {
        return Ok(());
    }
    if output().is_none() {
        return Err("cannot open diagnostics log".into());
    }
    if let Some(path) = std::env::var_os("ZZZZZIGLERRR_DIAGNOSTICS_CONFIG") {
        let config = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        crate::do_set_config(state, config)?;
    }
    record(
        json!({"phase":"startup", "config":state.config.lock().expect("config mutex poisoned").clone()}),
    );
    if std::env::var("ZZZZZIGLERRR_DIAGNOSTICS_ENABLE").as_deref() == Ok("1") {
        crate::do_toggle_jiggle(state);
        record(json!({"phase":"activation", "handler":"do_toggle_jiggle"}));
    }
    Ok(())
}
