// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/client.rs.

use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::config::{Credentials, Secret};
use crate::ffi::{self, rc_client_t};
use crate::notice::{self, Event, Game, Notice};
use crate::transport::{Request, Transport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Login {
    Ok {
        user: String,
        token: String,
    },
    Rejected {
        reason: String,
        needs_password: bool,
    },
    Unreachable,
}

pub struct Cheevos {
    client: *mut rc_client_t,
    jobs: Mutex<Sender<Job>>,
    transport: std::sync::Arc<dyn Transport>,
    notices: Mutex<VecDeque<Notice>>,
    arrived: Condvar,
    generation: AtomicU64,
    console: AtomicU32,
    logged_in: AtomicBool,
    pending: Mutex<Option<Pending>>,
}

unsafe impl Send for Cheevos {}
unsafe impl Sync for Cheevos {}

struct Pending {
    generation: u64,
    hash: String,
    in_flight: bool,
}

struct Job {
    request: Request,
    callback: ffi::ServerCallback,
    data: usize,
}

const MAX_NOTICES: usize = 8;

impl Cheevos {
    pub fn start(
        host: Option<&str>,
        transport: impl FnOnce(&str) -> Box<dyn Transport>,
    ) -> &'static Cheevos {
        crate::cdreader::init();
        let client = unsafe { ffi::rc_client_create(crate::memory::read_memory, server_call) };
        assert!(!client.is_null(), "rc_client_create failed");
        unsafe {
            ffi::rc_client_set_hardcore_enabled(client, 0);
            ffi::rc_client_set_allow_background_memory_reads(client, 0);
            ffi::rc_client_enable_logging(client, ffi::LOG_WARN, log);
            ffi::rc_client_set_event_handler(client, on_event);
            if let Some(host) = host.and_then(|h| CString::new(h).ok()) {
                ffi::rc_client_set_host(client, host.as_ptr());
            }
        }
        let transport: std::sync::Arc<dyn Transport> = transport(&user_agent(client)).into();
        let (tx, rx) = channel::<Job>();
        let this: &'static Cheevos = Box::leak(Box::new(Cheevos {
            client,
            jobs: Mutex::new(tx),
            transport: transport.clone(),
            notices: Mutex::new(VecDeque::new()),
            arrived: Condvar::new(),
            generation: AtomicU64::new(0),
            console: AtomicU32::new(0),
            logged_in: AtomicBool::new(false),
            pending: Mutex::new(None),
        }));
        unsafe { ffi::rc_client_set_userdata(client, this as *const Cheevos as *mut c_void) };
        std::thread::Builder::new()
            .name("slot-cheevos-http".into())
            .spawn(move || {
                for job in rx {
                    let mut response = transport.send(&job.request);
                    let len = response.body.len();
                    response.body.push(0);
                    unsafe {
                        ffi::slot_respond(
                            job.callback,
                            job.data as *mut c_void,
                            response.body.as_ptr().cast(),
                            len,
                            response.status,
                        )
                    };
                }
            })
            .expect("spawn cheevos http thread");
        this
    }

    pub fn transport(&self) -> &dyn Transport {
        &*self.transport
    }

    pub fn login(&self, credentials: &Credentials, done: impl FnOnce(Login) + Send + 'static) {
        let (Ok(user), Ok(secret)) = (
            CString::new(credentials.username.as_str()),
            CString::new(match &credentials.secret {
                Secret::Password(s) | Secret::Token(s) => s.as_str(),
            }),
        ) else {
            done(Login::Rejected {
                reason: "unreadable username or password".into(),
                needs_password: true,
            });
            return;
        };
        let done: Box<Box<dyn FnOnce(Login) + Send>> = Box::new(Box::new(done));
        let data = Box::into_raw(done).cast::<c_void>();
        unsafe {
            match credentials.secret {
                Secret::Password(_) => ffi::rc_client_begin_login_with_password(
                    self.client,
                    user.as_ptr(),
                    secret.as_ptr(),
                    on_login,
                    data,
                ),
                Secret::Token(_) => ffi::rc_client_begin_login_with_token(
                    self.client,
                    user.as_ptr(),
                    secret.as_ptr(),
                    on_login,
                    data,
                ),
            }
        };
    }

    pub fn logged_in(&self) -> bool {
        self.logged_in.load(Ordering::Acquire)
    }

    pub fn load_game(&self, hash: &str) -> u64 {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        *self.pending.lock().unwrap() = Some(Pending {
            generation,
            hash: hash.to_string(),
            in_flight: false,
        });
        self.retry_load();
        generation
    }

    pub fn retry_load(&self) {
        let mut pending = self.pending.lock().unwrap();
        let Some(p) = pending.as_mut().filter(|p| !p.in_flight) else {
            return;
        };
        let Ok(hash) = CString::new(p.hash.as_str()) else {
            *pending = None;
            return;
        };
        p.in_flight = true;
        let data = p.generation as usize as *mut c_void;
        drop(pending);
        unsafe { ffi::rc_client_begin_load_game(self.client, hash.as_ptr(), on_load, data) };
    }

    pub fn wants_load(&self) -> bool {
        self.pending
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|p| !p.in_flight)
    }

    pub fn unload_game(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.pending.lock().unwrap() = None;
        self.console.store(0, Ordering::Release);
        unsafe { ffi::rc_client_unload_game(self.client) };
    }

    pub fn game_console(&self) -> Option<u32> {
        Some(self.console.load(Ordering::Acquire)).filter(|&c| c != 0)
    }

    pub fn do_frame(&self) {
        unsafe { ffi::rc_client_do_frame(self.client) };
    }

    pub fn idle(&self) {
        unsafe { ffi::rc_client_idle(self.client) };
    }

    pub fn reset(&self) {
        unsafe { ffi::rc_client_reset(self.client) };
    }

    // --- Fragile: the in-game list, and signing in again as someone else ---

    /// The loaded game's achievements, unlocked first; empty with no game loaded.
    pub fn achievements(&self) -> Vec<Achievement> {
        let mut out: Vec<Achievement> = Vec::new();
        let data = (&mut out as *mut Vec<Achievement>).cast::<c_void>();
        unsafe { ffi::slot_achievements(self.client, collect, data) };
        out
    }

    pub fn logout(&self) {
        unsafe { ffi::rc_client_logout(self.client) };
        self.logged_in.store(false, Ordering::Release);
    }

    pub fn push(&self, notice: Notice) {
        let mut q = self.notices.lock().unwrap();
        if q.len() >= MAX_NOTICES {
            match q.iter().position(|n| !n.is_unlock()) {
                Some(i) => {
                    q.remove(i);
                }
                None if !notice.is_unlock() => return,
                None => {}
            }
        }
        q.push_back(notice);
        self.arrived.notify_all();
    }

    pub fn next_notice(&self, timeout: Duration) -> Option<Notice> {
        let q = self.notices.lock().unwrap();
        let (mut q, _) = self
            .arrived
            .wait_timeout_while(q, timeout, |q| q.is_empty())
            .unwrap();
        q.pop_front()
    }

    fn game(&self) -> Option<Game> {
        let mut g = ffi::SlotGame {
            title: std::ptr::null(),
            badge_url: std::ptr::null(),
            console: 0,
            unlocked: 0,
            total: 0,
        };
        if unsafe { ffi::slot_game_fields(self.client, &mut g) } == 0 {
            return None;
        }
        Some(Game {
            title: unsafe { ffi::text(g.title) },
            badge_url: unsafe { ffi::text(g.badge_url) },
            console: g.console,
            unlocked: g.unlocked,
            total: g.total,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Achievement {
    pub title: String,
    pub description: String,
    pub points: u32,
    pub unlocked: bool,
}

unsafe extern "C" fn collect(
    data: *mut c_void,
    title: *const c_char,
    description: *const c_char,
    points: u32,
    unlocked: c_int,
) {
    let out = &mut *data.cast::<Vec<Achievement>>();
    out.push(Achievement {
        title: ffi::text(title),
        description: ffi::text(description),
        points,
        unlocked: unlocked != 0,
    });
}

fn user_agent(client: *mut rc_client_t) -> String {
    let mut clause = [0 as c_char; 128];
    unsafe { ffi::rc_client_get_user_agent_clause(client, clause.as_mut_ptr(), clause.len()) };
    format!(
        "Fragile/{} ({}; {}) {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        unsafe { ffi::text(clause.as_ptr()) }
    )
}

unsafe fn owner<'a>(client: *const rc_client_t) -> Option<&'a Cheevos> {
    let p = ffi::rc_client_get_userdata(client) as *const Cheevos;
    p.as_ref()
}

unsafe extern "C" fn log(message: *const c_char, _client: *const rc_client_t) {
    eprintln!("slot: cheevos: {}", ffi::text(message));
}

unsafe extern "C" fn server_call(
    request: *const ffi::rc_api_request_t,
    callback: ffi::ServerCallback,
    data: *mut c_void,
    client: *mut rc_client_t,
) {
    let (mut url, mut post, mut content_type) =
        (std::ptr::null(), std::ptr::null(), std::ptr::null());
    ffi::slot_request_fields(request, &mut url, &mut post, &mut content_type);
    let request = Request {
        url: ffi::text(url),
        post: (!post.is_null()).then(|| ffi::text(post)),
        content_type: (!content_type.is_null()).then(|| ffi::text(content_type)),
    };
    let job = Job {
        request,
        callback,
        data: data as usize,
    };
    let sent = owner(client).is_some_and(|c| c.jobs.lock().unwrap().send(job).is_ok());
    if !sent {
        ffi::slot_respond(callback, data, c"".as_ptr(), 0, ffi::RETRYABLE_CLIENT_ERROR);
    }
}

unsafe extern "C" fn on_login(
    result: c_int,
    message: *const c_char,
    client: *mut rc_client_t,
    data: *mut c_void,
) {
    let done = Box::from_raw(data.cast::<Box<dyn FnOnce(Login) + Send>>());
    let Some(this) = owner(client) else {
        return;
    };
    let login = match result {
        ffi::RC_OK => {
            this.logged_in.store(true, Ordering::Release);
            Login::Ok {
                user: ffi::text(ffi::slot_user_name(client)),
                token: ffi::text(ffi::slot_user_token(client)),
            }
        }
        ffi::RC_NO_RESPONSE => Login::Unreachable,
        _ => Login::Rejected {
            reason: ffi::text(message),
            needs_password: matches!(result, ffi::RC_EXPIRED_TOKEN | ffi::RC_INVALID_CREDENTIALS),
        },
    };
    done(login);
}

unsafe extern "C" fn on_load(
    result: c_int,
    _message: *const c_char,
    client: *mut rc_client_t,
    data: *mut c_void,
) {
    let Some(this) = owner(client) else {
        return;
    };
    let generation = data as usize as u64;
    if this.generation.load(Ordering::Acquire) != generation {
        return;
    }
    let mut pending = this.pending.lock().unwrap();
    match result {
        ffi::RC_OK => {
            *pending = None;
            drop(pending);
            if let Some(game) = this.game() {
                this.console.store(game.console, Ordering::Release);
                this.push(notice::loaded(game));
            }
        }
        ffi::RC_NO_GAME_LOADED => {
            *pending = None;
            drop(pending);
            this.push(Notice::Unrecognised);
        }
        ffi::RC_ABORTED => *pending = None,
        _ => {
            if let Some(p) = pending.as_mut() {
                p.in_flight = false;
            }
        }
    }
}

unsafe extern "C" fn on_event(event: *const ffi::rc_client_event_t, client: *mut rc_client_t) {
    let Some(this) = owner(client) else {
        return;
    };
    let mut e = ffi::SlotEvent {
        kind: 0,
        points: 0,
        title: std::ptr::null(),
        description: std::ptr::null(),
        badge_url: std::ptr::null(),
        message: std::ptr::null(),
    };
    ffi::slot_event_fields(event, &mut e);
    let event = Event {
        kind: e.kind,
        title: ffi::text(e.title),
        description: ffi::text(e.description),
        points: e.points,
        badge_url: ffi::text(e.badge_url),
        message: ffi::text(e.message),
    };
    if let Some(n) = notice::from_event(event) {
        this.push(n);
    }
}
