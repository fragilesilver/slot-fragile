// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/ffi.rs.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint, c_void};

pub enum rc_client_t {}
pub enum rc_api_request_t {}
pub enum rc_client_event_t {}
pub enum rc_client_async_handle_t {}

pub type ReadMemory = unsafe extern "C" fn(u32, *mut u8, u32, *mut rc_client_t) -> u32;
pub type ServerCallback = unsafe extern "C" fn(*const c_void, *mut c_void);
pub type ServerCall =
    unsafe extern "C" fn(*const rc_api_request_t, ServerCallback, *mut c_void, *mut rc_client_t);
pub type Callback = unsafe extern "C" fn(c_int, *const c_char, *mut rc_client_t, *mut c_void);
pub type EventHandler = unsafe extern "C" fn(*const rc_client_event_t, *mut rc_client_t);
pub type LogHandler = unsafe extern "C" fn(*const c_char, *const rc_client_t);
pub type AchievementCallback =
    unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char, u32, c_int);

pub const RC_OK: c_int = 0;
pub const RC_NO_GAME_LOADED: c_int = -29;
pub const RC_ABORTED: c_int = -31;
pub const RC_NO_RESPONSE: c_int = -32;
pub const RC_INVALID_CREDENTIALS: c_int = -34;
pub const RC_EXPIRED_TOKEN: c_int = -35;

pub const RETRYABLE_CLIENT_ERROR: c_int = -2;

pub const LOG_WARN: c_int = 2;

#[repr(C)]
pub struct RetroMemoryDescriptor {
    pub flags: u64,
    pub ptr: *mut c_void,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
    pub addrspace: *const c_char,
}

#[repr(C)]
pub struct SlotEvent {
    pub kind: u32,
    pub points: u32,
    pub title: *const c_char,
    pub description: *const c_char,
    pub badge_url: *const c_char,
    pub message: *const c_char,
}

#[repr(C)]
pub struct SlotGame {
    pub title: *const c_char,
    pub badge_url: *const c_char,
    pub console: u32,
    pub unlocked: u32,
    pub total: u32,
}

extern "C" {
    pub fn rc_client_create(read: ReadMemory, server: ServerCall) -> *mut rc_client_t;
    pub fn rc_client_set_userdata(client: *mut rc_client_t, userdata: *mut c_void);
    pub fn rc_client_get_userdata(client: *const rc_client_t) -> *mut c_void;
    pub fn rc_client_set_event_handler(client: *mut rc_client_t, handler: EventHandler);
    pub fn rc_client_set_hardcore_enabled(client: *mut rc_client_t, enabled: c_int);
    pub fn rc_client_set_allow_background_memory_reads(client: *mut rc_client_t, allowed: c_int);
    pub fn rc_client_enable_logging(client: *mut rc_client_t, level: c_int, cb: LogHandler);
    pub fn rc_client_set_host(client: *mut rc_client_t, host: *const c_char);
    pub fn rc_client_get_user_agent_clause(
        client: *mut rc_client_t,
        buffer: *mut c_char,
        size: usize,
    ) -> usize;
    pub fn rc_client_begin_login_with_password(
        client: *mut rc_client_t,
        username: *const c_char,
        password: *const c_char,
        callback: Callback,
        userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;
    pub fn rc_client_begin_login_with_token(
        client: *mut rc_client_t,
        username: *const c_char,
        token: *const c_char,
        callback: Callback,
        userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;
    pub fn rc_client_begin_load_game(
        client: *mut rc_client_t,
        hash: *const c_char,
        callback: Callback,
        userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;
    pub fn rc_client_unload_game(client: *mut rc_client_t);
    pub fn rc_client_do_frame(client: *mut rc_client_t);
    pub fn rc_client_idle(client: *mut rc_client_t);
    pub fn rc_client_reset(client: *mut rc_client_t);

    pub fn rc_hash_generate_from_buffer(
        hash: *mut c_char,
        console_id: u32,
        buffer: *const u8,
        size: usize,
    ) -> c_int;

    pub fn rc_hash_generate_from_file(
        hash: *mut c_char,
        console_id: u32,
        path: *const c_char,
    ) -> c_int;

    pub fn slot_regions_new(
        descriptors: *const RetroMemoryDescriptor,
        count: c_uint,
        console_id: u32,
    ) -> *mut c_void;
    pub fn slot_regions_free(regions: *mut c_void);
    pub fn slot_regions_read(regions: *const c_void, address: u32, buffer: *mut u8, n: u32) -> u32;
    pub fn slot_request_fields(
        request: *const rc_api_request_t,
        url: *mut *const c_char,
        post: *mut *const c_char,
        content_type: *mut *const c_char,
    );
    pub fn slot_respond(
        callback: ServerCallback,
        data: *mut c_void,
        body: *const c_char,
        len: usize,
        status: c_int,
    );
    pub fn slot_event_fields(event: *const rc_client_event_t, out: *mut SlotEvent);
    pub fn slot_game_fields(client: *mut rc_client_t, out: *mut SlotGame) -> c_int;
    pub fn slot_user_token(client: *const rc_client_t) -> *const c_char;
    pub fn slot_user_name(client: *const rc_client_t) -> *const c_char;
    // --- Fragile ---
    pub fn rc_client_logout(client: *mut rc_client_t);
    pub fn slot_achievements(
        client: *mut rc_client_t,
        cb: AchievementCallback,
        data: *mut c_void,
    ) -> c_int;
}

pub unsafe fn text(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
}
