// SPDX-License-Identifier: GPL-3.0-or-later
// DSperate - Nintendo DS emulator. Copyright (C) 2026 DSperate contributors.
//
// Libretro core frontend for DSperate.
#include "libretro.h"
#include "core/nds.h"
#include "core/cart/cart.h"
#include "core/state/state.h"
#if DSPERATE_JIT
#include "core/cpu/jit/jit.h"
#endif

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

namespace {

retro_environment_t environ_cb = nullptr;
retro_video_refresh_t video_cb = nullptr;
retro_audio_sample_t audio_cb = nullptr;
retro_audio_sample_batch_t audio_batch_cb = nullptr;
retro_input_poll_t input_poll_cb = nullptr;
retro_input_state_t input_state_cb = nullptr;

std::unique_ptr<ds::NDS> g_nds;
#if DSPERATE_JIT
bool g_jit_attached = false;
#endif
bool g_use_jit = true;

// Layouts
enum ScreenLayout {
  LAYOUT_DOMINANT_V = 0,
  LAYOUT_TOP_ONLY,
  LAYOUT_BOTTOM_ONLY,
  LAYOUT_VERTICAL,
  LAYOUT_HORIZONTAL,
  LAYOUT_PIP,
};

int g_layout = LAYOUT_DOMINANT_V;
int g_screen_gap = 0;
bool g_swap_screens = false;
int g_swap_button = RETRO_DEVICE_ID_JOYPAD_L2;
int g_touch_button = RETRO_DEVICE_ID_JOYPAD_R2;
int g_mic_button = RETRO_DEVICE_ID_JOYPAD_L3;
int g_stylus_control = 1; // 0 = off, 1 = right analog, 2 = left analog
int g_stylus_mode = 0;    // 0 = relative (velocity), 1 = absolute
float g_stylus_speed = 1.5f;
bool g_show_cursor = true;

// Virtual stylus coordinates on bottom screen (256x192)
float g_stylus_x = 128.0f;
float g_stylus_y = 96.0f;
bool g_prev_swap_btn = false;

// Audio buffer: SPU mixes at ~32728.5 Hz. DS frame is ~546 stereo frames.
std::array<int16_t, 4096 * 2> g_audio_buf{};

// Noise buffer for fake microphone
std::array<int16_t, 1024> g_mic_noise{};

// Framebuffer for libretro composition: max size 512 x 512
std::vector<uint32_t> g_composite_buf(512 * 512, 0xFF000000);

void init_mic_noise() {
  for (size_t i = 0; i < g_mic_noise.size(); ++i) {
    g_mic_noise[i] = static_cast<int16_t>((std::rand() % 16384) - 8192);
  }
}

void update_system_geometry() {
  if (!environ_cb) return;

  struct retro_system_av_info av_info {};
  unsigned w = 256;
  unsigned h = 192;
  float aspect = 4.0f / 3.0f;

  switch (g_layout) {
    case LAYOUT_DOMINANT_V:
      w = 384;
      h = 480;
      aspect = 384.0f / 480.0f;
      break;
    case LAYOUT_TOP_ONLY:
    case LAYOUT_BOTTOM_ONLY:
    case LAYOUT_PIP:
      w = 256;
      h = 192;
      aspect = 4.0f / 3.0f;
      break;
    case LAYOUT_VERTICAL:
      w = 256;
      h = 384 + g_screen_gap;
      aspect = 256.0f / static_cast<float>(h);
      break;
    case LAYOUT_HORIZONTAL:
      w = 512;
      h = 192;
      aspect = 512.0f / 192.0f;
      break;
  }

  av_info.geometry.base_width = w;
  av_info.geometry.base_height = h;
  av_info.geometry.max_width = 512;
  av_info.geometry.max_height = 512;
  av_info.geometry.aspect_ratio = aspect;
  av_info.timing.fps = 59.8261;
  av_info.timing.sample_rate = 32728.5;

  environ_cb(RETRO_ENVIRONMENT_SET_GEOMETRY, &av_info.geometry);
}

void check_variables(bool first_time = false) {
  if (!environ_cb) return;

  struct retro_variable var {};

  var.key = "dsperate_layout";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    int old_layout = g_layout;
    if (std::strcmp(var.value, "Top Dominant") == 0) g_layout = LAYOUT_DOMINANT_V;
    else if (std::strcmp(var.value, "Top Only") == 0) g_layout = LAYOUT_TOP_ONLY;
    else if (std::strcmp(var.value, "Bottom Only") == 0) g_layout = LAYOUT_BOTTOM_ONLY;
    else if (std::strcmp(var.value, "Vertical") == 0) g_layout = LAYOUT_VERTICAL;
    else if (std::strcmp(var.value, "Horizontal") == 0) g_layout = LAYOUT_HORIZONTAL;
    else if (std::strcmp(var.value, "Picture in Picture") == 0) g_layout = LAYOUT_PIP;
    if (!first_time && g_layout != old_layout) update_system_geometry();
  }

  var.key = "dsperate_screen_gap";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    int old_gap = g_screen_gap;
    g_screen_gap = std::atoi(var.value);
    if (!first_time && g_screen_gap != old_gap && g_layout == LAYOUT_VERTICAL) {
      update_system_geometry();
    }
  }

  var.key = "dsperate_swap_button";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    if (std::strcmp(var.value, "L2") == 0) g_swap_button = RETRO_DEVICE_ID_JOYPAD_L2;
    else if (std::strcmp(var.value, "R2") == 0) g_swap_button = RETRO_DEVICE_ID_JOYPAD_R2;
    else if (std::strcmp(var.value, "L3") == 0) g_swap_button = RETRO_DEVICE_ID_JOYPAD_L3;
    else if (std::strcmp(var.value, "R3") == 0) g_swap_button = RETRO_DEVICE_ID_JOYPAD_R3;
    else g_swap_button = -1;
  }

  var.key = "dsperate_touch_button";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    if (std::strcmp(var.value, "R2") == 0) g_touch_button = RETRO_DEVICE_ID_JOYPAD_R2;
    else if (std::strcmp(var.value, "L2") == 0) g_touch_button = RETRO_DEVICE_ID_JOYPAD_L2;
    else if (std::strcmp(var.value, "R3") == 0) g_touch_button = RETRO_DEVICE_ID_JOYPAD_R3;
    else if (std::strcmp(var.value, "L3") == 0) g_touch_button = RETRO_DEVICE_ID_JOYPAD_L3;
    else if (std::strcmp(var.value, "A") == 0) g_touch_button = RETRO_DEVICE_ID_JOYPAD_A;
    else g_touch_button = RETRO_DEVICE_ID_JOYPAD_R2;
  }

  var.key = "dsperate_mic_button";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    if (std::strcmp(var.value, "L3") == 0) g_mic_button = RETRO_DEVICE_ID_JOYPAD_L3;
    else if (std::strcmp(var.value, "R3") == 0) g_mic_button = RETRO_DEVICE_ID_JOYPAD_R3;
    else if (std::strcmp(var.value, "L2") == 0) g_mic_button = RETRO_DEVICE_ID_JOYPAD_L2;
    else g_mic_button = -1;
  }

  var.key = "dsperate_stylus_control";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    if (std::strcmp(var.value, "Right Analog") == 0) g_stylus_control = 1;
    else if (std::strcmp(var.value, "Left Analog") == 0) g_stylus_control = 2;
    else g_stylus_control = 0;
  }

  var.key = "dsperate_stylus_mode";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    if (std::strcmp(var.value, "Absolute") == 0) g_stylus_mode = 1;
    else g_stylus_mode = 0;
  }

  var.key = "dsperate_stylus_speed";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    g_stylus_speed = static_cast<float>(std::atof(var.value));
    if (g_stylus_speed <= 0.0f) g_stylus_speed = 1.5f;
  }

  var.key = "dsperate_show_cursor";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    g_show_cursor = (std::strcmp(var.value, "On") == 0);
  }

  var.key = "dsperate_cpu_engine";
  if (environ_cb(RETRO_ENVIRONMENT_GET_VARIABLE, &var) && var.value) {
    g_use_jit = (std::strcmp(var.value, "JIT (AArch64)") == 0);
  }
}

void set_core_options() {
  if (!environ_cb) return;

  static const struct retro_variable vars[] = {
    { "dsperate_layout", "Screen Layout; Top Dominant|Top Only|Bottom Only|Vertical|Horizontal|Picture in Picture" },
    { "dsperate_screen_gap", "Vertical Screen Gap; 0|4|8|16|32|64|90" },
    { "dsperate_swap_button", "Screen Swap Button; L2|R2|L3|R3|Off" },
    { "dsperate_touch_button", "Touch / Stylus Click; R2|L2|R3|L3|A" },
    { "dsperate_mic_button", "Microphone Button; L3|R3|L2|Off" },
    { "dsperate_stylus_control", "Stylus Stick; Right Analog|Left Analog|Off" },
    { "dsperate_stylus_mode", "Stylus Mode; Relative (Velocity)|Absolute" },
    { "dsperate_stylus_speed", "Stylus Speed; 1.5|1.0|2.0|2.5|3.0|0.5" },
    { "dsperate_show_cursor", "Show Stylus Cursor; On|Off" },
    { "dsperate_cpu_engine", "CPU Engine; JIT (AArch64)|Interpreter" },
    { nullptr, nullptr }
  };

  environ_cb(RETRO_ENVIRONMENT_SET_VARIABLES, const_cast<struct retro_variable*>(vars));
}

void draw_cursor(uint32_t* dst, int pitch_px, int cx, int cy, int max_w, int max_h) {
  if (cx < 0 || cx >= max_w || cy < 0 || cy >= max_h) return;

  const uint32_t col = 0x00FF3333; // Bright red crosshair
  const uint32_t border = 0x00FFFFFF;

  for (int d = -3; d <= 3; ++d) {
    int px = cx + d;
    int py = cy;
    if (px >= 0 && px < max_w && py >= 0 && py < max_h) {
      dst[py * pitch_px + px] = (d == 0) ? border : col;
    }
    px = cx;
    py = cy + d;
    if (px >= 0 && px < max_w && py >= 0 && py < max_h) {
      dst[py * pitch_px + px] = (d == 0) ? border : col;
    }
  }
}

} // namespace

#ifdef __cplusplus
extern "C" {
#endif

RETRO_API void retro_set_environment(retro_environment_t cb) {
  environ_cb = cb;
  if (environ_cb) {
    set_core_options();
  }
}

RETRO_API void retro_set_video_refresh(retro_video_refresh_t cb) {
  video_cb = cb;
}

RETRO_API void retro_set_audio_sample(retro_audio_sample_t cb) {
  audio_cb = cb;
}

RETRO_API void retro_set_audio_sample_batch(retro_audio_sample_batch_t cb) {
  audio_batch_cb = cb;
}

RETRO_API void retro_set_input_poll(retro_input_poll_t cb) {
  input_poll_cb = cb;
}

RETRO_API void retro_set_input_state(retro_input_state_t cb) {
  input_state_cb = cb;
}

RETRO_API unsigned retro_api_version(void) {
  return RETRO_API_VERSION;
}

RETRO_API void retro_get_system_info(struct retro_system_info *info) {
  info->library_name = "DSperate";
  info->library_version = "v0.2.0";
  info->valid_extensions = "nds|bin|zip";
  info->need_fullpath = false;
  info->block_extract = false;
}

RETRO_API void retro_get_system_av_info(struct retro_system_av_info *info) {
  check_variables(true);

  unsigned w = 256;
  unsigned h = 192;
  float aspect = 4.0f / 3.0f;

  switch (g_layout) {
    case LAYOUT_DOMINANT_V:
      w = 384;
      h = 480;
      aspect = 384.0f / 480.0f;
      break;
    case LAYOUT_TOP_ONLY:
    case LAYOUT_BOTTOM_ONLY:
    case LAYOUT_PIP:
      w = 256;
      h = 192;
      aspect = 4.0f / 3.0f;
      break;
    case LAYOUT_VERTICAL:
      w = 256;
      h = 384 + g_screen_gap;
      aspect = 256.0f / static_cast<float>(h);
      break;
    case LAYOUT_HORIZONTAL:
      w = 512;
      h = 192;
      aspect = 512.0f / 192.0f;
      break;
  }

  info->geometry.base_width = w;
  info->geometry.base_height = h;
  info->geometry.max_width = 512;
  info->geometry.max_height = 512;
  info->geometry.aspect_ratio = aspect;

  info->timing.fps = 59.8261;
  info->timing.sample_rate = 32728.5;
}

RETRO_API void retro_init(void) {
  enum retro_pixel_format fmt = RETRO_PIXEL_FORMAT_XRGB8888;
  if (environ_cb) {
    environ_cb(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, &fmt);
  }
  init_mic_noise();
}

RETRO_API void retro_deinit(void) {
  if (g_nds) {
#if DSPERATE_JIT
    if (g_jit_attached) {
      ds::jit::detach(*g_nds);
      g_jit_attached = false;
    }
#endif
    g_nds.reset();
  }
}

RETRO_API void retro_set_controller_port_device(unsigned port, unsigned device) {
  (void)port;
  (void)device;
}

RETRO_API void retro_reset(void) {
  if (g_nds) {
    g_nds->reset();
    g_nds->setup_direct_boot();
  }
}

RETRO_API bool retro_load_game(const struct retro_game_info *game) {
  if (!game) return false;

  check_variables(true);

  g_nds = std::make_unique<ds::NDS>();

  // Check system directory for BIOS dumps
  const char* sys_dir = nullptr;
  std::string bios9_path, bios7_path, fw_path;
  if (environ_cb && environ_cb(RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY, &sys_dir) && sys_dir) {
    std::string base = sys_dir;
    if (!base.empty() && base.back() != '/') base += '/';

    std::array<std::string, 3> subdirs = { base, base + "nds/", base + "ds/" };
    for (const auto& d : subdirs) {
      if (FILE* f = std::fopen((d + "bios9.bin").c_str(), "rb")) { std::fclose(f); bios9_path = d + "bios9.bin"; }
      if (FILE* f = std::fopen((d + "bios7.bin").c_str(), "rb")) { std::fclose(f); bios7_path = d + "bios7.bin"; }
      if (FILE* f = std::fopen((d + "firmware.bin").c_str(), "rb")) { std::fclose(f); fw_path = d + "firmware.bin"; }
    }
  }

  std::string err;
  g_nds->load_bios(bios9_path, bios7_path, fw_path, {}, &err);

  // Load ROM
  bool loaded = false;
  if (game->path && game->path[0]) {
    loaded = g_nds->load_rom(game->path);
  }
  if (!loaded && game->data && game->size > 0) {
    std::vector<ds::u8> rom_buf(static_cast<const ds::u8*>(game->data),
                                static_cast<const ds::u8*>(game->data) + game->size);
    loaded = g_nds->load_rom_image(std::move(rom_buf));
  }

  if (!loaded) {
    g_nds.reset();
    return false;
  }

  g_nds->setup_direct_boot();

#if DSPERATE_JIT
  if (g_use_jit) {
    g_jit_attached = ds::jit::attach(*g_nds, true, true);
  }
#endif

  return true;
}

RETRO_API bool retro_load_game_special(unsigned game_type, const struct retro_game_info *info, size_t num_info) {
  (void)game_type;
  (void)info;
  (void)num_info;
  return false;
}

RETRO_API void retro_unload_game(void) {
  if (g_nds) {
#if DSPERATE_JIT
    if (g_jit_attached) {
      ds::jit::detach(*g_nds);
      g_jit_attached = false;
    }
#endif
    g_nds.reset();
  }
}

RETRO_API unsigned retro_get_region(void) {
  return RETRO_REGION_NTSC;
}

RETRO_API void* retro_get_memory_data(unsigned id) {
  if (!g_nds) return nullptr;
  switch (id) {
    case RETRO_MEMORY_SAVE_RAM:
      return g_nds->cart ? g_nds->cart->sram().data() : nullptr;
    case RETRO_MEMORY_SYSTEM_RAM:
      return g_nds->bus.main_ram.get();
    default:
      return nullptr;
  }
}

RETRO_API size_t retro_get_memory_size(unsigned id) {
  if (!g_nds) return 0;
  switch (id) {
    case RETRO_MEMORY_SAVE_RAM:
      return g_nds->cart ? g_nds->cart->sram().size() : 0;
    case RETRO_MEMORY_SYSTEM_RAM:
      return g_nds->bus.main_ram_size();
    default:
      return 0;
  }
}

RETRO_API size_t retro_serialize_size(void) {
  return 8 * 1024 * 1024; // 8MB state buffer
}

RETRO_API bool retro_serialize(void *data, size_t size) {
  if (!g_nds || !data) return false;
  ds::state::Writer w;
  std::string err;
  if (!g_nds->save_state(w, err)) return false;
  if (w.data().size() > size) return false;
  std::memcpy(data, w.data().data(), w.data().size());
  return true;
}

RETRO_API bool retro_unserialize(const void *data, size_t size) {
  if (!g_nds || !data || size == 0) return false;
  ds::state::Reader r(static_cast<const ds::u8*>(data), size);
  std::string err;
  return g_nds->load_state(r, err);
}

RETRO_API void retro_cheat_reset(void) {
  if (g_nds) {
    g_nds->cheats.codes.clear();
  }
}

RETRO_API void retro_cheat_set(unsigned index, bool enabled, const char *code) {
  (void)index;
  (void)enabled;
  (void)code;
}

RETRO_API void retro_run(void) {
  if (!g_nds) return;

  check_variables(false);

  if (input_poll_cb) {
    input_poll_cb();
  }

  // --- Map Controls ---
  ds::u32 pressed = 0;
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A))      pressed |= (1 << ds::io::Io::BTN_A);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_B))      pressed |= (1 << ds::io::Io::BTN_B);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT)) pressed |= (1 << ds::io::Io::BTN_SELECT);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START))  pressed |= (1 << ds::io::Io::BTN_START);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT))  pressed |= (1 << ds::io::Io::BTN_RIGHT);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT))   pressed |= (1 << ds::io::Io::BTN_LEFT);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP))     pressed |= (1 << ds::io::Io::BTN_UP);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN))   pressed |= (1 << ds::io::Io::BTN_DOWN);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_R))      pressed |= (1 << ds::io::Io::BTN_R);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_L))      pressed |= (1 << ds::io::Io::BTN_L);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_X))      pressed |= (1 << ds::io::Io::BTN_X);
  if (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_Y))      pressed |= (1 << ds::io::Io::BTN_Y);

  g_nds->io.set_buttons(pressed);

  // --- Screen Swap Hotkey ---
  if (g_swap_button >= 0) {
    bool swap_btn = (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, g_swap_button) != 0);
    if (swap_btn && !g_prev_swap_btn) {
      g_swap_screens = !g_swap_screens;
    }
    g_prev_swap_btn = swap_btn;
  }

  // --- Stylus Analog Control ---
  if (g_stylus_control > 0) {
    int analog_idx = (g_stylus_control == 1) ? RETRO_DEVICE_INDEX_ANALOG_RIGHT : RETRO_DEVICE_INDEX_ANALOG_LEFT;
    int16_t ax = input_state_cb(0, RETRO_DEVICE_ANALOG, analog_idx, RETRO_DEVICE_ID_ANALOG_X);
    int16_t ay = input_state_cb(0, RETRO_DEVICE_ANALOG, analog_idx, RETRO_DEVICE_ID_ANALOG_Y);

    const int deadzone = 4000;
    if (g_stylus_mode == 0) { // Relative
      if (std::abs(ax) > deadzone) {
        float vx = static_cast<float>(ax) / 32768.0f;
        g_stylus_x += vx * g_stylus_speed * 4.0f;
      }
      if (std::abs(ay) > deadzone) {
        float vy = static_cast<float>(ay) / 32768.0f;
        g_stylus_y += vy * g_stylus_speed * 4.0f;
      }
    } else { // Absolute
      if (std::abs(ax) > deadzone || std::abs(ay) > deadzone) {
        g_stylus_x = (static_cast<float>(ax) + 32768.0f) * 255.0f / 65535.0f;
        g_stylus_y = (static_cast<float>(ay) + 32768.0f) * 191.0f / 65535.0f;
      }
    }

    g_stylus_x = std::max(0.0f, std::min(255.0f, g_stylus_x));
    g_stylus_y = std::max(0.0f, std::min(191.0f, g_stylus_y));
  }

  // --- Touch Down / Up ---
  bool touching = false;
  if (g_touch_button >= 0) {
    touching = (input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, g_touch_button) != 0);
  }
  g_nds->io.set_touch(static_cast<int>(g_stylus_x), static_cast<int>(g_stylus_y), touching);

  // --- Microphone Pulse ---
  if (g_mic_button >= 0 && input_state_cb(0, RETRO_DEVICE_JOYPAD, 0, g_mic_button)) {
    g_nds->io.set_mic(g_mic_noise.data(), g_mic_noise.size());
  } else {
    g_nds->io.set_mic(nullptr, 0);
  }

  // --- Run DS Frame ---
  g_nds->run_frame();

  // --- Audio Output ---
  if (audio_batch_cb) {
    size_t frames = g_nds->spu.take(g_audio_buf.data(), 4096);
    if (frames > 0) {
      audio_batch_cb(g_audio_buf.data(), frames);
    }
  }

  // --- Video Composition ---
  if (video_cb) {
    const uint32_t* src_top = g_nds->gpu.framebuffer(g_swap_screens ? 1 : 0);
    const uint32_t* src_bot = g_nds->gpu.framebuffer(g_swap_screens ? 0 : 1);

    unsigned out_w = 256;
    unsigned out_h = 192;

    switch (g_layout) {
      case LAYOUT_DOMINANT_V: {
        out_w = 384;
        out_h = 480;

        // Clear composite frame
        std::memset(g_composite_buf.data(), 0, 384 * 480 * sizeof(uint32_t));

        // Top Screen: 1.5x scale (256x192 -> 384x288)
        for (int y = 0; y < 288; ++y) {
          const uint32_t* src_row = src_top + ((y * 2) / 3) * 256;
          uint32_t* dst_row = g_composite_buf.data() + y * 384;
          for (int x = 0; x < 384; ++x) {
            dst_row[x] = src_row[(x * 2) / 3];
          }
        }

        // Bottom Screen: 1.0x scale (256x192), centered horizontally (x = (384-256)/2 = 64)
        for (int y = 0; y < 192; ++y) {
          const uint32_t* src_row = src_bot + y * 256;
          uint32_t* dst_row = g_composite_buf.data() + (288 + y) * 384 + 64;
          std::memcpy(dst_row, src_row, 256 * sizeof(uint32_t));
        }

        if (g_show_cursor) {
          int cur_x = 0;
          int cur_y = 0;
          if (g_swap_screens) {
            cur_x = static_cast<int>(g_stylus_x * 1.5f);
            cur_y = static_cast<int>(g_stylus_y * 1.5f);
          } else {
            cur_x = 64 + static_cast<int>(g_stylus_x);
            cur_y = 288 + static_cast<int>(g_stylus_y);
          }
          draw_cursor(g_composite_buf.data(), 384, cur_x, cur_y, 384, 480);
        }
        break;
      }

      case LAYOUT_TOP_ONLY: {
        std::memcpy(g_composite_buf.data(), src_top, 256 * 192 * sizeof(uint32_t));
        out_w = 256;
        out_h = 192;
        break;
      }

      case LAYOUT_BOTTOM_ONLY: {
        std::memcpy(g_composite_buf.data(), src_bot, 256 * 192 * sizeof(uint32_t));
        if (g_show_cursor) {
          draw_cursor(g_composite_buf.data(), 256, static_cast<int>(g_stylus_x), static_cast<int>(g_stylus_y), 256, 192);
        }
        out_w = 256;
        out_h = 192;
        break;
      }

      case LAYOUT_VERTICAL: {
        out_w = 256;
        out_h = 384 + g_screen_gap;

        // Top Screen
        std::memcpy(g_composite_buf.data(), src_top, 256 * 192 * sizeof(uint32_t));

        // Gap
        if (g_screen_gap > 0) {
          std::memset(g_composite_buf.data() + 256 * 192, 0, 256 * g_screen_gap * sizeof(uint32_t));
        }

        // Bottom Screen
        uint32_t* bot_dst = g_composite_buf.data() + 256 * (192 + g_screen_gap);
        std::memcpy(bot_dst, src_bot, 256 * 192 * sizeof(uint32_t));

        if (g_show_cursor) {
          int cur_y = (g_swap_screens ? 0 : (192 + g_screen_gap)) + static_cast<int>(g_stylus_y);
          draw_cursor(g_composite_buf.data(), 256, static_cast<int>(g_stylus_x), cur_y, 256, out_h);
        }
        break;
      }

      case LAYOUT_HORIZONTAL: {
        out_w = 512;
        out_h = 192;

        for (int y = 0; y < 192; ++y) {
          uint32_t* row = g_composite_buf.data() + y * 512;
          std::memcpy(row, src_top + y * 256, 256 * sizeof(uint32_t));
          std::memcpy(row + 256, src_bot + y * 256, 256 * sizeof(uint32_t));
        }

        if (g_show_cursor) {
          int cur_x = (g_swap_screens ? 0 : 256) + static_cast<int>(g_stylus_x);
          draw_cursor(g_composite_buf.data(), 512, cur_x, static_cast<int>(g_stylus_y), 512, 192);
        }
        break;
      }

      case LAYOUT_PIP: {
        out_w = 256;
        out_h = 192;

        // Full main screen
        std::memcpy(g_composite_buf.data(), src_top, 256 * 192 * sizeof(uint32_t));

        // Inset sub-screen scaled down 3x (85 x 64) in bottom right corner
        const int pip_w = 85;
        const int pip_h = 64;
        const int pip_x = 256 - pip_w - 2;
        const int pip_y = 192 - pip_h - 2;

        // 1-pixel border
        for (int x = pip_x - 1; x <= pip_x + pip_w; ++x) {
          g_composite_buf[(pip_y - 1) * 256 + x] = 0x00FFFFFF;
          g_composite_buf[(pip_y + pip_h) * 256 + x] = 0x00FFFFFF;
        }
        for (int y = pip_y - 1; y <= pip_y + pip_h; ++y) {
          g_composite_buf[y * 256 + (pip_x - 1)] = 0x00FFFFFF;
          g_composite_buf[y * 256 + (pip_x + pip_w)] = 0x00FFFFFF;
        }

        for (int y = 0; y < pip_h; ++y) {
          int src_y = y * 3;
          for (int x = 0; x < pip_w; ++x) {
            int src_x = x * 3;
            g_composite_buf[(pip_y + y) * 256 + (pip_x + x)] = src_bot[src_y * 256 + src_x];
          }
        }

        if (g_show_cursor && g_swap_screens) {
          draw_cursor(g_composite_buf.data(), 256, static_cast<int>(g_stylus_x), static_cast<int>(g_stylus_y), 256, 192);
        }
        break;
      }
    }

    video_cb(g_composite_buf.data(), out_w, out_h, out_w * sizeof(uint32_t));
  }
}

#ifdef __cplusplus
}
#endif
