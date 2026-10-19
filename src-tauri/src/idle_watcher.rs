pub trait IdleWatcher: Send + Sync {
    /// Seconds the user has been idle. 0 means currently active.
    fn idle_seconds(&self) -> u64;
}

#[cfg(target_os = "linux")]
pub use linux::WaylandIdleWatcher;

#[cfg(target_os = "linux")]
mod linux {
    use super::IdleWatcher;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use wayland_client::globals::GlobalListContents;
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_seat::WlSeat;
    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
        self, ExtIdleNotificationV1,
    };
    use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

    /// How long the compositor waits without input before firing the "idled" event.
    /// Kept short relative to the jiggle interval so a real idle period is still
    /// detected well before the next scheduler tick, even after our own jiggle
    /// resets the compositor's input timer.
    const IDLE_TIMEOUT_MS: u32 = 8_000;

    struct State {
        idle_since: Arc<Mutex<Option<Instant>>>,
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
            match event {
                ext_idle_notification_v1::Event::Idled => {
                    *state.idle_since.lock().expect("idle mutex poisoned") = Some(Instant::now());
                }
                ext_idle_notification_v1::Event::Resumed => {
                    *state.idle_since.lock().expect("idle mutex poisoned") = None;
                }
                _ => {}
            }
        }
    }

    pub struct WaylandIdleWatcher {
        idle_since: Arc<Mutex<Option<Instant>>>,
    }

    impl WaylandIdleWatcher {
        /// Spawns a background thread running the Wayland event loop. Returns
        /// `None` if the compositor doesn't advertise `ext-idle-notify-v1`
        /// (e.g. Sway on older wlroots) — callers should fall back to
        /// `idle_only` being unavailable rather than failing hard.
        pub fn spawn() -> Option<Self> {
            let conn = Connection::connect_to_env().ok()?;
            let (globals, mut queue) =
                wayland_client::globals::registry_queue_init::<State>(&conn).ok()?;
            let qh = queue.handle();

            let seat: WlSeat = globals
                .bind(&qh, 1..=1, ())
                .ok()?;
            let notifier: ExtIdleNotifierV1 = globals.bind(&qh, 1..=1, ()).ok()?;

            let idle_since = Arc::new(Mutex::new(None));
            let mut state = State {
                idle_since: idle_since.clone(),
            };

            let _notification = notifier.get_idle_notification(IDLE_TIMEOUT_MS, &seat, &qh, ());

            std::thread::spawn(move || loop {
                if queue.blocking_dispatch(&mut state).is_err() {
                    break;
                }
            });

            Some(Self { idle_since })
        }
    }

    impl IdleWatcher for WaylandIdleWatcher {
        fn idle_seconds(&self) -> u64 {
            match *self.idle_since.lock().expect("idle mutex poisoned") {
                Some(since) => (IDLE_TIMEOUT_MS as u64 / 1000) + since.elapsed().as_secs(),
                None => 0,
            }
        }
    }
}
