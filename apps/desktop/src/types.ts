export type RangeKey = "today" | "7d" | "30d";

export type Provider = "openai" | "anthropic";

export interface UsageSlice {
  name: string;
  seconds: number;
  percentage: number;
  color: string;
  detail?: string;
}

export interface ActivityEvent {
  id: string;
  appName: string;
  windowTitle?: string;
  url?: string;
  pageTitle?: string;
  searchQuery?: string;
  browserProfile?: string;
  startedAt: string;
  durationSeconds: number;
  topic?: string;
  source: "collector" | "chrome" | "history" | "editor" | "firefox" | "safari";
}

export interface ActivityPreview {
  kind: "youtube" | "link";
  url: string;
  title?: string;
  thumbnailDataUrl?: string;
  embedUrl?: string;
}

export interface TopicInsight {
  id: string;
  title: string;
  description: string;
  metric: string;
  evidence: string;
}

export type RecommendationKind = "continuity" | "behavioral";

export interface Recommendation {
  id: string;
  kind: RecommendationKind;
  title: string;
  body: string;
  evidence: string;
  createdAt: string;
}

export interface DashboardData {
  range: RangeKey;
  trackedSeconds: number;
  focusedSeconds: number;
  activeTopics: ActiveTopic[];
  appUsage: UsageSlice[];
  siteUsage: UsageSlice[];
  recentActivity: ActivityEvent[];
  insights: TopicInsight[];
  recommendations: Recommendation[];
  generatedAt?: string;
}

export interface ActiveTopic {
  name: string;
  count: number;
}

export interface ProfileItem {
  id: string;
  label: string;
  description?: string;
  confidence?: number;
  provenance: "observed" | "inferred" | "user";
}

export interface ProfileSection {
  id: string;
  title: string;
  items: ProfileItem[];
}

export interface ProfileData {
  summary: string;
  sections: ProfileSection[];
  updatedAt?: string;
  memorySync?: {
    status: "everos" | "local-only";
    message: string;
  };
}

export interface BrowserProfile {
  id: string;
  browser: "chrome" | "firefox" | "safari";
  name: string;
  path: string;
  selected: boolean;
  support: "required" | "best-effort" | "unavailable";
}

export interface CollectionStatus {
  enabled: boolean;
  accessibilityGranted: boolean;
  browserConnected: boolean;
  lastCapturedAt?: string;
  dataPath?: string;
  degradedReasons: string[];
}

export interface PairingInfo {
  nativeHost: string;
  pairingToken: string;
  localhostEndpoint: string;
  protocolVersion: number;
}

export interface SettingsData {
  provider: Provider;
  hasProviderKey: boolean;
  behavioralGuidanceEnabled: boolean;
  launchAtLogin: boolean;
  selectedBrowserProfileIds: string[];
  excludedApps: string[];
  excludedDomains: string[];
  collectionStatus: CollectionStatus;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  createdAt: string;
}

export type ChatMode = "optimized";

export interface MemoryRecord {
  id: string;
  text: string;
  memoryType: string;
  source: string;
  createdAt: number;
  importance?: number;
  score?: number;
}

export interface ContextEconomics {
  queryId: string;
  mode: ChatMode;
  model: string;
  baselineInputTokens: number;
  optimizedInputTokens: number;
  tokensSaved: number;
  reductionPercent: number;
  actualInputTokens?: number;
  outputTokens?: number;
  latencyMs: number;
  estimatedCostUsd?: number;
  memoryCount: number;
  measurementMethod: string;
  telemetryStatus: string;
  baselineContextPreview: string;
  optimizedContextPreview: string;
}

export interface ChatRunResult {
  message: ChatMessage;
  retrievedMemories: MemoryRecord[];
  economics: ContextEconomics;
  integration: {
    memoryProvider: string;
    memoryWarning?: string;
    telemetryStatus: string;
  };
}

export interface IntegrationStatus {
  everos: {
    configured: boolean;
    message: string;
  };
  snowflake: {
    configured: boolean;
    message: string;
    promptTokenizationEnabled: boolean;
  };
}

export interface MemorySyncReceipt {
  provider: string;
  storedCount: number;
  message: string;
}

export interface BootstrapStatus {
  phase: "not-started" | "importing" | "profiling" | "complete" | "error";
  importedEvents: number;
  progress: number;
  message: string;
}
