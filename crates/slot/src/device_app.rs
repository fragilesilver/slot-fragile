use std::path::PathBuf;
use std::time::{Duration, Instant};

use slot::frontend::Frontend;
use slot::hdmi::{DeviceHdmi, HdmiController, HDMI_ALSA_DEVICE};
use slot::input::DeviceInput;
use slot::panel_turn::{panel_turn, PanelTurn};
use slot_gfx::{Compositor, FbdevSurface, Surface};
use slot_power::{trace_first_frame, DevicePlatform};

struct RescueInput<I> {
    inner: I,
    menu_held: bool,
    vol_up_held: bool,
    held_since: Option<Instant>,
    rescue_triggered: bool,
}

impl<I: slot_input::InputSource> RescueInput<I> {
    fn new(inner: I) -> Self {
        RescueInput {
            inner,
            menu_held: false,
            vol_up_held: false,
            held_since: None,
            rescue_triggered: false,
        }
    }

    fn check_rescue(&mut self) -> bool {
        if self.rescue_triggered {
            return false;
        }
        if self.menu_held && self.vol_up_held {
            let start = self.held_since.get_or_insert_with(Instant::now);
            if start.elapsed() >= Duration::from_millis(1500) {
                self.rescue_triggered = true;
                return true;
            }
        } else {
            self.held_since = None;
        }
        false
    }
}

impl RescueInput<DeviceInput> {
    fn steer_with_stick(&self, on: bool) {
        self.inner.steer_with_stick(on);
    }
}

impl<I: slot_input::InputSource> slot_input::InputSource for RescueInput<I> {
    fn poll(&mut self, now: slot_input::Millis) -> Vec<slot_input::RawEvent> {
        let events = self.inner.poll(now);
        for ev in &events {
            match ev {
                slot_input::RawEvent::Down(slot_input::Btn::Menu) => self.menu_held = true,
                slot_input::RawEvent::Up(slot_input::Btn::Menu) => self.menu_held = false,
                slot_input::RawEvent::Down(slot_input::Btn::VolUp) => self.vol_up_held = true,
                slot_input::RawEvent::Up(slot_input::Btn::VolUp) => self.vol_up_held = false,
                _ => {}
            }
        }
        events
    }
}

fn re_exec() -> ! {
    eprintln!("slot: re-executing frontend process for display change...");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/bin/slot"));
        let mut cmd = std::process::Command::new(exe);
        cmd.args(std::env::args_os().skip(1));
        let err = cmd.exec();
        eprintln!("slot: re-exec failed: {err}");
    }
    std::process::exit(0);
}

const CARD: &str = "/mnt/sdcard";

const MIN_FRAME: Duration = Duration::from_millis(12);

const STEP_TIMEOUT: Duration = Duration::from_millis(12);

const MARGIN: Duration = Duration::from_micros(1500);

const LOG_SYNC: Duration = Duration::from_secs(1);

struct Pacer {
    last_return: Option<Instant>,
    period: Duration,
    work: Duration,
    works: [Duration; WORK_WINDOW],
    next: usize,
}

const WORK_WINDOW: usize = 32;

impl Pacer {
    fn new() -> Self {
        Pacer {
            last_return: None,
            period: Duration::from_micros(16_760),
            work: Duration::from_millis(4),
            works: [Duration::from_millis(4); WORK_WINDOW],
            next: 0,
        }
    }

    fn wait(&self) {
        let Some(last) = self.last_return else {
            return;
        };
        let delay = self.period.saturating_sub(self.work + MARGIN);
        if let Some(left) = (last + delay).checked_duration_since(Instant::now()) {
            std::thread::sleep(left);
        }
    }

    fn swapped(&mut self, work: Duration, blocked: bool) -> bool {
        let now = Instant::now();
        let mut missed = false;
        if let Some(last) = self.last_return {
            let seen = now - last;
            missed = seen > self.period.mul_f64(1.5);
            if blocked && seen > Duration::from_millis(12) && seen < Duration::from_millis(22) {
                self.period = self.period.mul_f64(0.95) + seen.mul_f64(0.05);
            }
        }
        self.works[self.next] = work;
        self.next = (self.next + 1) % WORK_WINDOW;
        self.work = self.works.iter().copied().max().unwrap_or(work);
        self.last_return = Some(now);
        missed
    }
}

pub fn run() {
    slot::boot_time::mark("start");
    let _ = std::thread::Builder::new()
        .name("slot-fonts".into())
        .spawn(slot_ui::warm_fonts);
    let audio = std::thread::Builder::new()
        .name("slot-audio-open".into())
        .spawn(slot::audio::opened_sink);
    let root = std::env::var_os("SLOT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
    let mut hdmi = DeviceHdmi::new(&root);
    let hdmi_mode = slot::hdmi::read_hdmi_mode(Some(&root));
    let hdmi_connected = hdmi.is_connected();
    let mut hdmi_active = false;
    if hdmi_mode.is_enabled() && hdmi_connected {
        eprintln!("slot: hdmi connected at boot, activating {hdmi_mode:?}");
        if let Ok(()) = hdmi.switch_on(hdmi_mode) {
            hdmi_active = true;
            slot::audio::set_route(Some(HDMI_ALSA_DEVICE.to_string()));
        }
    }
    let mut surface = match FbdevSurface::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("surface");
    let mut compositor = match Compositor::new(&surface) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("compositor");
    let panel = surface.window_size();
    let env = |key| std::env::var(key).ok();
    let turn = if hdmi_active {
        PanelTurn::None
    } else {
        panel_turn(
            panel,
            slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").as_deref(),
            env("SLOT_ROTATE").as_deref(),
        )
    };
    let turned = turn != PanelTurn::None;
    let window = if turned { (panel.1, panel.0) } else { panel };
    eprintln!(
        "slot: baseos target {}, model {}, rotation ccw {}",
        slot::baseos::var("BASEOS_TARGET").unwrap_or_default(),
        slot::baseos::var("BASEOS_MODEL").unwrap_or_default(),
        slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").unwrap_or_default()
    );
    eprintln!(
        "slot: panel {}x{}, canvas {}x{}, turn {turn:?}",
        panel.0, panel.1, window.0, window.1
    );
    if turned {
        if let Err(e) = compositor.set_panel_turn(panel, turn == PanelTurn::Ccw) {
            eprintln!("slot: {e}");
            return;
        }
    }
    let platform = DevicePlatform::new(root.clone());
    eprintln!("slot: {}", platform.report());
    platform.trace_boot();
    slot::boot_time::mark("platform");
    let sink = match audio.map(|h| h.join()) {
        Ok(Ok(sink)) => sink,
        _ => slot::audio::opened_sink(),
    };
    slot::boot_time::mark("audio");
    let mut frontend = Frontend::boot_with(Box::new(platform), sink);
    slot::boot_time::mark("frontend");
    let target = slot::baseos::var("BASEOS_TARGET").unwrap_or_default();
    frontend.set_left_stick(slot::input::evdev::has_left_stick(&target));
    frontend.set_lid(slot::input::evdev::has_lid(&target));
    frontend.upload_boot_faces(&mut compositor);
    slot::boot_time::mark("faces");
    frontend.upload_bezel(&mut compositor, window);
    slot::boot_time::mark("bezel");
    let mut input = RescueInput::new(DeviceInput::open(&root));
    slot::boot_time::mark("input");
    let card = root.clone();
    let _ = std::thread::Builder::new()
        .name("slot-bootlogo".into())
        .spawn(move || slot::bootlogo::refresh(&card));
    let _ = std::thread::Builder::new()
        .name("slot-logsync".into())
        .spawn(sync_log_writes);
    let mut drawn = false;
    frontend.drive_emulator();
    let mut pacer = Pacer::new();
    let (mut frames, mut missed, mut early) = (0u32, 0u32, 0u32);
    let mut last_hdmi_check = Instant::now();
    let mut last_hdmi_mode = frontend.app().hdmi_mode();
    loop {
        pacer.wait();
        let began = Instant::now();
        input.steer_with_stick(frontend.app().left_stick_dpad());
        frontend.advance(&mut input);

        if input.check_rescue() {
            eprintln!("slot: hdmi rescue triggered via hardware shortcut (Menu + VolUp)");
            let _ = hdmi.rescue();
            slot::audio::set_route(None);
            frontend.app_mut().settle_saves();
            re_exec();
        }

        if last_hdmi_check.elapsed() >= Duration::from_millis(500) {
            last_hdmi_check = Instant::now();
            let connected_now = hdmi.is_connected();
            let current_mode = frontend.app().hdmi_mode();
            let mode_changed = current_mode != last_hdmi_mode;
            last_hdmi_mode = current_mode;

            if connected_now != hdmi_connected || (connected_now && mode_changed) {
                eprintln!(
                    "slot: hdmi transition (connected: {hdmi_connected} -> {connected_now}, mode: {current_mode:?})"
                );
                if connected_now && current_mode.is_enabled() {
                    let _ = hdmi.switch_on(current_mode);
                    slot::audio::set_route(Some(HDMI_ALSA_DEVICE.to_string()));
                } else {
                    let _ = hdmi.switch_off();
                    slot::audio::set_route(None);
                }
                frontend.app_mut().settle_saves();
                re_exec();
            }
        }
        if std::path::Path::new("/tmp/slot_restart").exists() {
            let _ = std::fs::remove_file("/tmp/slot_restart");
            eprintln!("slot: restart signaled via web dashboard");
            frontend.app_mut().settle_saves();
            re_exec();
        }
        if frontend.restarting() {
            frontend.restart();
        }
        if frontend.repairing() {
            frontend.repair();
        }
        if frontend.powering_off() {
            frontend.poweroff();
            return;
        }
        frontend.step_emulator(pacer.period, STEP_TIMEOUT);
        frontend.render(&mut compositor, window);
        let swap = Instant::now();
        let work = swap - began;
        if let Err(e) = surface.swap() {
            eprintln!("slot: {e}");
            return;
        }
        let swap_took = swap.elapsed();
        slot::latency::swapped(swap_took.as_secs_f64() * 1000.0);
        let dropped = pacer.swapped(work, swap_took > Duration::from_millis(1));
        if slot::latency::tracing() {
            frames += 1;
            missed += u32::from(dropped);
            early += u32::from(swap_took > pacer.period / 2);
            if frames == 600 {
                eprintln!(
                    "slot: pace: 600 frames, {missed} missed a latch, {early} drawn early, work {:.1} ms, period {:.2} ms",
                    pacer.work.as_secs_f64() * 1e3,
                    pacer.period.as_secs_f64() * 1e3
                );
                (frames, missed, early) = (0, 0, 0);
            }
        }
        if !drawn {
            drawn = true;
            slot::boot_time::mark("first frame");
            trace_first_frame();
        }
        if let Some(left) = MIN_FRAME.checked_sub(began.elapsed()) {
            std::thread::sleep(left);
        }
    }
}

fn sync_log_writes() {
    let mut synced = 0;
    loop {
        std::thread::sleep(LOG_SYNC);
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(2, &mut st) } != 0 {
            return;
        }
        if st.st_size != synced {
            #[cfg(target_os = "linux")]
            unsafe {
                libc::syncfs(2);
            }
            synced = st.st_size;
        }
    }
}
