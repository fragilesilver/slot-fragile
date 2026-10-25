// Ported from cloudwithax/unslot@6339c18, crates/slot/src/scrape.rs. Changed: Fragile's six
// systems, the internet through ureq (as RetroAchievements has it) rather than BaseOS's curl, a
// game looked for by its file name (Fragile has no arcade sets), the art kept to draw again in
// another style, LaunchBox cart front indexing and cropping (ported from pvaibhav/slot), and
// offline Artwork.pack support.

//! Labels for the card's games, found online and drawn into `System/Cache/Scraper/Labels`, which
//! the shelf reads where `Labels/` has none of the player's own.
//!
//! A game is known by its CRC32 and by its file name, both, for coverage: the CRC finds a dump
//! however it is named, the name finds one the database does not list. libretro's databases give
//! the names, and libretro-thumbnails the logos and box art, for every system and with no account.

pub mod catalog;
pub mod crop;
pub mod pack;

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use slot_store::{Cart, LabelStyle, Platform, SCRAPED_LABELS};

/// The databases fetched once and kept, under the card's System folder with the label cache.
pub const CACHE: &str = "System/Cache/Scraper";
const DATS: &str = "https://raw.githubusercontent.com/libretro/libretro-database/master";
const THUMBS: &str = "https://thumbnails.libretro.com";
/// Bigger than any cartridge; a disc is matched by name instead of read whole.
const CRC_MAX: u64 = 64 << 20;
/// How many names a game is looked for under before it is given up on.
const NAMES: usize = 4;

/// libretro's sets for a system's games, the one to try first first.
pub fn source(platform: Platform) -> Vec<&'static str> {
    match platform {
        Platform::Gba => vec!["Nintendo - Game Boy Advance"],
        Platform::Gb => vec!["Nintendo - Game Boy", "Nintendo - Game Boy Color"],
        Platform::Gbc => vec!["Nintendo - Game Boy Color", "Nintendo - Game Boy"],
        Platform::Nes => vec!["Nintendo - Nintendo Entertainment System"],
        Platform::Snes => vec!["Nintendo - Super Nintendo Entertainment System"],
        Platform::PlayStation => vec!["Sony - PlayStation"],
        Platform::Nds => vec!["Nintendo - Nintendo DS"],
    }
}

/// The disc systems', whose databases libretro keeps from redump rather than no-intro.
const REDUMP: &[&str] = &["Sony - PlayStation"];

fn dat_url(set: &str) -> String {
    let path = match REDUMP.contains(&set) {
        true => format!("metadat/redump/{set}"),
        false => format!("metadat/no-intro/{set}"),
    };
    format!("{DATS}/{}.dat", encode(&path).replace("%2F", "/"))
}

/// libretro-thumbnails names a picture after the game, with what its filesystem forbids made `_`.
pub fn thumbnail_name(game: &str) -> String {
    game.chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect()
}

fn thumb_url(set: &str, kind: &str, game: &str) -> String {
    format!(
        "{THUMBS}/{}/{kind}/{}.png",
        encode(set),
        encode(&thumbnail_name(game))
    )
}

/// Percent-encoding for a URL path segment.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A libretro database: which game each CRC32 is, and every game's name.
#[derive(Default, Debug)]
pub struct Dat {
    by_crc: HashMap<u32, String>,
    by_title: HashMap<String, Vec<String>>,
    names: std::collections::HashSet<String>,
}

/// Reads a clrmamepro dat as libretro writes them: `game (` blocks with a `name` and `rom` lines.
pub fn parse_dat(text: &str) -> Dat {
    let mut dat = Dat::default();
    let mut game: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with("game (") {
            game = None;
        } else if let Some(rest) = line.strip_prefix("name \"") {
            let name = rest.trim_end_matches('"').to_string();
            if dat.names.insert(name.to_lowercase()) {
                dat.by_title
                    .entry(title_key(&name))
                    .or_default()
                    .push(name.clone());
            }
            game = Some(name);
        } else if line.starts_with("rom (") {
            let crc = line
                .split_whitespace()
                .skip_while(|w| *w != "crc")
                .nth(1)
                .and_then(|c| u32::from_str_radix(c, 16).ok());
            if let (Some(crc), Some(name)) = (crc, &game) {
                dat.by_crc.entry(crc).or_insert_with(|| name.clone());
            }
        }
    }
    dat
}

/// A name with its tags and punctuation gone, so `Pokemon - Emerald Version (USA, Europe)` and
/// `pokemon_emerald_version` meet.
pub fn title_key(name: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            c if depth == 0 && c.is_alphanumeric() => out.extend(c.to_lowercase()),
            _ => {}
        }
    }
    out
}

/// How much a dump's tags are wanted: a release before a beta, the common regions first.
fn rank(name: &str) -> usize {
    let lower = name.to_lowercase();
    let region = ["(usa", "(world", "(europe", "(japan"]
        .iter()
        .position(|r| lower.contains(r))
        .unwrap_or(4);
    let unfinished = ["(beta", "(proto", "(demo", "(sample", "(pirate", "(hack"]
        .iter()
        .any(|t| lower.contains(t));
    region + usize::from(unfinished) * 10
}

impl Dat {
    /// The names a game might be listed under, best first: its CRC's, its file name as it is,
    /// then the games that share its title once tags are set aside.
    pub fn names_for(&self, crc: Option<u32>, stem: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut add = |n: &str| {
            if !out.iter().any(|o| o == n) {
                out.push(n.to_string());
            }
        };
        if let Some(name) = crc.and_then(|c| self.by_crc.get(&c)) {
            add(name);
        }
        if self.names.contains(&stem.to_lowercase()) {
            add(stem);
        }
        let mut alike = self
            .by_title
            .get(&title_key(stem))
            .cloned()
            .unwrap_or_default();
        alike.sort_by_key(|n| rank(n));
        for name in alike.iter().take(NAMES) {
            add(name);
        }
        out
    }
}

/// The CRC32 of a cartridge, as the databases list it: of the whole file.
pub fn crc_of(rom: &Path) -> Option<u32> {
    let mut f = std::fs::File::open(rom).ok()?;
    if f.metadata().ok()?.len() > CRC_MAX {
        return None;
    }
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = vec![0u8; 64 << 10];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize())
}

fn is_disc(rom: &Path) -> bool {
    let ext = rom.extension().and_then(|e| e.to_str()).unwrap_or_default();
    ["cue", "chd", "pbp", "m3u", "iso"]
        .iter()
        .any(|d| ext.eq_ignore_ascii_case(d))
}

/// Fetching, as the worker does it.
pub trait Fetch: Send {
    /// What is at `url`: `Ok(None)` when there is nothing there, `Err` when the network failed.
    fn get(&mut self, url: &str) -> Result<Option<Vec<u8>>, String>;
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ScrapeView {
    pub running: bool,
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
    /// The games given a label since slot started, in the order they got one. The app puts
    /// each on its cart as it appears here.
    pub made: Vec<(Platform, String)>,
    /// Games looked for and not found.
    pub missed: usize,
    pub error: Option<String>,
}

/// Fragile: whether this build asks the real servers: the device does, and anywhere else
/// does with `SLOT_SCRAPE_LIVE=1`.
pub fn live() -> bool {
    cfg!(feature = "device") || std::env::var_os("SLOT_SCRAPE_LIVE").is_some()
}

/// Fragile: a game to label, and how.
pub type Job = (Cart, LabelStyle);

/// The handle the app keeps.
pub struct Scraper {
    jobs: Sender<Vec<Job>>,
    view: Arc<Mutex<ScrapeView>>,
}

impl Scraper {
    pub fn spawn(fetch: Box<dyn Fetch>, root: &Path) -> Scraper {
        let (jobs, rx) = channel();
        let view = Arc::new(Mutex::new(ScrapeView::default()));
        let shared = view.clone();
        let root = root.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("slot-scraper".into())
            .spawn(move || work(fetch, root, rx, shared));
        if let Err(e) = spawned {
            eprintln!("slot: scraper: {e}");
        }
        Scraper { jobs, view }
    }

    /// The scraper for this build: the internet on the device, pretend art anywhere else.
    pub fn for_card(root: &Path) -> Scraper {
        let fetch: Box<dyn Fetch> = match live() {
            true => Box::new(Http::new()),
            false => Box::new(SimFetch),
        };
        Scraper::spawn(fetch, root)
    }

    /// Finds labels for `carts`, each drawn its way. A game looked for before is drawn again
    /// from the art kept on the card, without the internet.
    pub fn fetch(&self, carts: Vec<Job>) {
        let _ = self.jobs.send(carts);
    }

    pub fn view(&self) -> ScrapeView {
        self.view.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

fn work(
    mut fetch: Box<dyn Fetch>,
    root: PathBuf,
    jobs: Receiver<Vec<Job>>,
    view: Arc<Mutex<ScrapeView>>,
) {
    // Finding art goes on while a game plays, so it gives way to the game for the CPU. Linux
    // keeps a nice value for each thread, and 0 is this one.
    #[cfg(target_os = "linux")]
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
    }
    let set =
        |f: &mut dyn FnMut(&mut ScrapeView)| f(&mut view.lock().unwrap_or_else(|e| e.into_inner()));
    let mut dats: HashMap<&'static str, Dat> = HashMap::new();
    while let Ok(carts) = jobs.recv() {
        let total = carts.len();
        set(&mut |v| {
            v.running = true;
            v.done = 0;
            v.total = total;
            v.error = None;
        });
        for (cart, style) in carts {
            // Fragile: a label already drawn in its style is left as it is.
            let label = root
                .join(SCRAPED_LABELS)
                .join(cart.platform.dir_name())
                .join(format!("{}.png", cart.stem));
            let drawn = std::fs::read_to_string(art_path(&root, &cart, STYLE)).ok();
            if label.exists() && drawn.as_deref() == Some(style.key()) {
                set(&mut |v| v.done += 1);
                continue;
            }
            set(&mut |v| v.current = Some(cart.stem.clone()));
            let art = art_for(&mut *fetch, &root, &mut dats, &cart);
            match art.map(|art| draw(&cart, style, &art)) {
                Ok(Some(png)) => {
                    // Fragile: beside the player's own labels, never over them.
                    let dir = root.join(SCRAPED_LABELS).join(cart.platform.dir_name());
                    let path = dir.join(format!("{}.png", cart.stem));
                    let written =
                        std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, png));
                    match written {
                        Ok(()) => {
                            let _ = std::fs::write(art_path(&root, &cart, STYLE), style.key());
                            set(&mut |v| v.made.push((cart.platform, cart.stem.clone())))
                        }
                        Err(e) => set(&mut |v| v.error = Some(format!("{}: {e}", path.display()))),
                    }
                }
                Ok(None) => set(&mut |v| v.missed += 1),
                Err(e) => {
                    eprintln!("slot: scraper: {e}");
                    set(&mut |v| v.error = Some(e.clone()));
                    break;
                }
            }
            set(&mut |v| v.done += 1);
        }
        set(&mut |v| {
            v.running = false;
            v.current = None;
        });
    }
}

/// Fragile: what was found for a game, kept on the card so a new style draws without the
/// internet: its logo and its box.
#[derive(Default, Debug)]
pub struct Art {
    pub logo: Option<Vec<u8>>,
    pub boxart: Option<Vec<u8>>,
    pub cart: Option<Vec<u8>>,
}

/// Where a game's art is kept, under the scraper's cache.
pub const ART: &str = "System/Cache/Scraper/Art";

fn art_path(root: &Path, cart: &Cart, kind: &str) -> PathBuf {
    root.join(ART)
        .join(cart.platform.dir_name())
        .join(format!("{}.{kind}", cart.stem))
}

/// Says a game was looked for, whatever was found, so it is not asked for again.
const LOOKED: &str = "looked";
/// The style a game's label was last drawn in.
const STYLE: &str = "style";

/// A game's art, from the card when it was looked for before, else from the internet.
fn art_for(
    fetch: &mut dyn Fetch,
    root: &Path,
    dats: &mut HashMap<&'static str, Dat>,
    cart: &Cart,
) -> Result<Art, String> {
    let kinds = ["logo.png", "box.png", "cart.png"];
    if art_path(root, cart, LOOKED).exists() {
        let [logo, boxart, cart] = kinds.map(|k| std::fs::read(art_path(root, cart, k)).ok());
        return Ok(Art { logo, boxart, cart });
    }
    let art = find_art(fetch, root, dats, cart)?;
    if let Some(dir) = art_path(root, cart, LOOKED).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    for (kind, bytes) in kinds.iter().zip([&art.logo, &art.boxart, &art.cart]) {
        if let Some(bytes) = bytes {
            let _ = std::fs::write(art_path(root, cart, kind), bytes);
        }
    }
    let _ = std::fs::write(art_path(root, cart, LOOKED), b"");
    Ok(art)
}

/// A game's art from offline packs, LaunchBox CDN, and libretro-thumbnails.
fn find_art(
    fetch: &mut dyn Fetch,
    root: &Path,
    dats: &mut HashMap<&'static str, Dat>,
    cart: &Cart,
) -> Result<Art, String> {
    let crc = (!is_disc(&cart.rom)).then(|| crc_of(&cart.rom)).flatten();
    let png = |bytes: Option<Vec<u8>>| bytes.filter(|b| b.starts_with(b"\x89PNG"));
    let mut art = Art::default();

    // 1. Check offline pack first (Labels/Artwork.pack)
    if let Some(c) = crc {
        if let Some(pack_label) = pack::find_in_pack(root, c, cart.platform) {
            art.cart = Some(pack_label);
        }
    }

    // 2. Check LaunchBox CDN for cart front (for cartridge systems)
    if art.cart.is_none() && !cart.platform.spec().discs {
        if let Some(c) = crc {
            for url in catalog::lookup_launchbox(c) {
                if let Ok(Some(bytes)) = fetch.get(&url) {
                    if let Some(label_png) = crop::prepare_cart_label(&bytes, cart.platform, 2) {
                        art.cart = Some(label_png);
                        break;
                    }
                }
            }
        }
    }

    // 3. Libretro thumbnails for logo and boxart
    for set in source(cart.platform) {
        if art.logo.is_some() && art.boxart.is_some() {
            break;
        }
        let dat = load_dat(fetch, root, dats, set)?;
        let stem = cart.stem.as_str();
        let mut names = dat.names_for(crc, stem);
        if !names.iter().any(|n| n == stem) {
            names.push(stem.to_string());
        }
        for name in names.iter().take(NAMES + 2) {
            if art.boxart.is_none() {
                art.boxart = png(fetch.get(&thumb_url(set, "Named_Boxarts", name))?);
            }
            if art.logo.is_none() {
                art.logo = png(fetch.get(&thumb_url(set, "Named_Logos", name))?);
            }
            if art.logo.is_some() && art.boxart.is_some() {
                break;
            }
        }
    }
    Ok(art)
}

/// Fragile: a game's label in `style`, from what was found for it, at twice the label's size;
/// None when nothing was. A style with no art of its own takes the next: the cart's printed
/// label, the box, then the logo.
pub fn draw(cart: &Cart, style: LabelStyle, art: &Art) -> Option<Vec<u8>> {
    let (w, h) = slot_ui::label_size(cart);
    let (w, h) = (w * 2, h * 2);
    let hue = slot_ui::hue_of(slot_ui::label_colour(&slot_ui::label_text(cart)));
    let cover =
        |bytes: &Option<Vec<u8>>| bytes.as_deref().and_then(|b| slot_ui::box_label(b, w, h));
    let logo = || slot_ui::scraped_label(art.logo.as_deref(), art.boxart.as_deref(), hue, w, h);
    let disc = cart.platform.spec().discs;
    match style {
        LabelStyle::Art | LabelStyle::Label if disc => cover(&art.cart)
            .or_else(|| cover(&art.boxart))
            .or_else(logo),
        LabelStyle::Label => cover(&art.cart).or_else(logo),
        LabelStyle::Art => cover(&art.boxart).or_else(logo),
        LabelStyle::Logo => logo(),
    }
}

/// A set's database, from memory, the card, or libretro, in that order.
fn load_dat<'a>(
    fetch: &mut dyn Fetch,
    root: &Path,
    dats: &'a mut HashMap<&'static str, Dat>,
    set: &'static str,
) -> Result<&'a Dat, String> {
    if !dats.contains_key(set) {
        let path = root.join(CACHE).join(format!("{set}.dat"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => {
                let bytes = fetch.get(&dat_url(set))?.unwrap_or_default();
                if !bytes.is_empty() {
                    let _ = std::fs::create_dir_all(root.join(CACHE));
                    let _ = std::fs::write(&path, &bytes);
                }
                String::from_utf8_lossy(&bytes).into_owned()
            }
        };
        dats.insert(set, parse_dat(&text));
    }
    Ok(&dats[set])
}

/// The internet, through ureq, as RetroAchievements reaches it.
struct Http(ureq::Agent);

/// A libretro database is a few megabytes; nothing fetched is let past this.
const MAX_BODY: u64 = 64 << 20;

impl Http {
    fn new() -> Http {
        Http(
            ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(90))
                .user_agent("fragile")
                .build(),
        )
    }
}

impl Fetch for Http {
    fn get(&mut self, url: &str) -> Result<Option<Vec<u8>>, String> {
        match self.0.get(url).call() {
            Ok(answer) => {
                let mut body = Vec::new();
                answer
                    .into_reader()
                    .take(MAX_BODY)
                    .read_to_end(&mut body)
                    .map_err(|_| "The art servers could not be reached".to_string())?;
                Ok(Some(body))
            }
            // ScreenScraper says it does not know a game with a 404 or a 400.
            Err(ureq::Error::Status(404 | 400 | 430, _)) => Ok(None),
            Err(ureq::Error::Status(code, _)) => Err(format!("The art server answered {code}")),
            Err(ureq::Error::Transport(_)) => Err("The art servers could not be reached".into()),
        }
    }
}

/// Pretend art for the desktop and the tests: a libretro database knowing one game per system by
/// the CRC of a ROM of eight zero bytes, and a logo and box for each game it knows.
pub struct SimFetch;

impl SimFetch {
    pub const CRC_ZEROS8: u32 = 0x6522_DF69;

    pub fn logo() -> Vec<u8> {
        sim_png(32, 8, [255, 255, 255, 255])
    }
}

fn sim_png(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let data: Vec<u8> = (0..w * h).flat_map(|_| px).collect();
        if let Ok(mut writer) = enc.write_header() {
            let _ = writer.write_image_data(&data);
        }
    }
    out
}

impl Fetch for SimFetch {
    fn get(&mut self, url: &str) -> Result<Option<Vec<u8>>, String> {
        if url.ends_with(".dat") {
            return Ok(Some(
                format!(
                    "game (\n\tname \"Pretend Game (USA)\"\n\trom ( name \"Pretend Game (USA).bin\" size 8 crc {:08X} )\n)\n",
                    SimFetch::CRC_ZEROS8
                )
                .into_bytes(),
            ));
        }
        if url.contains("Pretend%20Game%20%28USA%29") {
            return Ok(Some(match url.contains("Named_Logos") {
                true => SimFetch::logo(),
                false => sim_png(16, 16, [200, 40, 40, 255]),
            }));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = r#"clrmamepro (
	name "Nintendo - Game Boy Advance"
)

game (
	name "Pokemon - Emerald Version (USA, Europe)"
	rom ( name "Pokemon - Emerald Version (USA, Europe).gba" size 16777216 crc 1F1C08FB md5 X sha1 Y )
)
game (
	name "Pokemon - Emerald Version (Japan)"
	rom ( name "Pokemon - Emerald Version (Japan).gba" size 16777216 crc 4881F3F8 md5 X sha1 Y )
)
game (
	name "Pokemon - Emerald Version (USA) (Beta)"
	rom ( name "Pokemon - Emerald Version (USA) (Beta).gba" size 16777216 crc 0BADBEEF md5 X sha1 Y )
)
"#;

    #[test]
    fn a_crc_finds_its_game_however_the_file_is_named() {
        let dat = parse_dat(DAT);
        assert_eq!(
            dat.names_for(Some(0x4881_F3F8), "emerald hack final")[0],
            "Pokemon - Emerald Version (Japan)"
        );
    }

    #[test]
    fn a_name_finds_a_game_whose_crc_is_unknown_best_release_first() {
        let dat = parse_dat(DAT);
        let names = dat.names_for(Some(0x1234_5678), "pokemon_emerald_version");
        assert_eq!(
            names,
            [
                "Pokemon - Emerald Version (USA, Europe)",
                "Pokemon - Emerald Version (Japan)",
                "Pokemon - Emerald Version (USA) (Beta)",
            ]
        );
        assert_eq!(
            dat.names_for(None, "Pokemon - Emerald Version (Japan)")[0],
            "Pokemon - Emerald Version (Japan)",
            "an exact file name did not come first"
        );
    }

    #[test]
    fn thumbnails_are_named_and_addressed_as_libretro_has_them() {
        assert_eq!(
            thumbnail_name("Lilo & Stitch: Trouble?"),
            "Lilo _ Stitch_ Trouble_"
        );
        assert_eq!(
            thumb_url("Nintendo - Game Boy", "Named_Logos", "Tetris (World) (Rev 1)"),
            "https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Logos/Tetris%20%28World%29%20%28Rev%201%29.png"
        );
        assert!(dat_url("Sony - PlayStation").contains("/redump/Sony%20-%20PlayStation.dat"));
    }

    #[test]
    fn each_system_looks_in_its_own_set_and_a_game_boy_game_in_the_colour_one_too() {
        assert_eq!(
            source(Platform::Gb),
            ["Nintendo - Game Boy", "Nintendo - Game Boy Color"]
        );
        assert_eq!(source(Platform::PlayStation), ["Sony - PlayStation"]);
        assert!(dat_url("Sony - PlayStation").contains("/redump/Sony%20-%20PlayStation.dat"));
        assert!(dat_url("Nintendo - Game Boy Advance").contains("/no-intro/"));
    }

    #[test]
    fn a_game_the_database_knows_gets_a_label_on_the_card() {
        let d = tempfile::tempdir().unwrap();
        let games = d.path().join("Games/GBA");
        std::fs::create_dir_all(&games).unwrap();
        std::fs::write(games.join("whatever.gba"), [0u8; 8]).unwrap();
        std::fs::write(games.join("unknown.gba"), [1u8; 8]).unwrap();
        let carts = slot_store::scan(d.path()).unwrap();
        let s = Scraper::spawn(Box::new(SimFetch), d.path());
        s.fetch(carts.into_iter().map(|c| (c, LabelStyle::Art)).collect());
        let start = std::time::Instant::now();
        let v = loop {
            let v = s.view();
            if (!v.running && v.done == 2) || start.elapsed().as_secs() > 10 {
                break v;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        assert_eq!(v.made, [(Platform::Gba, "whatever".to_string())]);
        assert_eq!(v.missed, 1);
        let label = d.path().join(SCRAPED_LABELS).join("GBA/whatever.png");
        assert!(std::fs::read(&label).unwrap().starts_with(b"\x89PNG"));
        assert!(
            d.path()
                .join(CACHE)
                .join("Nintendo - Game Boy Advance.dat")
                .exists(),
            "the database was not kept"
        );
    }
}
