#pragma once

#include <stddef.h>
#include <stdint.h>

#define GP_NV_MAX_SETTINGS 32
#define GP_NV_KEY_LENGTH 64

typedef struct GpNvSetting {
    uint32_t id;
    uint32_t value;
    uint32_t present;
    char key[GP_NV_KEY_LENGTH];
} GpNvSetting;

typedef struct GpNvSnapshot {
    uint32_t schema_version;
    uint32_t initialized;
    uint32_t gpu_found;
    uint32_t driver_version;
    uint32_t profile_found;
    uint32_t profile_created;
    uint32_t scaling_supported;
    uint32_t scaling;
    uint32_t setting_count;
    char gpu_name[128];
    char driver_branch[64];
    char profile_name[128];
    GpNvSetting settings[GP_NV_MAX_SETTINGS];
} GpNvSnapshot;

typedef struct GpNvApplyReport {
    uint32_t settings_applied;
    uint32_t settings_skipped;
    uint32_t settings_unsupported;
    uint32_t profile_found;
    uint32_t profile_created;
    uint32_t scaling_requested;
    uint32_t scaling_applied;
    char scaling_message[256];
} GpNvApplyReport;

#ifdef __cplusplus
extern "C" {
#endif

int gp_nvapi_capture(GpNvSnapshot* snapshot, uint32_t create_profile,
                     const uint16_t* executable, const uint16_t* profile_name,
                     const uint16_t* friendly_name,
                     char* error, size_t error_size);
int gp_nvapi_apply(const GpNvSnapshot* snapshot, uint32_t restore_mode,
                   const uint16_t* executable, const uint16_t* profile_name,
                   const uint16_t* friendly_name,
                   GpNvApplyReport* report, char* error, size_t error_size);

#ifdef __cplusplus
}
#endif
