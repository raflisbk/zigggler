#[cfg(target_os = "linux")]
pub use linux::KwinIdleSimulator;

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::{Mutex, MutexGuard};
    use std::time::{SystemTime, UNIX_EPOCH};
    use wayland_client::globals::GlobalListContents;
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_seat::WlSeat;
    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_protocols_plasma::idle::client::org_kde_kwin_idle::OrgKdeKwinIdle;
    use wayland_protocols_plasma::idle::client::org_kde_kwin_idle_timeout::{
        self, OrgKdeKwinIdleTimeout,
    };

    /// Timeout passed to `get_idle_timeout` — irrelevant to us since we only
    /// ever use the returned object to call `simulate_user_activity`, never
    /// to read its Idle/Resumed events.
    const IDLE_TIMEOUT_MS: u32 = 8_000;

    fn log_ts(msg: &str) {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        eprintln!("zzzzziglerrr[kwin_idle_simulate] t={secs} {msg}");
    }

    struct State;

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

    impl Dispatch<OrgKdeKwinIdle, ()> for State {
        fn event(
            _state: &mut Self,
            _proxy: &OrgKdeKwinIdle,
            _event: wayland_protocols_plasma::idle::client::org_kde_kwin_idle::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<OrgKdeKwinIdleTimeout, ()> for State {
        fn event(
            _state: &mut Self,
            _proxy: &OrgKdeKwinIdleTimeout,
            _event: org_kde_kwin_idle_timeout::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
            // We only use this object to send `simulate_user_activity` — its
            // own Idle/Resumed events are irrelevant to us.
        }
    }

    /// Calls `org_kde_kwin_idle_timeout.simulate_user_activity()` on demand —
    /// confirmed (docs/goals.md 2026-10-22) to reset the seat-wide activity
    /// signal KWin propagates to every idle-tracking consumer, including
    /// `ext-idle-notify-v1` watchers like ActivityWatch's aw-awatcher. Runs
    /// alongside `UinputInjector`, not instead of it — uinput still provides
    /// the actual visible cursor nudge.
    pub struct KwinIdleSimulator {
        conn: Mutex<Connection>,
        timeout: OrgKdeKwinIdleTimeout,
    }

    impl KwinIdleSimulator {
        /// Returns `None` if `org_kde_kwin_idle` isn't exposed by the
        /// compositor (anything but KWin, e.g. Mutter/Sway on P1) — callers
        /// should fail open and keep relying on uinput alone rather than
        /// treating this as fatal.
        pub fn connect() -> Option<Self> {
            let conn = Connection::connect_to_env().ok()?;
            let (globals, mut queue) =
                wayland_client::globals::registry_queue_init::<State>(&conn).ok()?;
            let qh = queue.handle();

            let seat: WlSeat = globals.bind(&qh, 1..=1, ()).ok()?;
            let idle_manager: OrgKdeKwinIdle = globals.bind(&qh, 1..=1, ()).ok()?;
            let timeout = idle_manager.get_idle_timeout(&seat, IDLE_TIMEOUT_MS, &qh, ());

            // Round-trip once so the get_idle_timeout request is actually
            // flushed and the object is live before we return it.
            queue.roundtrip(&mut State).ok()?;

            log_ts("connected, org_kde_kwin_idle available");
            Some(Self {
                conn: Mutex::new(conn),
                timeout,
            })
        }

        pub fn simulate(&self) {
            self.timeout.simulate_user_activity();
            match self.conn_lock().flush() {
                Ok(()) => log_ts("simulate_user_activity sent ok"),
                Err(err) => log_ts(&format!("simulate_user_activity flush FAILED: {err}")),
            }
        }

        fn conn_lock(&self) -> MutexGuard<'_, Connection> {
            self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }
}
