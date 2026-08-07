import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityEvent,
  BootstrapStatus,
  BrowserProfile,
  ChatMode,
  ChatMessage,
  ChatRunResult,
  DashboardData,
  IntegrationStatus,
  MemorySyncReceipt,
  PairingInfo,
  ProfileData,
  Provider,
  RangeKey,
  SettingsData,
} from "../types";
import {
  mockActivity,
  mockBrowsers,
  mockDashboard,
  mockProfile,
  mockSettings,
} from "./mockData";

const isTauri = () => "__TAURI_INTERNALS__" in window;
export const isDesktopRuntime = isTauri;

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauri()) {
    return fallback === undefined ? (undefined as T) : structuredClone(fallback);
  }
  return invoke<T>(command, args);
}

export const api = {
  activityIcon: (appName: string, url?: string) => {
    let previewIcon: string | undefined;
    if (url) {
      try {
        previewIcon = new URL("/favicon.ico", url).toString();
      } catch {
        previewIcon = undefined;
      }
    }
    return call<string | null>("get_activity_icon", { appName, url }, previewIcon ?? null);
  },
  dashboard: (range: RangeKey) =>
    call<DashboardData>("get_dashboard", { range }, { ...mockDashboard, range }),
  activity: (range: RangeKey, query = "") =>
    call<ActivityEvent[]>("get_activity_history", { range, query }, mockActivity),
  profile: () => call<ProfileData>("get_profile", undefined, mockProfile),
  settings: () => call<SettingsData>("get_settings", undefined, mockSettings),
  browserProfiles: () => call<BrowserProfile[]>("get_browser_profiles", undefined, mockBrowsers),
  bootstrapStatus: () =>
    call<BootstrapStatus>(
      "get_bootstrap_status",
      undefined,
      { phase: "not-started", importedEvents: 0, progress: 0, message: "Ready to import browser history." },
    ),
  setCollectionEnabled: (enabled: boolean) =>
    call<SettingsData>("set_collection_enabled", { enabled }, { ...mockSettings, collectionStatus: { ...mockSettings.collectionStatus, enabled } }),
  requestAccessibility: () => call<boolean>("request_accessibility_permission", undefined, false),
  setBrowserProfiles: (profileIds: string[]) =>
    call<void>("set_browser_profiles", { profileIds }, undefined),
  startBootstrap: () => call<BootstrapStatus>("start_bootstrap", undefined, undefined),
  reimportChromeHistory: () =>
    call<ProfileData>("reimport_chrome_history", undefined, mockProfile),
  refreshProfile: () => call<ProfileData>("refresh_profile", undefined, mockProfile),
  saveCorrection: (label: string, description?: string, id?: string) =>
    call<ProfileData>("save_profile_correction", { id, label, description }, mockProfile),
  removeCorrection: (id: string) =>
    call<ProfileData>("remove_profile_correction", { id }, mockProfile),
  dismissInference: (id: string) =>
    call<ProfileData>("dismiss_profile_inference", { id }, mockProfile),
  saveProfileSummary: (summary: string) =>
    call<ProfileData>("save_profile_summary", { summary }, { ...mockProfile, summary }),
  syncProfileMemories: () =>
    call<MemorySyncReceipt>(
      "sync_profile_memories",
      undefined,
      { provider: "preview", storedCount: mockProfile.sections.length, message: "Preview mode uses sample memories." },
    ),
  seedDemoMemories: () =>
    call<MemorySyncReceipt>(
      "seed_demo_memories",
      undefined,
      { provider: "preview", storedCount: 3, message: "Seeded 3 sample preview memories." },
    ),
  integrationStatus: () =>
    call<IntegrationStatus>(
      "get_integration_status",
      undefined,
      {
        everos: { configured: false, message: "Browser preview uses sample memories." },
        snowflake: { configured: false, message: "Browser preview uses sample telemetry.", promptTokenizationEnabled: false },
      },
    ),
  saveProviderKey: (provider: Provider, key: string) =>
    call<void>("save_provider_key", { provider, key }, undefined),
  removeProviderKey: (provider: Provider) =>
    call<void>("remove_provider_key", { provider }, undefined),
  testProvider: (provider: Provider) =>
    call<string>("test_provider", { provider }, "Connection successful."),
  pairingInfo: () =>
    call<PairingInfo>(
      "get_pairing_info",
      undefined,
      {
        nativeHost: "com.knowu.companion",
        pairingToken: "preview-only",
        localhostEndpoint: "http://127.0.0.1:48321",
        protocolVersion: 1,
      },
    ),
  installNativeHost: (extensionId: string) =>
    call<string>("install_native_host", { extensionId }, "Preview mode does not install a native host."),
  saveSettings: (settings: Partial<SettingsData>) =>
    call<SettingsData>("save_settings", { settings }, { ...mockSettings, ...settings }),
  dismissRecommendation: (id: string, feedback?: string) =>
    call<void>("dismiss_recommendation", { id, feedback }, undefined),
  chat: (messages: ChatMessage[], mode: ChatMode = "optimized", contextBrief?: string) =>
    call<ChatRunResult>(
      "chat",
      { messages, mode, contextBrief },
      {
        message: {
          id: crypto.randomUUID(),
          role: "assistant",
          content:
            "I’m running in browser preview mode, so this is a sample KnowU answer. The native app retrieves these memories from EverOS and records aggregate economics in Snowflake.",
          createdAt: new Date().toISOString(),
        },
        retrievedMemories: [
          {
            id: "preview-memory",
            text: "Prefers local-first architecture and explicit privacy boundaries.",
            memoryType: "preference",
            source: "preview",
            createdAt: Math.floor(Date.now() / 1000),
            score: 0.94,
          },
        ],
        economics: {
          queryId: crypto.randomUUID(),
          mode,
          model: "preview-model",
          baselineInputTokens: 3842,
          optimizedInputTokens: 721,
          tokensSaved: 3121,
          reductionPercent: 81.23,
          outputTokens: 84,
          latencyMs: 620,
          memoryCount: 1,
          measurementMethod: "preview_sample",
          telemetryStatus: "preview-only",
          baselineContextPreview: "Sample full profile and summarized activity context.",
          optimizedContextPreview: "Sample query-specific approved memory context.",
        },
        integration: {
          memoryProvider: "preview",
          telemetryStatus: "preview-only",
        },
      },
    ),
  deleteAllData: () => call<void>("delete_all_data", undefined, undefined),
};
