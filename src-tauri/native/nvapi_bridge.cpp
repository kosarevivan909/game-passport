#include "nvapi_bridge.h"

#include "nvapi.h"
#include "NvApiDriverSettings.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

namespace {

struct PortableSetting {
    NvU32 id;
    const char* key;
};

// Only public DWORD settings from NVIDIA's NvApiDriverSettings.h are accepted.
static const PortableSetting kPortableSettings[] = {
    {PREFERRED_PSTATE_ID, "power_management_mode"},
    {FRL_FPS_ID, "max_frame_rate"},
    {VSYNCMODE_ID, "vertical_sync"},
    {QUALITY_ENHANCEMENTS_ID, "texture_filtering_quality"},
    {PS_SHADERDISKCACHE_ID, "shader_cache"},
    {PS_SHADERDISKCACHE_MAX_SIZE_ID, "shader_cache_size"},
    {ANISO_MODE_SELECTOR_ID, "anisotropic_filtering_mode"},
    {ANISO_MODE_LEVEL_ID, "anisotropic_filtering_level"},
    {PS_TEXFILTER_ANISO_OPTS2_ID, "anisotropic_sample_optimization"},
    {PS_TEXFILTER_BILINEAR_IN_ANISO_ID, "anisotropic_filter_optimization"},
    {PS_TEXFILTER_DISABLE_TRILIN_SLOPE_ID, "trilinear_optimization"},
    {PS_TEXFILTER_NO_NEG_LODBIAS_ID, "negative_lod_bias"},
    {PRERENDERLIMIT_ID, "maximum_pre_rendered_frames"},
    {FXAA_ENABLE_ID, "fxaa"},
    {MAXWELL_B_SAMPLE_INTERLEAVE_ID, "mfaa"},
    {REFRESH_RATE_OVERRIDE_ID, "preferred_refresh_rate"},
};

void set_error(char* buffer, size_t size, const char* message, NvAPI_Status status) {
    if (!buffer || size == 0) return;
    NvAPI_ShortString nv_message = {};
    NvAPI_GetErrorMessage(status, nv_message);
    if (nv_message[0]) {
        snprintf(buffer, size, "%s NVAPI status %d: %s", message, status, nv_message);
    } else {
        snprintf(buffer, size, "%s NVAPI status %d.", message, status);
    }
}

void copy_ascii(char* destination, size_t size, const char* source) {
    if (!destination || size == 0) return;
    strncpy_s(destination, size, source ? source : "", _TRUNCATE);
}

void copy_wide_to_ascii(char* destination, size_t size, const NvU16* source) {
    if (!destination || size == 0) return;
    destination[0] = '\0';
    if (!source) return;
    size_t index = 0;
    while (source[index] && index + 1 < size) {
        NvU16 value = source[index];
        destination[index] = value < 128 ? static_cast<char>(value) : '?';
        ++index;
    }
    destination[index] = '\0';
}

bool is_whitelisted(NvU32 id) {
    for (const auto& setting : kPortableSettings) {
        if (setting.id == id) return true;
    }
    return false;
}

NvAPI_Status open_application_profile(NvDRSSessionHandle session, bool create,
                                      const NvU16* executable, const NvU16* profile_name,
                                      const NvU16* friendly_name,
                                      NvDRSProfileHandle* profile, bool* found,
                                      bool* created) {
    *found = false;
    *created = false;
    NVDRS_APPLICATION application = {};
    application.version = NVDRS_APPLICATION_VER;
    NvAPI_Status status = NvAPI_DRS_FindApplicationByName(
        session, const_cast<NvU16*>(executable),
        profile, &application);
    if (status == NVAPI_OK) {
        *found = true;
        return NVAPI_OK;
    }
    if (!create && status == NVAPI_EXECUTABLE_NOT_FOUND) return status;
    if (!create) return status;

    NVDRS_PROFILE profile_info = {};
    profile_info.version = NVDRS_PROFILE_VER;
    wcsncpy_s(reinterpret_cast<wchar_t*>(profile_info.profileName),
              NVAPI_UNICODE_STRING_MAX, reinterpret_cast<const wchar_t*>(profile_name), _TRUNCATE);
    status = NvAPI_DRS_FindProfileByName(
        session, const_cast<NvU16*>(profile_name),
        profile);
    if (status != NVAPI_OK) {
        status = NvAPI_DRS_CreateProfile(session, &profile_info, profile);
        if (status != NVAPI_OK) return status;
        *created = true;
    }

    application = {};
    application.version = NVDRS_APPLICATION_VER;
    wcsncpy_s(reinterpret_cast<wchar_t*>(application.appName),
              NVAPI_UNICODE_STRING_MAX, reinterpret_cast<const wchar_t*>(executable), _TRUNCATE);
    wcsncpy_s(reinterpret_cast<wchar_t*>(application.userFriendlyName),
              NVAPI_UNICODE_STRING_MAX, reinterpret_cast<const wchar_t*>(friendly_name), _TRUNCATE);
    status = NvAPI_DRS_CreateApplication(session, *profile, &application);
    if (status == NVAPI_OK) *created = true;
    if (status == NVAPI_EXECUTABLE_ALREADY_IN_USE) {
        application = {};
        application.version = NVDRS_APPLICATION_VER;
        status = NvAPI_DRS_FindApplicationByName(
            session, const_cast<NvU16*>(executable),
            profile, &application);
    }
    if (status == NVAPI_OK) *found = true;
    return status;
}

NvAPI_Status allocate_display_config(NvU32* path_count,
                                     NV_DISPLAYCONFIG_PATH_INFO** path_info) {
    NvU32 count = 0;
    NvAPI_Status status = NvAPI_DISP_GetDisplayConfig(&count, nullptr);
    if (status != NVAPI_OK || count == 0) return status;
    auto* paths = static_cast<NV_DISPLAYCONFIG_PATH_INFO*>(
        calloc(count, sizeof(NV_DISPLAYCONFIG_PATH_INFO)));
    if (!paths) return NVAPI_OUT_OF_MEMORY;
    for (NvU32 i = 0; i < count; ++i) paths[i].version = NV_DISPLAYCONFIG_PATH_INFO_VER;
    status = NvAPI_DISP_GetDisplayConfig(&count, paths);
    if (status != NVAPI_OK) {
        free(paths);
        return status;
    }
    for (NvU32 i = 0; i < count; ++i) {
        paths[i].sourceModeInfo = static_cast<NV_DISPLAYCONFIG_SOURCE_MODE_INFO*>(
            calloc(1, sizeof(NV_DISPLAYCONFIG_SOURCE_MODE_INFO)));
        paths[i].targetInfo = static_cast<NV_DISPLAYCONFIG_PATH_TARGET_INFO*>(
            calloc(paths[i].targetInfoCount, sizeof(NV_DISPLAYCONFIG_PATH_TARGET_INFO)));
        if (!paths[i].sourceModeInfo || !paths[i].targetInfo) {
            status = NVAPI_OUT_OF_MEMORY;
            goto cleanup;
        }
        for (NvU32 j = 0; j < paths[i].targetInfoCount; ++j) {
            paths[i].targetInfo[j].details =
                static_cast<NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO*>(
                    calloc(1, sizeof(NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO)));
            if (!paths[i].targetInfo[j].details) {
                status = NVAPI_OUT_OF_MEMORY;
                goto cleanup;
            }
            paths[i].targetInfo[j].details->version =
                NV_DISPLAYCONFIG_PATH_ADVANCED_TARGET_INFO_VER;
        }
    }
    status = NvAPI_DISP_GetDisplayConfig(&count, paths);
    if (status != NVAPI_OK) goto cleanup;
    *path_count = count;
    *path_info = paths;
    return NVAPI_OK;

cleanup:
    for (NvU32 i = 0; i < count; ++i) {
        if (paths[i].targetInfo) {
            for (NvU32 j = 0; j < paths[i].targetInfoCount; ++j)
                free(paths[i].targetInfo[j].details);
        }
        free(paths[i].targetInfo);
        free(paths[i].sourceModeInfo);
    }
    free(paths);
    return status;
}

void free_display_config(NvU32 path_count, NV_DISPLAYCONFIG_PATH_INFO* paths) {
    if (!paths) return;
    for (NvU32 i = 0; i < path_count; ++i) {
        if (paths[i].targetInfo) {
            for (NvU32 j = 0; j < paths[i].targetInfoCount; ++j)
                free(paths[i].targetInfo[j].details);
        }
        free(paths[i].targetInfo);
        free(paths[i].sourceModeInfo);
    }
    free(paths);
}

NV_DISPLAYCONFIG_PATH_INFO* primary_nvidia_path(NvU32 count,
                                                NV_DISPLAYCONFIG_PATH_INFO* paths) {
    for (NvU32 i = 0; i < count; ++i) {
        if (!paths[i].IsNonNVIDIAAdapter && paths[i].sourceModeInfo &&
            paths[i].sourceModeInfo->bGDIPrimary && paths[i].targetInfoCount > 0 &&
            paths[i].targetInfo[0].details) {
            return &paths[i];
        }
    }
    return nullptr;
}

NvAPI_Status get_scaling(NvU32* scaling) {
    NvU32 count = 0;
    NV_DISPLAYCONFIG_PATH_INFO* paths = nullptr;
    NvAPI_Status status = allocate_display_config(&count, &paths);
    if (status != NVAPI_OK) return status;
    auto* path = primary_nvidia_path(count, paths);
    if (!path) {
        free_display_config(count, paths);
        return NVAPI_NVIDIA_DEVICE_NOT_FOUND;
    }
    *scaling = static_cast<NvU32>(path->targetInfo[0].details->scaling);
    free_display_config(count, paths);
    return NVAPI_OK;
}

NvAPI_Status set_scaling(NvU32 desired) {
    NvU32 count = 0;
    NV_DISPLAYCONFIG_PATH_INFO* paths = nullptr;
    NvAPI_Status status = allocate_display_config(&count, &paths);
    if (status != NVAPI_OK) return status;
    auto* path = primary_nvidia_path(count, paths);
    if (!path) {
        free_display_config(count, paths);
        return NVAPI_NVIDIA_DEVICE_NOT_FOUND;
    }
    path->targetInfo[0].details->scaling = static_cast<NV_SCALING>(desired);
    status = NvAPI_DISP_SetDisplayConfig(
        count, paths,
        NV_DISPLAYCONFIG_SAVE_TO_PERSISTENCE | NV_DISPLAYCONFIG_DRIVER_RELOAD_ALLOWED);
    free_display_config(count, paths);
    return status;
}

}  // namespace

extern "C" int gp_nvapi_capture(GpNvSnapshot* snapshot, uint32_t create_profile,
                                 const uint16_t* executable, const uint16_t* profile_name,
                                 const uint16_t* friendly_name,
                                 char* error,
                                 size_t error_size) {
    if (!snapshot || !executable || !profile_name || !friendly_name) return -1;
    memset(snapshot, 0, sizeof(*snapshot));
    snapshot->schema_version = 1;
    NvAPI_Status status = NvAPI_Initialize();
    if (status != NVAPI_OK) {
        set_error(error, error_size, "NvAPI_Initialize failed.", status);
        return status;
    }
    snapshot->initialized = 1;

    NvPhysicalGpuHandle handles[NVAPI_MAX_PHYSICAL_GPUS] = {};
    NvU32 gpu_count = 0;
    status = NvAPI_EnumPhysicalGPUs(handles, &gpu_count);
    if (status != NVAPI_OK || gpu_count == 0) {
        set_error(error, error_size, "No NVIDIA GPU was reported.", status);
        return status == NVAPI_OK ? NVAPI_NVIDIA_DEVICE_NOT_FOUND : status;
    }
    snapshot->gpu_found = 1;
    NvAPI_ShortString gpu_name = {};
    if (NvAPI_GPU_GetFullName(handles[0], gpu_name) == NVAPI_OK)
        copy_ascii(snapshot->gpu_name, sizeof(snapshot->gpu_name), gpu_name);
    NvAPI_ShortString branch = {};
    NvU32 version = 0;
    if (NvAPI_SYS_GetDriverAndBranchVersion(&version, branch) == NVAPI_OK) {
        snapshot->driver_version = version;
        copy_ascii(snapshot->driver_branch, sizeof(snapshot->driver_branch), branch);
    }

    NvDRSSessionHandle session = nullptr;
    status = NvAPI_DRS_CreateSession(&session);
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not create an NVIDIA DRS session.", status);
        return status;
    }
    status = NvAPI_DRS_LoadSettings(session);
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not load NVIDIA driver profiles.", status);
        NvAPI_DRS_DestroySession(session);
        return status;
    }
    NvDRSProfileHandle profile = nullptr;
    bool found = false;
    bool created = false;
    status = open_application_profile(session, create_profile != 0,
        reinterpret_cast<const NvU16*>(executable), reinterpret_cast<const NvU16*>(profile_name),
        reinterpret_cast<const NvU16*>(friendly_name), &profile, &found, &created);
    if (!create_profile && status == NVAPI_EXECUTABLE_NOT_FOUND) {
        snapshot->profile_found = 0;
        snapshot->profile_created = 0;
        copy_ascii(snapshot->profile_name, sizeof(snapshot->profile_name),
                   "No existing application profile");
        NvAPI_DRS_DestroySession(session);
        NvU32 scaling = 0;
        if (get_scaling(&scaling) == NVAPI_OK) {
            snapshot->scaling_supported = 1;
            snapshot->scaling = scaling;
        }
        return NVAPI_OK;
    }
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not find or create the NVIDIA application profile.", status);
        NvAPI_DRS_DestroySession(session);
        return status;
    }
    snapshot->profile_found = found ? 1 : 0;
    snapshot->profile_created = created ? 1 : 0;
    copy_ascii(snapshot->profile_name, sizeof(snapshot->profile_name),
               "Game Passport application profile");

    const size_t count = sizeof(kPortableSettings) / sizeof(kPortableSettings[0]);
    for (size_t i = 0; i < count && i < GP_NV_MAX_SETTINGS; ++i) {
        auto& output = snapshot->settings[snapshot->setting_count++];
        output.id = kPortableSettings[i].id;
        copy_ascii(output.key, sizeof(output.key), kPortableSettings[i].key);
        NVDRS_SETTING setting = {};
        setting.version = NVDRS_SETTING_VER;
        status = NvAPI_DRS_GetSetting(session, profile, output.id, &setting);
        if (status == NVAPI_OK && setting.settingType == NVDRS_DWORD_TYPE &&
            setting.settingLocation == NVDRS_CURRENT_PROFILE_LOCATION) {
            output.present = 1;
            output.value = setting.u32CurrentValue;
        }
    }
    if (created) {
        status = NvAPI_DRS_SaveSettings(session);
        if (status != NVAPI_OK) {
            set_error(error, error_size, "Could not save the newly created application profile.", status);
            NvAPI_DRS_DestroySession(session);
            return status;
        }
    }
    NvAPI_DRS_DestroySession(session);

    NvU32 scaling = 0;
    if (get_scaling(&scaling) == NVAPI_OK) {
        snapshot->scaling_supported = 1;
        snapshot->scaling = scaling;
    }
    return NVAPI_OK;
}

extern "C" int gp_nvapi_apply(const GpNvSnapshot* snapshot, uint32_t restore_mode,
                               const uint16_t* executable, const uint16_t* profile_name,
                               const uint16_t* friendly_name,
                               GpNvApplyReport* report, char* error,
                               size_t error_size) {
    if (!snapshot || !report || !executable || !profile_name || !friendly_name) return -1;
    memset(report, 0, sizeof(*report));
    NvAPI_Status status = NvAPI_Initialize();
    if (status != NVAPI_OK) {
        set_error(error, error_size, "NvAPI_Initialize failed.", status);
        return status;
    }

    NvDRSSessionHandle session = nullptr;
    status = NvAPI_DRS_CreateSession(&session);
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not create an NVIDIA DRS session.", status);
        return status;
    }
    status = NvAPI_DRS_LoadSettings(session);
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not load NVIDIA driver profiles.", status);
        NvAPI_DRS_DestroySession(session);
        return status;
    }

    NvDRSProfileHandle profile = nullptr;
    bool found = false;
    bool created = false;
    status = open_application_profile(session, restore_mode == 0,
        reinterpret_cast<const NvU16*>(executable), reinterpret_cast<const NvU16*>(profile_name),
        reinterpret_cast<const NvU16*>(friendly_name), &profile, &found, &created);
    if (restore_mode && !snapshot->profile_found) {
        if (status == NVAPI_OK && found) {
            NVDRS_PROFILE info = {};
            info.version = NVDRS_PROFILE_VER;
            if (NvAPI_DRS_GetProfileInfo(session, profile, &info) == NVAPI_OK &&
                wcscmp(reinterpret_cast<const wchar_t*>(info.profileName), reinterpret_cast<const wchar_t*>(profile_name)) == 0) {
                status = NvAPI_DRS_DeleteProfile(session, profile);
                if (status == NVAPI_OK) status = NvAPI_DRS_SaveSettings(session);
            } else {
                status = NVAPI_OK;
            }
        } else if (status == NVAPI_EXECUTABLE_NOT_FOUND) {
            status = NVAPI_OK;
        }
        NvAPI_DRS_DestroySession(session);
        if (status != NVAPI_OK) set_error(error, error_size, "Could not restore NVIDIA profile state.", status);
        return status;
    }
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not find or create the NVIDIA application profile.", status);
        NvAPI_DRS_DestroySession(session);
        return status;
    }
    report->profile_found = found ? 1 : 0;
    report->profile_created = created ? 1 : 0;

    const uint32_t count = snapshot->setting_count > GP_NV_MAX_SETTINGS
                               ? GP_NV_MAX_SETTINGS
                               : snapshot->setting_count;
    for (uint32_t i = 0; i < count; ++i) {
        const auto& input = snapshot->settings[i];
        if (!is_whitelisted(input.id)) {
            ++report->settings_unsupported;
            continue;
        }
        if (restore_mode && !input.present) {
            status = NvAPI_DRS_DeleteProfileSetting(session, profile, input.id);
            if (status == NVAPI_OK || status == NVAPI_SETTING_NOT_FOUND) {
                ++report->settings_applied;
            } else {
                ++report->settings_skipped;
            }
            continue;
        }
        if (!input.present) continue;
        NVDRS_SETTING setting = {};
        setting.version = NVDRS_SETTING_VER;
        setting.settingId = input.id;
        setting.settingType = NVDRS_DWORD_TYPE;
        setting.u32CurrentValue =
            (!restore_mode && input.id == REFRESH_RATE_OVERRIDE_ID)
                ? REFRESH_RATE_OVERRIDE_HIGHEST_AVAILABLE
                : input.value;
        status = NvAPI_DRS_SetSetting(session, profile, &setting);
        if (status == NVAPI_OK)
            ++report->settings_applied;
        else
            ++report->settings_skipped;
    }

    if (!restore_mode) {
        bool refresh_in_snapshot = false;
        for (uint32_t i = 0; i < count; ++i)
            if (snapshot->settings[i].id == REFRESH_RATE_OVERRIDE_ID &&
                snapshot->settings[i].present)
                refresh_in_snapshot = true;
        if (!refresh_in_snapshot) {
            NVDRS_SETTING refresh = {};
            refresh.version = NVDRS_SETTING_VER;
            refresh.settingId = REFRESH_RATE_OVERRIDE_ID;
            refresh.settingType = NVDRS_DWORD_TYPE;
            refresh.u32CurrentValue = REFRESH_RATE_OVERRIDE_HIGHEST_AVAILABLE;
            status = NvAPI_DRS_SetSetting(session, profile, &refresh);
            if (status == NVAPI_OK)
                ++report->settings_applied;
            else
                ++report->settings_skipped;
        }
    }
    status = NvAPI_DRS_SaveSettings(session);
    NvAPI_DRS_DestroySession(session);
    if (status != NVAPI_OK) {
        set_error(error, error_size, "Could not persist NVIDIA application profile settings.", status);
        return status;
    }

    if (snapshot->scaling_supported) {
        report->scaling_requested = 1;
        status = set_scaling(snapshot->scaling);
        if (status == NVAPI_OK) {
            report->scaling_applied = 1;
            copy_ascii(report->scaling_message, sizeof(report->scaling_message),
                       "NVAPI accepted the requested scaling mode.");
        } else {
            set_error(report->scaling_message, sizeof(report->scaling_message),
                      "NVIDIA scaling was not applied.", status);
        }
    } else {
        copy_ascii(report->scaling_message, sizeof(report->scaling_message),
                   "No portable scaling value was available in the snapshot.");
    }
    return NVAPI_OK;
}
