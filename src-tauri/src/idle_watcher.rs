pub trait IdleWatcher: Send + Sync {
    /// Seconds the user has been idle. 0 means currently active.
    fn idle_seconds(&self) -> u64;
}

#[cfg(target_os = "linux")]
pub use linux::WaylandIdleWatcher;

#[cfg(target_os = "linux")]
mod linux {
    use super::IdleWatcher;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    use wayland_client::globals::GlobalListContents;
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_seat::WlSeat;
    use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
    use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
        self, ExtIdleNotificationV1,
    };
    use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

    /// How long the compositor waits without input before firing the "idled" event.
    /// Kept short relative to the jiggle interval so a real idle period is still
    /// detected well before the next scheduler tick, even after our own jiggle
    /// resets the compositor's input timer.
    const IDLE_TIMEOUT_MS: u32 = 8_000;

    fn log_ts(msg: &str) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        eprintln!("zzzzziglerrr[idle_watcher] t={secs} {msg}");
    }

    struct State {
        idle_since: Arc<Mutex<Option<Instant>>>,
        /// When the previous Idled/Resumed transition happened, and how many
        /// transitions have fired so far — diagnostic only, lets a long
        /// session's logs show exactly how often/fast the compositor is
        /// flipping idle state instead of just the raw event names.
        last_transition: Instant,
        transition_count: u64,
    }

    /// Recovers the lock value even if a previous panic poisoned the mutex —
    /// one bad event handler shouldn't permanently brick idle tracking for the
    /// rest of the process.
    fn lock_recoverable<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    impl Dispatch<WlRegistry, GlobalListContents> for State {
        fn event(
            _state: &mut Self,
            _proxy: &WlRegistry,
            _event: wayland_client::protocol::wl_registry::Event,
            _data: &GlobalListContents,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<WlSeat, ()> for State {
        fn event(
            _state: &mut Self,
            _proxy: &WlSeat,
            _event: wayland_client::protocol::wl_seat::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ExtIdleNotifierV1, ()> for State {
        fn event(
            _state: &mut Self,
            _proxy: &ExtIdleNotifierV1,
            _event: wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ExtIdleNotificationV1, ()> for State {
        fn event(
            state: &mut Self,
            _proxy: &ExtIdleNotificationV1,
            event: ext_idle_notification_v1::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
            let name = match event {
                ext_idle_notification_v1::Event::Idled => {
                    *lock_recoverable(&state.idle_since) = Some(Instant::now());
                    Some("Idled")
                }
                ext_idle_notification_v1::Event::Resumed => {
                    *lock_recoverable(&state.idle_since) = None;
                    Some("Resumed")
                }
                _ => None,
            };
            if let Some(name) = name {
                let now = Instant::now();
                let since_prev = now.duration_since(state.last_transition).as_secs_f64();
                state.last_transition = now;
                state.transition_count += 1;
                log_ts(&format!(
                    "{name} (transition #{}, {since_prev:.1}s since previous transition)",
                    state.transition_count
                ));
            }
        }
    }

    pub struct WaylandIdleWatcher {
        idle_since: Arc<Mutex<Option<Instant>>>,
        /// False once the Wayland dispatch thread has stopped (panic or
        /// protocol error). When that happens we can no longer trust
        /// `idle_since`, so `idle_seconds()` fails open instead of freezing
        /// at whatever value it last had — a dead watcher must never
        /// permanently block jiggling under `idle_only`.
        dispatch_alive: Arc<AtomicBool>,
    }

    impl WaylandIdleWatcher {
        /// Spawns a background thread running the Wayland event loop. Returns
        /// `None` if the compositor doesn't advertise `ext-idle-notify-v1`
        /// (e.g. Sway on older wlroots) — callers should fall back to
        /// `idle_only` being unavailable rather than failing hard.
        pub fn spawn() -> Option<Self> {
            let conn = Connection::connect_to_env().ok()?;
            let (globals, queue) =
                wayland_client::globals::registry_queue_init::<State>(&conn).ok()?;
            let qh = queue.handle();

            let seat: WlSeat = globals.bind(&qh, 1..=1, ()).ok()?;
            let notifier: ExtIdleNotifierV1 = globals.bind(&qh, 1..=1, ()).ok()?;

            let idle_since = Arc::new(Mutex::new(None));
            let state = State {
                idle_since: idle_since.clone(),
                last_transition: Instant::now(),
                transition_count: 0,
            };

            // Kept alive for the lifetime of the dispatch thread — dropping it
            // early would be surprising (and risks the compositor tearing down
            // the object), so it's moved into the loop instead of discarded.
            let notification = notifier.get_idle_notification(IDLE_TIMEOUT_MS, &seat, &qh, ());

            let dispatch_alive = Arc::new(AtomicBool::new(true));
            spawn_dispatch_thread(queue, state, notification, dispatch_alive.clone());

            Some(Self {
                idle_since,
                dispatch_alive,
            })
        }
    }

    fn spawn_dispatch_thread(
        mut queue: EventQueue<State>,
        mut state: State,
        _notification: ExtIdleNotificationV1,
        dispatch_alive: Arc<AtomicBool>,
    ) {
        std::thread::spawn(move || {
            log_ts("dispatch thread started");
            loop {
                if let Err(err) = queue.blocking_dispatch(&mut state) {
                    log_ts(&format!(
                        "dispatch thread stopping, blocking_dispatch error: {err}"
                    ));
                    break;
                }
            }
            dispatch_alive.store(false, Ordering::SeqCst);
            log_ts("dispatch thread dead, idle_seconds() fails open from now on");
        });
    }

    impl IdleWatcher for WaylandIdleWatcher {
        fn idle_seconds(&self) -> u64 {
            if !self.dispatch_alive.load(Ordering::SeqCst) {
                // Watcher is dead — we have no idea if the user is idle.
                // Fail open rather than silently never jiggling again.
                return IDLE_TIMEOUT_MS as u64 / 1000 + 1;
            }
            match *lock_recoverable(&self.idle_since) {
                Some(since) => (IDLE_TIMEOUT_MS as u64 / 1000) + since.elapsed().as_secs(),
                None => 0,
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::WindowsIdleWatcher;

#[cfg(target_os = "windows")]
mod windows {
    use super::IdleWatcher;
    use ::windows::Win32::System::SystemInformation::GetTickCount;
    use ::windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    /// `GetLastInputInfo` (user32) — milliseconds since the last input event
    /// system-wide. Unlike Linux's `ext-idle-notify-v1` (which doesn't count
    /// synthetic uinput as activity, requiring the separate
    /// `org_kde_kwin_idle.simulate_user_activity` signal), web research
    /// suggests `SendInput` directly resets this counter since both paths
    /// go through the same low-level input queue — **not yet verified on
    /// real Windows hardware**. If that assumption turns out false, a
    /// second reset signal analogous to `kwin_idle_simulate` would be
    /// needed here too.
    pub struct WindowsIdleWatcher;

    impl WindowsIdleWatcher {
        pub fn new() -> Self {
            Self
        }
    }

    impl IdleWatcher for WindowsIdleWatcher {
        fn idle_seconds(&self) -> u64 {
            let mut info = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if unsafe { GetLastInputInfo(&mut info) }.0 == 0 {
                // Call failed — fail open rather than silently blocking
                // idle_only forever.
                return u64::MAX;
            }
            let now = unsafe { GetTickCount() };
            now.wrapping_sub(info.dwTime) as u64 / 1000
        }
    }
}
