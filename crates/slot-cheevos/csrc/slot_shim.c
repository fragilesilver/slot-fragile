// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/csrc/slot_shim.c.
#include <stdlib.h>
#include <string.h>

#include "rc_client.h"
#include "rc_libretro.h"

typedef struct slot_event_t {
  uint32_t type;
  uint32_t points;
  const char* title;
  const char* description;
  const char* badge_url;
  const char* message;
} slot_event_t;

typedef struct slot_game_t {
  const char* title;
  const char* badge_url;
  uint32_t console_id;
  uint32_t unlocked;
  uint32_t total;
} slot_game_t;

static void RC_CCONV slot_no_core_memory(uint32_t id, rc_libretro_core_memory_info_t* info) {
  (void)id;
  info->data = NULL;
  info->size = 0;
}

void* slot_regions_new(const struct retro_memory_descriptor* descriptors, unsigned count, uint32_t console_id) {
  struct retro_memory_map map;
  rc_libretro_memory_regions_t* regions = (rc_libretro_memory_regions_t*)calloc(1, sizeof(*regions));
  if (!regions)
    return NULL;
  map.descriptors = descriptors;
  map.num_descriptors = count;
  if (!rc_libretro_memory_init(regions, &map, slot_no_core_memory, console_id)) {
    free(regions);
    return NULL;
  }
  return regions;
}

void slot_regions_free(void* regions) {
  if (!regions)
    return;
  rc_libretro_memory_destroy((rc_libretro_memory_regions_t*)regions);
  free(regions);
}

uint32_t slot_regions_read(const void* regions, uint32_t address, uint8_t* buffer, uint32_t num_bytes) {
  return rc_libretro_memory_read((const rc_libretro_memory_regions_t*)regions, address, buffer, num_bytes);
}

void slot_request_fields(const rc_api_request_t* request, const char** url, const char** post_data, const char** content_type) {
  *url = request->url;
  *post_data = request->post_data;
  *content_type = request->content_type;
}

void slot_respond(rc_client_server_callback_t callback, void* callback_data, const char* body, size_t body_length, int http_status_code) {
  rc_api_server_response_t response;
  response.body = body;
  response.body_length = body_length;
  response.http_status_code = http_status_code;
  callback(&response, callback_data);
}

void slot_event_fields(const rc_client_event_t* event, slot_event_t* out) {
  memset(out, 0, sizeof(*out));
  out->type = event->type;
  if (event->achievement) {
    out->title = event->achievement->title;
    out->description = event->achievement->description;
    out->badge_url = event->achievement->badge_url;
    out->points = event->achievement->points;
  }
  if (event->subset) {
    out->title = event->subset->title;
    out->badge_url = event->subset->badge_url;
  }
  if (event->server_error)
    out->message = event->server_error->error_message;
}

int slot_game_fields(rc_client_t* client, slot_game_t* out) {
  rc_client_user_game_summary_t summary;
  const rc_client_game_t* game = rc_client_get_game_info(client);
  memset(out, 0, sizeof(*out));
  if (!game || !game->id)
    return 0;
  rc_client_get_user_game_summary(client, &summary);
  out->title = game->title;
  out->badge_url = game->badge_url;
  out->console_id = game->console_id;
  out->unlocked = summary.num_unlocked_achievements;
  out->total = summary.num_core_achievements;
  return 1;
}

const char* slot_user_token(const rc_client_t* client) {
  const rc_client_user_t* user = rc_client_get_user_info(client);
  return user ? user->token : NULL;
}

const char* slot_user_name(const rc_client_t* client) {
  const rc_client_user_t* user = rc_client_get_user_info(client);
  return user ? user->display_name : NULL;
}

/* --- Fragile: the loaded game's achievements for the in-game list, and signing out --- */

typedef void (*slot_achievement_cb)(void* data, const char* title, const char* description,
                                    uint32_t points, int unlocked);

/* Calls `cb` for each of the loaded game's core achievements, unlocked ones first as rcheevos
 * buckets them. Returns 0 when no game is loaded. */
int slot_achievements(rc_client_t* client, slot_achievement_cb cb, void* data) {
  uint32_t b, a;
  rc_client_achievement_list_t* list = rc_client_create_achievement_list(
      client, RC_CLIENT_ACHIEVEMENT_CATEGORY_CORE, RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_LOCK_STATE);
  if (!list)
    return 0;
  for (b = 0; b < list->num_buckets; ++b) {
    const rc_client_achievement_bucket_t* bucket = &list->buckets[b];
    for (a = 0; a < bucket->num_achievements; ++a) {
      const rc_client_achievement_t* ach = bucket->achievements[a];
      cb(data, ach->title, ach->description, ach->points, ach->unlocked != 0);
    }
  }
  rc_client_destroy_achievement_list(list);
  return 1;
}
