use std::path::Path;

use crate::atomic::atomic_write;
use crate::platform::Platform;

const CONTROLS_FILE: &str = "controls.txt";
const CAROUSEL_FILE: &str = "carousel.txt";
const SHORTCUTS_FILE: &str = "shortcuts.txt";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Device {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
}

impl Device {
    pub fn leads(self) -> bool {
        matches!(self, Device::Select | Device::Start)
    }

    pub const ALL: [Device; 14] = [
        Device::Up,
        Device::Down,
        Device::Left,
        Device::Right,
        Device::A,
        Device::B,
        Device::X,
        Device::Y,
        Device::L1,
        Device::R1,
        Device::L2,
        Device::R2,
        Device::Select,
        Device::Start,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Device::Up => "up",
            Device::Down => "down",
            Device::Left => "left",
            Device::Right => "right",
            Device::A => "a",
            Device::B => "b",
            Device::X => "x",
            Device::Y => "y",
            Device::L1 => "l1",
            Device::R1 => "r1",
            Device::L2 => "l2",
            Device::R2 => "r2",
            Device::Select => "select",
            Device::Start => "start",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Device::Up => "UP",
            Device::Down => "DOWN",
            Device::Left => "LEFT",
            Device::Right => "RIGHT",
            Device::A => "A",
            Device::B => "B",
            Device::X => "X",
            Device::Y => "Y",
            Device::L1 => "L1",
            Device::R1 => "R1",
            Device::L2 => "L2",
            Device::R2 => "R2",
            Device::Select => "SELECT",
            Device::Start => "START",
        }
    }

    fn parse(key: &str) -> Option<Device> {
        Device::ALL
            .into_iter()
            .find(|d| d.key().eq_ignore_ascii_case(key.trim()))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Modifier {
    Select,
    Menu,
    Button(Device),
}

impl Modifier {
    fn key(self) -> &'static str {
        match self {
            Modifier::Select => "select",
            Modifier::Menu => "menu",
            Modifier::Button(d) => d.key(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Modifier::Select => "SEL",
            Modifier::Menu => "MENU",
            Modifier::Button(d) => d.label(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Binding {
    pub modifier: Option<Modifier>,
    pub button: Device,
}

impl Binding {
    pub fn plain(button: Device) -> Option<Binding> {
        Some(Binding {
            modifier: None,
            button,
        })
    }

    pub fn with(modifier: Modifier, button: Device) -> Option<Binding> {
        Some(Binding {
            modifier: Some(modifier),
            button,
        })
    }

    pub fn pair(first: Device, second: Device) -> Option<Binding> {
        match (first, second) {
            _ if first == second => Binding::plain(first),
            (Device::Select, other) | (other, Device::Select) => {
                Binding::with(Modifier::Select, other)
            }
            (Device::Start, other) | (other, Device::Start) => {
                Binding::with(Modifier::Button(Device::Start), other)
            }
            _ if (first as usize) < (second as usize) => {
                Binding::with(Modifier::Button(first), second)
            }
            _ => Binding::with(Modifier::Button(second), first),
        }
    }

    pub fn text(binding: Option<Binding>) -> String {
        match binding {
            None => "NONE".to_string(),
            Some(Binding {
                modifier: None,
                button,
            }) => button.label().to_string(),
            Some(Binding {
                modifier: Some(m),
                button,
            }) => format!("{}+{}", m.label(), button.label()),
        }
    }

    fn key(binding: Option<Binding>) -> String {
        match binding {
            None => "none".to_string(),
            Some(Binding {
                modifier: None,
                button,
            }) => button.key().to_string(),
            Some(Binding {
                modifier: Some(m),
                button,
            }) => format!("{}+{}", m.key(), button.key()),
        }
    }

    fn parse(value: &str) -> Option<Option<Binding>> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("none") {
            return Some(None);
        }
        let Some((m, b)) = value.split_once('+') else {
            return Some(Binding::plain(Device::parse(value)?));
        };
        let button = Device::parse(b)?;
        if m.trim().eq_ignore_ascii_case(Modifier::Menu.key()) {
            return Some(Binding::with(Modifier::Menu, button));
        }
        Some(Binding::pair(Device::parse(m)?, button))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum GameButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
    L,
    R,
    TurboA,
    TurboB,
    // --- extras: buttons of a pad that has them, for a system whose row says so ---
    X,
    Y,
    L2,
    R2,
}

impl GameButton {
    /// The buttons past upstream's twelve: X and Y where turbo does not take them, L2 and R2
    /// where the games read them.
    pub const EXTRA: [GameButton; 4] =
        [GameButton::X, GameButton::Y, GameButton::L2, GameButton::R2];

    /// Every button, upstream's and the extras, in index order.
    pub const EVERY: [GameButton; 16] = [
        GameButton::Up,
        GameButton::Down,
        GameButton::Left,
        GameButton::Right,
        GameButton::A,
        GameButton::B,
        GameButton::Select,
        GameButton::Start,
        GameButton::L,
        GameButton::R,
        GameButton::TurboA,
        GameButton::TurboB,
        GameButton::X,
        GameButton::Y,
        GameButton::L2,
        GameButton::R2,
    ];

    /// A pad of its own X and Y, and L and R: a Super Nintendo's.
    const FACE: [GameButton; 12] = [
        GameButton::Up,
        GameButton::Down,
        GameButton::Left,
        GameButton::Right,
        GameButton::A,
        GameButton::B,
        GameButton::X,
        GameButton::Y,
        GameButton::Select,
        GameButton::Start,
        GameButton::L,
        GameButton::R,
    ];

    /// The face pad with triggers too: a PlayStation's.
    const FACE_TRIGGERS: [GameButton; 14] = [
        GameButton::Up,
        GameButton::Down,
        GameButton::Left,
        GameButton::Right,
        GameButton::A,
        GameButton::B,
        GameButton::X,
        GameButton::Y,
        GameButton::Select,
        GameButton::Start,
        GameButton::L,
        GameButton::R,
        GameButton::L2,
        GameButton::R2,
    ];

    pub const ALL: [GameButton; 12] = [
        GameButton::Up,
        GameButton::Down,
        GameButton::Left,
        GameButton::Right,
        GameButton::A,
        GameButton::B,
        GameButton::Select,
        GameButton::Start,
        GameButton::L,
        GameButton::R,
        GameButton::TurboA,
        GameButton::TurboB,
    ];

    const GAME_BOY: [GameButton; 10] = [
        GameButton::Up,
        GameButton::Down,
        GameButton::Left,
        GameButton::Right,
        GameButton::A,
        GameButton::B,
        GameButton::Select,
        GameButton::Start,
        GameButton::TurboA,
        GameButton::TurboB,
    ];

    pub fn of(platform: Platform) -> &'static [GameButton] {
        let spec = platform.spec();
        match (spec.shoulders, spec.turbo, spec.triggers) {
            (_, false, true) => &GameButton::FACE_TRIGGERS,
            (_, false, false) => &GameButton::FACE,
            (true, true, _) => &GameButton::ALL,
            (false, true, _) => &GameButton::GAME_BOY,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GameButton::Up => "Up",
            GameButton::Down => "Down",
            GameButton::Left => "Left",
            GameButton::Right => "Right",
            GameButton::A => "A Button",
            GameButton::B => "B Button",
            GameButton::Select => "Select",
            GameButton::Start => "Start",
            GameButton::L => "L Button",
            GameButton::R => "R Button",
            GameButton::TurboA => "Turbo A",
            GameButton::TurboB => "Turbo B",
            GameButton::X => "X Button",
            GameButton::Y => "Y Button",
            GameButton::L2 => "L2 Button",
            GameButton::R2 => "R2 Button",
        }
    }

    fn key(self) -> &'static str {
        match self {
            GameButton::Up => "up",
            GameButton::Down => "down",
            GameButton::Left => "left",
            GameButton::Right => "right",
            GameButton::A => "a",
            GameButton::B => "b",
            GameButton::Select => "select",
            GameButton::Start => "start",
            GameButton::L => "l",
            GameButton::R => "r",
            GameButton::TurboA => "turbo_a",
            GameButton::TurboB => "turbo_b",
            GameButton::X => "x",
            GameButton::Y => "y",
            GameButton::L2 => "l2",
            GameButton::R2 => "r2",
        }
    }

    fn default_device(self) -> Device {
        match self {
            GameButton::Up => Device::Up,
            GameButton::Down => Device::Down,
            GameButton::Left => Device::Left,
            GameButton::Right => Device::Right,
            GameButton::A => Device::A,
            GameButton::B => Device::B,
            GameButton::Select => Device::Select,
            GameButton::Start => Device::Start,
            GameButton::L => Device::L1,
            GameButton::R => Device::R1,
            GameButton::TurboA => Device::X,
            GameButton::TurboB => Device::Y,
            GameButton::X => Device::X,
            GameButton::Y => Device::Y,
            GameButton::L2 => Device::L2,
            GameButton::R2 => Device::R2,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Shortcut {
    SaveState,
    LoadState,
    BrightnessUp,
    BrightnessDown,
    BlueLightDown,
    BlueLightUp,
    ColorCorrection,
    PaletteBack,
    PaletteNext,
    ShaderNext,
    ShaderBack,
    FastForward,
    Rewind,
}

impl Shortcut {
    pub const ALL: [Shortcut; 13] = [
        Shortcut::SaveState,
        Shortcut::LoadState,
        Shortcut::BrightnessUp,
        Shortcut::BrightnessDown,
        Shortcut::BlueLightDown,
        Shortcut::BlueLightUp,
        Shortcut::ColorCorrection,
        Shortcut::PaletteBack,
        Shortcut::PaletteNext,
        Shortcut::ShaderNext,
        Shortcut::ShaderBack,
        Shortcut::FastForward,
        Shortcut::Rewind,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Shortcut::SaveState => "Save State",
            Shortcut::LoadState => "Load State",
            Shortcut::BrightnessUp => "Brightness Up",
            Shortcut::BrightnessDown => "Brightness Down",
            Shortcut::BlueLightDown => "Blue Light Down",
            Shortcut::BlueLightUp => "Blue Light Up",
            Shortcut::ColorCorrection => "Color Correction",
            Shortcut::PaletteBack => "Palette Back",
            Shortcut::PaletteNext => "Palette Next",
            Shortcut::ShaderNext => "Shader Next",
            Shortcut::ShaderBack => "Shader Back",
            Shortcut::FastForward => "Fast Forward",
            Shortcut::Rewind => "Rewind",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Shortcut::SaveState => "save_state",
            Shortcut::LoadState => "load_state",
            Shortcut::BrightnessUp => "brightness_up",
            Shortcut::BrightnessDown => "brightness_down",
            Shortcut::BlueLightDown => "blue_light_down",
            Shortcut::BlueLightUp => "blue_light_up",
            Shortcut::ColorCorrection => "color_correction",
            Shortcut::PaletteBack => "palette_back",
            Shortcut::PaletteNext => "palette_next",
            Shortcut::ShaderNext => "shader_next",
            Shortcut::ShaderBack => "shader_back",
            Shortcut::FastForward => "fast_forward",
            Shortcut::Rewind => "rewind",
        }
    }

    fn default_binding(self) -> Option<Binding> {
        let select = |b| Binding::with(Modifier::Select, b);
        match self {
            Shortcut::SaveState => select(Device::R1),
            Shortcut::LoadState => select(Device::L1),
            Shortcut::BrightnessUp => select(Device::Up),
            Shortcut::BrightnessDown => select(Device::Down),
            Shortcut::BlueLightDown => select(Device::Left),
            Shortcut::BlueLightUp => select(Device::Right),
            Shortcut::ColorCorrection => select(Device::Y),
            Shortcut::PaletteBack => select(Device::L2),
            Shortcut::PaletteNext => select(Device::R2),
            Shortcut::ShaderNext => select(Device::A),
            Shortcut::ShaderBack => select(Device::B),
            Shortcut::FastForward => Binding::plain(Device::R2),
            Shortcut::Rewind => Binding::plain(Device::L2),
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Controls {
    game: [Option<Binding>; 16],
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            game: GameButton::EVERY.map(|g| Binding::plain(g.default_device())),
        }
    }
}

impl Controls {
    pub fn game(&self, button: GameButton) -> Option<Binding> {
        self.game[button.index()]
    }

    pub fn set_game(&mut self, button: GameButton, binding: Option<Binding>) {
        self.game[button.index()] = binding;
    }

    /// Upstream's twelve always, an extra button only once it is moved, so the file reads as
    /// upstream's until then.
    fn text(&self) -> String {
        let moved = |g: &&GameButton| self.game(**g) != Binding::plain(g.default_device());
        GameButton::ALL
            .iter()
            .chain(GameButton::EXTRA.iter().filter(moved))
            .map(|g| format!("{}={}\n", g.key(), Binding::key(self.game(*g))))
            .collect()
    }

    fn parse(text: &str) -> Controls {
        let mut c = Controls::default();
        for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
            let game = GameButton::EVERY
                .into_iter()
                .find(|g| g.key().eq_ignore_ascii_case(key.trim()));
            if let (Some(g), Some(binding)) = (game, Binding::parse(value)) {
                c.set_game(g, binding);
            }
        }
        c
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ShortcutKeys {
    keys: [Option<Binding>; 13],
}

impl Default for ShortcutKeys {
    fn default() -> Self {
        ShortcutKeys {
            keys: Shortcut::ALL.map(Shortcut::default_binding),
        }
    }
}

impl ShortcutKeys {
    pub fn get(&self, shortcut: Shortcut) -> Option<Binding> {
        self.keys[shortcut.index()]
    }

    pub fn set(&mut self, shortcut: Shortcut, binding: Option<Binding>) {
        self.keys[shortcut.index()] = binding;
    }
}

pub fn read_shortcuts(root: &Path) -> ShortcutKeys {
    let mut keys = ShortcutKeys::default();
    let Ok(text) = std::fs::read_to_string(root.join("Config").join(SHORTCUTS_FILE)) else {
        return keys;
    };
    for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
        let shortcut = Shortcut::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(key.trim()));
        if let (Some(shortcut), Some(binding)) = (shortcut, Binding::parse(value)) {
            keys.set(shortcut, binding);
        }
    }
    keys
}

pub fn write_shortcuts(root: &Path, keys: &ShortcutKeys) -> std::io::Result<()> {
    let text: String = Shortcut::ALL
        .iter()
        .map(|s| format!("{}={}\n", s.key(), Binding::key(keys.get(*s))))
        .collect();
    write(&root.join("Config").join(SHORTCUTS_FILE), &text)
}

fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(path, text.as_bytes())
}

pub fn read_controls(root: &Path) -> Controls {
    std::fs::read_to_string(root.join("Config").join(CONTROLS_FILE))
        .map_or_else(|_| Controls::default(), |text| Controls::parse(&text))
}

pub fn write_controls(root: &Path, c: &Controls) -> std::io::Result<()> {
    write(&root.join("Config").join(CONTROLS_FILE), &c.text())
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CarouselAction {
    PreviousShelf,
    NextShelf,
    Favorite,
    // --- extras ---
    Search,
    AddToList,
}

impl CarouselAction {
    pub const ALL: [CarouselAction; 5] = [
        CarouselAction::PreviousShelf,
        CarouselAction::NextShelf,
        CarouselAction::Favorite,
        CarouselAction::Search,
        CarouselAction::AddToList,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CarouselAction::PreviousShelf => "Previous Shelf",
            CarouselAction::NextShelf => "Next Shelf",
            CarouselAction::Favorite => "Favorite",
            CarouselAction::Search => "Search",
            CarouselAction::AddToList => "Add to List",
        }
    }

    fn key(self) -> &'static str {
        match self {
            CarouselAction::PreviousShelf => "previous_shelf",
            CarouselAction::NextShelf => "next_shelf",
            CarouselAction::Favorite => "favorite",
            CarouselAction::Search => "search",
            CarouselAction::AddToList => "add_to_list",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct CarouselKeys {
    keys: [Option<Binding>; 5],
}

impl Default for CarouselKeys {
    fn default() -> Self {
        CarouselKeys {
            keys: [
                Binding::plain(Device::L1),
                Binding::plain(Device::R1),
                Binding::plain(Device::Y),
                Binding::plain(Device::X),
                Binding::with(Modifier::Menu, Device::Y),
            ],
        }
    }
}

impl CarouselKeys {
    pub fn get(&self, action: CarouselAction) -> Option<Binding> {
        self.keys[action.index()]
    }

    pub fn set(&mut self, action: CarouselAction, binding: Option<Binding>) {
        self.keys[action.index()] = binding;
    }
}

pub fn read_carousel(root: &Path) -> CarouselKeys {
    let mut keys = CarouselKeys::default();
    let Ok(text) = std::fs::read_to_string(root.join("Config").join(CAROUSEL_FILE)) else {
        return keys;
    };
    for (key, value) in text.lines().filter_map(|l| l.split_once('=')) {
        let action = CarouselAction::ALL
            .into_iter()
            .find(|a| a.key().eq_ignore_ascii_case(key.trim()));
        if let (Some(action), Some(binding)) = (action, Binding::parse(value)) {
            keys.set(action, binding);
        }
    }
    keys
}

pub fn write_carousel(root: &Path, keys: &CarouselKeys) -> std::io::Result<()> {
    let text: String = CarouselAction::ALL
        .iter()
        .map(|a| format!("{}={}\n", a.key(), Binding::key(keys.get(*a))))
        .collect();
    write(&root.join("Config").join(CAROUSEL_FILE), &text)
}
