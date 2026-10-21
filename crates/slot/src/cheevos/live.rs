// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/live.rs.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use slot_cheevos::{
    badge, config, game, memory, Achievement, Cheevos, Credentials, Http, Login, Notice, Secret,
};
use slot_gfx::{Compositor, Draw, TexId};
use slot_retro::RetroCore;

use super::boot;
use super::overlay::{self, Face, Queue};

const LOGIN_BACKOFF: [u64; 4] = [15, 30, 60, 300];
const LOAD_RETRY: Duration = Duration::from_secs(5);
const IDLE_AFTER: Duration = Duration::from_millis(250);
const IDLE_EVERY: Duration = Duration::from_secs(1);

struct Live {
    cheevos: &'static Cheevos,
    root: PathBuf,
    faces: Mutex<Queue>,
    tickets: AtomicU64,
    current: AtomicU64,
    booting: AtomicU64,
    switch: Mutex<()>,
}

static LIVE: OnceLock<Live> = OnceLock::new();

struct Seated {
    ticket: u64,
    console: Arc<AtomicU32>,
    boot_left: u32,
    last_frame: Option<Instant>,
    last_idle: Option<Instant>,
}

struct Shown {
    tex: Option<TexId>,
    since: Option<Instant>,
}

thread_local! {
    static SEATED: RefCell<Option<Seated>> = const { RefCell::new(None) };
    static SHOWN: RefCell<Shown> = const { RefCell::new(Shown { tex: None, since: None }) };
}

// --- Fragile: on or off from the Network page, what the sign-in came to, the game's list ---

static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static STATUS: Mutex<Status> = Mutex::new(Status::SignedOut);
/// Bumped by each sign-in, so an older one still waiting gives up.
static SIGN_INS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    SignedOut,
    /// Waiting for the Wi-Fi, or for the server.
    SigningIn,
    SignedIn(String),
    Rejected(String),
}

fn set_status(status: Status) {
    *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = status;
}

pub fn status() -> Status {
    STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Off, games start without being followed; a game already playing keeps going.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}

/// The playing game's achievements, unlocked first.
pub fn achievements() -> Vec<Achievement> {
    LIVE.get()
        .map(|live| live.cheevos.achievements())
        .unwrap_or_default()
}

/// After the Network page writes new credentials: the first sign-in starts the client, a
/// later one signs the old account out and the new one in.
pub fn sign_in_again(root: &Path) {
    let Some(live) = LIVE.get() else {
        return start(root);
    };
    let Some(credentials) = Credentials::read(root) else {
        live.cheevos.logout();
        set_status(Status::SignedOut);
        return;
    };
    live.cheevos.logout();
    let root = root.to_path_buf();
    spawn("slot-cheevos-login", move || sign_in(&root, credentials));
}

pub fn start(root: &Path) {
    if LIVE.get().is_some() {
        return;
    }
    if std::env::var_os("SLOT_NO_CHEEVOS").is_some() {
        return;
    }
    let Some(credentials) = Credentials::read(root) else {
        return;
    };
    let cheevos = Cheevos::start(credentials.host.as_deref(), |ua| Box::new(Http::new(ua)));
    let live = Live {
        cheevos,
        root: root.to_path_buf(),
        faces: Mutex::new(Queue::default()),
        tickets: AtomicU64::new(0),
        current: AtomicU64::new(0),
        booting: AtomicU64::new(0),
        switch: Mutex::new(()),
    };
    if LIVE.set(live).is_err() {
        return;
    }
    let root = root.to_path_buf();
    spawn("slot-cheevos-login", move || sign_in(&root, credentials));
    spawn("slot-cheevos-face", draw_faces);
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = std::thread::Builder::new().name(name.into()).spawn(f) {
        eprintln!("slot: cheevos: {e}");
    }
}

fn sign_in(root: &Path, credentials: Credentials) {
    let Some(live) = LIVE.get() else { return };
    let me = SIGN_INS.fetch_add(1, Ordering::AcqRel) + 1;
    let current = || SIGN_INS.load(Ordering::Acquire) == me;
    set_status(Status::SigningIn);
    let mut failures = 0;
    loop {
        while !crate::wifi::online() {
            if !current() {
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        if !current() {
            return;
        }
        let (tx, rx) = channel();
        live.cheevos.login(&credentials, move |l| {
            let _ = tx.send(l);
        });
        match rx.recv() {
            Ok(Login::Ok { user, token }) => {
                if matches!(credentials.secret, Secret::Password(_)) {
                    if let Err(e) = config::remember_token(root, &token) {
                        eprintln!("slot: cheevos: saving token: {e}");
                    }
                }
                set_status(Status::SignedIn(user.clone()));
                live.cheevos.push(Notice::LoggedIn { user });
                break;
            }
            Ok(Login::Rejected {
                reason,
                needs_password,
            }) => {
                eprintln!("slot: cheevos: login rejected: {reason}");
                set_status(Status::Rejected(reason.clone()));
                live.cheevos.push(Notice::LoginFailed {
                    reason,
                    needs_password,
                });
                return;
            }
            Ok(Login::Unreachable) | Err(_) => {
                if failures == 0 {
                    live.cheevos.push(Notice::Offline);
                }
                let wait = LOGIN_BACKOFF[failures.min(LOGIN_BACKOFF.len() - 1)];
                failures += 1;
                std::thread::sleep(Duration::from_secs(wait));
            }
        }
    }
    while current() {
        if live.cheevos.wants_load() {
            live.cheevos.retry_load();
        }
        std::thread::sleep(LOAD_RETRY);
    }
}

fn draw_faces() {
    let Some(live) = LIVE.get() else { return };
    loop {
        let Some(notice) = live.cheevos.next_notice(Duration::from_secs(60)) else {
            continue;
        };
        let badge = notice
            .badge_url()
            .and_then(|url| live.cheevos.transport().fetch(url))
            .and_then(|png| badge::decode(&png));
        let face = overlay::compose(&notice, badge.as_deref());
        live.faces.lock().unwrap().push(face);
    }
}

pub struct Guard {
    ticket: Option<u64>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Some(ticket) = self.ticket else { return };
        SEATED.with_borrow_mut(|s| *s = None);
        if let Some(live) = LIVE.get() {
            live.booted(ticket);
            let _switch = live.switch.lock().unwrap();
            if live.current.load(Ordering::Acquire) == ticket {
                live.current.store(0, Ordering::Release);
                live.cheevos.unload_game();
            }
        }
        memory::clear();
    }
}

impl Live {
    fn booted(&self, ticket: u64) {
        let _ = self
            .booting
            .compare_exchange(ticket, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}

pub fn loaded(rom: &Path, resumed: bool) -> Guard {
    let Some(live) = LIVE.get().filter(|_| ENABLED.load(Ordering::Acquire)) else {
        return Guard { ticket: None };
    };
    let ticket = live.tickets.fetch_add(1, Ordering::AcqRel) + 1;
    {
        let _switch = live.switch.lock().unwrap();
        live.current.store(ticket, Ordering::Release);
    }
    let boot_left = boot::frames(&live.root, rom, resumed);
    if boot_left > 0 {
        live.booting.store(ticket, Ordering::Release);
    }
    let console = Arc::new(AtomicU32::new(0));
    SEATED.with_borrow_mut(|s| {
        *s = Some(Seated {
            ticket,
            console: console.clone(),
            boot_left,
            last_frame: None,
            last_idle: None,
        })
    });
    let rom = rom.to_path_buf();
    spawn("slot-cheevos-hash", move || {
        identify(live, ticket, &rom, &console)
    });
    Guard {
        ticket: Some(ticket),
    }
}

fn identify(live: &Live, ticket: u64, rom: &PathBuf, console: &AtomicU32) {
    let bytes = match std::fs::read(rom) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("slot: cheevos: {}: {e}", rom.display());
            return;
        }
    };
    let id = game::console_for(rom, &bytes);
    let hash = game::hash(id, &bytes);
    drop(bytes);
    let _switch = live.switch.lock().unwrap();
    if live.current.load(Ordering::Acquire) != ticket {
        return;
    }
    console.store(id, Ordering::Release);
    if let Some(hash) = hash {
        live.cheevos.load_game(&hash);
    }
}

fn ready(live: &Live, seated: &Seated, core: &dyn RetroCore) -> bool {
    if live.current.load(Ordering::Acquire) != seated.ticket {
        return false;
    }
    let console = live
        .cheevos
        .game_console()
        .unwrap_or_else(|| seated.console.load(Ordering::Acquire));
    console != 0 && memory::refresh(core.memory_map(), console)
}

pub fn frame(core: &dyn RetroCore) {
    let Some(live) = LIVE.get() else { return };
    SEATED.with_borrow_mut(|s| {
        let Some(seated) = s else { return };
        if seated.boot_left > 0 {
            seated.boot_left -= 1;
            if seated.boot_left == 0 {
                live.booted(seated.ticket);
            }
        }
        if ready(live, seated, core) {
            live.cheevos.do_frame();
            seated.last_frame = Some(Instant::now());
        }
    });
}

pub fn idle(core: &dyn RetroCore) {
    let Some(live) = LIVE.get() else { return };
    SEATED.with_borrow_mut(|s| {
        let Some(seated) = s else { return };
        let now = Instant::now();
        let running = seated
            .last_frame
            .is_some_and(|t| now.duration_since(t) < IDLE_AFTER);
        let recently = seated
            .last_idle
            .is_some_and(|t| now.duration_since(t) < IDLE_EVERY);
        if running || recently || !ready(live, seated, core) {
            return;
        }
        live.cheevos.idle();
        seated.last_idle = Some(now);
    });
}

pub fn reset() {
    let Some(live) = LIVE.get() else { return };
    let current = live.current.load(Ordering::Acquire);
    let seated = SEATED.with_borrow(|s| s.as_ref().is_some_and(|s| s.ticket == current));
    if seated && memory::ready() {
        live.cheevos.reset();
    }
}

pub fn overlay(compositor: &mut Compositor, draws: &mut Vec<Draw>) {
    let Some(live) = LIVE.get() else { return };
    SHOWN.with_borrow_mut(|shown| {
        let now = Instant::now();
        let mut alpha = shown
            .since
            .and_then(|t| overlay::alpha(now.duration_since(t)));
        if alpha.is_none() {
            shown.since = None;
            if live.booting.load(Ordering::Acquire) != 0 {
                return;
            }
            let Some(face) = live.faces.lock().unwrap().pop() else {
                return;
            };
            show(compositor, shown, &face);
            shown.since = Some(now);
            alpha = overlay::alpha(Duration::ZERO);
        }
        let (Some(tex), Some(alpha)) = (shown.tex, alpha) else {
            return;
        };
        let (x, y, w, h) = overlay::rect();
        draws.push(Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha,
        });
    });
}

fn show(compositor: &mut Compositor, shown: &mut Shown, face: &Face) {
    let (w, h) = (overlay::FACE_W, overlay::FACE_H);
    match shown.tex {
        Some(id) => compositor.update_texture(id, w, h, &face.rgba),
        None => shown.tex = Some(compositor.create_texture(w, h, &face.rgba)),
    }
}
