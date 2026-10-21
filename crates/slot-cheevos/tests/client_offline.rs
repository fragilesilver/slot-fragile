// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/client_offline.rs.

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slot_cheevos::{
    game, memory, Cheevos, Credentials, Login, Notice, Request, Response, Secret, Transport,
};
use slot_retro::MemoryDescriptor;

const HASH: &str = "0123456789abcdef0123456789abcdef";

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":10,"SoftcoreScore":5,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;
const SETS: &str = r#"{"Success":true,"GameId":1234,"Title":"Sample Game","ConsoleId":5,"ImageIconUrl":"http://server/Images/112233.png","RichPresenceGameId":1234,"RichPresencePatch":"","Sets":[{"AchievementSetId":1111,"GameId":1234,"Title":null,"Type":"core","ImageIconUrl":"http://server/Images/112233.png","Achievements":[{"ID":5501,"Title":"First Step","Description":"Set the flag","Flags":3,"Points":5,"MemAddr":"0xH0010=1","Author":"A","BadgeName":"00234","Created":1,"Modified":1}],"Leaderboards":[]}]}"#;
const UNKNOWN: &str = r#"{"Success":true,"GameId":0,"Title":"","ConsoleId":0,"ImageIconUrl":"","RichPresenceGameId":0,"RichPresencePatch":"","Sets":[]}"#;
const OK: &str = r#"{"Success":true}"#;
const AWARD: &str = r#"{"Success":true,"Score":15,"SoftcoreScore":10,"AchievementID":5501,"AchievementsRemaining":0}"#;

#[derive(Clone)]
struct Fake {
    sent: Arc<Mutex<Vec<Request>>>,
    sets: &'static str,
    online: bool,
    award_delay: Duration,
}

impl Fake {
    fn new(sets: &'static str) -> Self {
        Self {
            sent: Arc::default(),
            sets,
            online: true,
            award_delay: Duration::ZERO,
        }
    }

    fn sent_with(&self, needle: &str) -> Vec<String> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .filter_map(|r| r.post.clone())
            .filter(|p| p.contains(needle))
            .collect()
    }
}

impl Transport for Fake {
    fn send(&self, request: &Request) -> Response {
        self.sent.lock().unwrap().push(request.clone());
        if !self.online {
            return Response::unreachable();
        }
        let post = request.post.as_deref().unwrap_or_default();
        let body = match post.split('&').find_map(|kv| kv.strip_prefix("r=")) {
            Some("login2") => LOGIN,
            Some("achievementsets") => self.sets,
            Some("awardachievement") => {
                std::thread::sleep(self.award_delay);
                AWARD
            }
            _ => OK,
        };
        Response {
            status: 200,
            body: body.as_bytes().to_vec(),
        }
    }

    fn fetch(&self, _url: &str) -> Option<Vec<u8>> {
        None
    }
}

fn password() -> Credentials {
    Credentials {
        username: "Player".into(),
        secret: Secret::Password("hunter2".into()),
        host: None,
    }
}

fn login(c: &Cheevos) -> Login {
    let (tx, rx) = channel();
    c.login(&password(), move |l| tx.send(l).unwrap());
    rx.recv_timeout(Duration::from_secs(5))
        .expect("login answered")
}

fn iwram(ram: &mut [u8]) -> Vec<MemoryDescriptor> {
    vec![MemoryDescriptor {
        ptr: ram.as_mut_ptr() as usize,
        start: 0x3000000,
        len: ram.len(),
        ..Default::default()
    }]
}

fn pump_until(c: &Cheevos, want: impl Fn(&Notice) -> bool) -> Notice {
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end {
        c.do_frame();
        if let Some(n) = c.next_notice(Duration::from_millis(10)) {
            if want(&n) {
                return n;
            }
        }
    }
    panic!("no matching notice");
}

fn drain(c: &Cheevos) -> Vec<Notice> {
    std::iter::from_fn(|| c.next_notice(Duration::from_millis(20))).collect()
}

#[test]
fn a_password_login_hands_back_the_token_to_remember() {
    let fake = Fake::new(SETS);
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert_eq!(
        login(c),
        Login::Ok {
            user: "Player".into(),
            token: "ApiToken".into()
        }
    );
    assert!(c.logged_in());
}

#[test]
fn a_known_game_loads_and_unlocks_in_softcore() {
    let fake = Fake::new(SETS);
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert!(matches!(login(c), Login::Ok { .. }));

    let mut ram = vec![0u8; 0x8000];
    assert!(memory::refresh(&iwram(&mut ram), game::GBA));
    c.load_game(HASH);
    let loaded = pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    assert_eq!(
        loaded,
        Notice::GameLoaded {
            title: "Sample Game".into(),
            unlocked: 0,
            total: 1,
            badge_url: "http://server/Images/112233.png".into(),
        }
    );
    assert_eq!(c.game_console(), Some(game::GBA));

    for _ in 0..3 {
        c.do_frame();
    }
    ram[0x10] = 1;
    let unlocked = pump_until(c, Notice::is_unlock);
    assert!(
        matches!(unlocked, Notice::Unlocked { points: 5, ref title, .. } if title == "First Step")
    );

    let end = Instant::now() + Duration::from_secs(5);
    while fake.sent_with("r=awardachievement").is_empty() && Instant::now() < end {
        c.idle();
        std::thread::sleep(Duration::from_millis(10));
    }
    let awards = fake.sent_with("r=awardachievement");
    assert_eq!(awards.len(), 1);
    assert!(awards[0].contains("h=0"), "{}", awards[0]);
    assert!(fake
        .sent_with("r=startsession")
        .iter()
        .all(|s| s.contains("h=0")));
    c.unload_game();
    memory::clear();
}

#[test]
fn a_reset_makes_a_trigger_wait_for_its_condition_to_go_false_first() {
    let fake = Fake::new(SETS);
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert!(matches!(login(c), Login::Ok { .. }));

    let mut ram = vec![0u8; 0x8000];
    memory::refresh(&iwram(&mut ram), game::GBA);
    c.load_game(HASH);
    pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    for _ in 0..3 {
        c.do_frame();
    }

    ram[0x10] = 1;
    c.reset();
    for _ in 0..10 {
        c.do_frame();
    }
    assert!(!drain(c).iter().any(Notice::is_unlock));

    ram[0x10] = 0;
    c.do_frame();
    ram[0x10] = 1;
    pump_until(c, Notice::is_unlock);
    c.unload_game();
    memory::clear();
}

#[test]
fn an_unknown_hash_is_reported_as_unrecognised() {
    let fake = Fake::new(UNKNOWN);
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert!(matches!(login(c), Login::Ok { .. }));
    c.load_game(HASH);
    pump_until(c, |n| *n == Notice::Unrecognised);
    assert!(!c.wants_load());
}

#[test]
fn no_network_leaves_login_unreachable_and_the_load_waiting_to_retry() {
    let mut fake = Fake::new(SETS);
    fake.online = false;
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert_eq!(login(c), Login::Unreachable);
    assert!(!c.logged_in());
    c.load_game(HASH);
    let end = Instant::now() + Duration::from_secs(5);
    while !c.wants_load() && Instant::now() < end {
        c.idle();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(c.wants_load());
}

#[test]
fn unloading_drops_a_pending_load() {
    let mut fake = Fake::new(SETS);
    fake.online = false;
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    c.load_game(HASH);
    c.unload_game();
    assert!(!c.wants_load());
    assert_eq!(c.game_console(), None);
}

fn unlock_the_only_achievement(fake: &Fake) -> (&'static Cheevos, Vec<u8>) {
    let f = fake.clone();
    let c = Cheevos::start(None, move |_| Box::new(f));
    assert!(matches!(login(c), Login::Ok { .. }));
    let mut ram = vec![0u8; 0x8000];
    memory::refresh(&iwram(&mut ram), game::GBA);
    c.load_game(HASH);
    pump_until(c, |n| matches!(n, Notice::GameLoaded { .. }));
    for _ in 0..3 {
        c.do_frame();
    }
    ram[0x10] = 1;
    pump_until(c, Notice::is_unlock);
    (c, ram)
}

#[test]
fn ejecting_once_the_last_award_lands_but_before_the_next_frame_survives() {
    let fake = Fake::new(SETS);
    let (c, _ram) = unlock_the_only_achievement(&fake);
    let end = Instant::now() + Duration::from_secs(5);
    while fake.sent_with("r=awardachievement").is_empty() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(100));
    c.unload_game();
    memory::clear();
    assert!(drain(c)
        .iter()
        .any(|n| matches!(n, Notice::Mastered { .. })));
}

#[test]
fn ejecting_before_the_last_award_is_answered_survives() {
    let mut fake = Fake::new(SETS);
    fake.award_delay = Duration::from_millis(300);
    let (c, _ram) = unlock_the_only_achievement(&fake);
    c.unload_game();
    memory::clear();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(fake.sent_with("r=awardachievement").len(), 1);
}
