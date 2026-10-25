pub mod app;
pub mod audio;
pub mod baseos;
pub mod bezel;
pub mod bluetooth;
pub mod boot_time;
pub mod bootlogo;
pub mod build_info;
pub mod cable;
pub mod card_log;
#[cfg(unix)]
pub mod card_space;
pub mod cart_faces;
pub mod cheats;
pub mod cheevos;
pub mod core;
pub mod core_picker;
#[cfg(unix)]
pub mod crash;
#[cfg(not(unix))]
pub mod crash {
    pub fn install() {}
}
pub mod discs;
pub mod drc;
pub mod emu;
pub mod face_builder;
pub mod frames;
pub mod frontend;
pub mod gambatte;
pub mod hdmi;
pub mod input;
pub mod label_cache;
pub mod latency;
pub mod link_art_builder;
pub mod link_kind;
pub mod link_net;
pub mod link_radio;
pub mod link_screen;
pub mod link_start;
pub mod link_state;
pub mod loading_mark;
pub mod more_cores;
pub mod ota;
pub mod panel_turn;
pub mod persist;
pub mod pitch;
pub mod prefetch;
pub mod resample;
pub mod rewind;
pub mod root;
pub mod scrape;
pub mod session;
pub mod sha256;
pub mod shot_loader;
pub mod shot_writer;
pub mod ssh;
pub mod state_writer;
pub mod text_faces;
pub mod thumb;
pub mod timesync;
pub mod video_mode;
pub mod wallpaper;
pub mod web;
pub mod wifi;
