import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { api } from "./lib/api";
import { mockBrowsers, mockDashboard, mockProfile, mockSettings } from "./lib/mockData";
import type { ActivityEvent, ChatMessage, ChatRunResult, ProfileData } from "./types";

function clone<T>(value: T): T {
  return structuredClone(value);
}

function chatRun(message: ChatMessage): ChatRunResult {
  return {
    message,
    retrievedMemories: [
      {
        id: "memory-1",
        text: "Privacy is more important than feature count.",
        memoryType: "preference",
        source: "explicit_user",
        createdAt: 1,
        score: 0.95,
      },
    ],
    economics: {
      queryId: "query-1234",
      mode: "optimized",
      model: "gpt-5-mini",
      baselineInputTokens: 1000,
      optimizedInputTokens: 250,
      tokensSaved: 750,
      reductionPercent: 75,
      outputTokens: 50,
      latencyMs: 400,
      memoryCount: 1,
      measurementMethod: "provider_usage_scaled_estimate",
      telemetryStatus: "synced-to-snowflake",
      baselineContextPreview: "Full approved profile context",
      optimizedContextPreview: "Privacy is more important than feature count.\nQUERY-SPECIFIC LOCAL ACTIVITY FACTS\nSnowflake: matched events=220",
    },
    integration: {
      memoryProvider: "everos",
      telemetryStatus: "synced-to-snowflake",
    },
  };
}

function stubApi() {
  sessionStorage.clear();
  localStorage.removeItem("knowu.selected-thread");
  vi.spyOn(api, "settings").mockResolvedValue(clone(mockSettings));
  vi.spyOn(api, "openResource").mockResolvedValue(undefined);
  vi.spyOn(api, "dashboard").mockResolvedValue(clone(mockDashboard));
  vi.spyOn(api, "activity").mockResolvedValue(clone(mockDashboard.recentActivity));
  vi.spyOn(api, "profile").mockResolvedValue(clone(mockProfile));
  vi.spyOn(api, "browserProfiles").mockResolvedValue(clone(mockBrowsers));
  vi.spyOn(api, "setCollectionEnabled").mockImplementation(async (enabled) => ({
    ...clone(mockSettings),
    collectionStatus: { ...clone(mockSettings.collectionStatus), enabled },
  }));
  vi.spyOn(api, "saveSettings").mockImplementation(async (settings) => ({
    ...clone(mockSettings),
    ...settings,
  }));
  vi.spyOn(api, "setBrowserProfiles").mockResolvedValue(undefined);
  vi.spyOn(api, "reimportChromeHistory").mockResolvedValue(clone(mockProfile));
  vi.spyOn(api, "dismissRecommendation").mockResolvedValue(undefined);
  vi.spyOn(api, "refreshProfile").mockResolvedValue(clone(mockProfile));
  vi.spyOn(api, "integrationStatus").mockResolvedValue({
    everos: { configured: true, message: "EverOS configured" },
    snowflake: { configured: true, message: "Snowflake configured", promptTokenizationEnabled: false },
  });
}

async function renderRoute(hash: string) {
  window.location.hash = hash;
  render(<App />);
  await screen.findByText("KnowU");
}

function dashboardWithPreviewEvents(events: ActivityEvent[]) {
  vi.mocked(api.dashboard).mockResolvedValue({
    ...clone(mockDashboard),
    activeTopics: [{ name: "Classical music", count: events.length }],
    recentActivity: events,
    recommendations: [],
  });
}

function previewEvent(
  id: string,
  startedAt: string,
  url?: string,
  pageTitle = "Classical music",
): ActivityEvent {
  return {
    id,
    appName: url ? "Google Chrome" : "Music",
    pageTitle,
    url,
    startedAt,
    durationSeconds: 300,
    topic: "Classical music",
    source: url ? "history" : "collector",
  };
}

describe("application navigation", () => {
  beforeEach(stubApi);

  it("redirects unknown routes to the dashboard", async () => {
    await renderRoute("#/unknown");

    expect(await screen.findByRole("heading", { name: "Pick up where you left off." })).toBeInTheDocument();
  });

  it("navigates from the dashboard to activity history", async () => {
    await renderRoute("#/dashboard");

    fireEvent.click(screen.getByRole("link", { name: "Activity" }));

    expect(await screen.findByRole("heading", { name: "Your local timeline" })).toBeInTheDocument();
  });

  it("navigates to reconstructed work threads", async () => {
    await renderRoute("#/dashboard");

    fireEvent.click(screen.getByRole("link", { name: "Threads" }));

    expect(await screen.findByRole("heading", { name: "Your threads" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /KnowU implementation/ }));
    expect(screen.getByText("Thread evidence")).toBeInTheDocument();
  });
});

describe("onboarding", () => {
  beforeEach(stubApi);

  it("completes consent without changing the hook order", async () => {
    localStorage.clear();
    window.location.hash = "";
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /Continue/ }));
    fireEvent.click(screen.getByRole("button", { name: /Continue/ }));
    const profiles = await screen.findAllByRole("checkbox");
    fireEvent.click(profiles[0]);
    fireEvent.click(screen.getByRole("button", { name: /Continue/ }));
    fireEvent.change(screen.getByLabelText("API key"), {
      target: { value: "sk-test-only" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Build my first profile/ }));

    expect(await screen.findByRole("heading", { name: "Pick up where you left off." })).toBeInTheDocument();
    expect(localStorage.getItem("knowu.setup-complete")).toBe("true");
  });
});

describe("dashboard", () => {
  beforeEach(stubApi);

  it("leads with a resumable thread and supporting context status", async () => {
    await renderRoute("#/dashboard");

    expect(await screen.findByText("Continue where you left off")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "KnowU implementation" })).toBeInTheDocument();
    expect(screen.getByText("6h 06m observed")).toBeInTheDocument();
    expect(screen.getByText("76.2% sustained focus")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Resume thread/ })).toBeInTheDocument();
  });

  it("opens the latest web resource through the desktop API before reporting success", async () => {
    await renderRoute("#/dashboard");

    fireEvent.click(await screen.findByRole("button", { name: /Resume thread/ }));

    await waitFor(() => {
      expect(api.openResource).toHaveBeenCalledWith("https://v2.tauri.app/security/capabilities/");
      expect(screen.getByRole("status")).toHaveTextContent("Opened the latest available resource.");
    });
  });

  it("reports when the latest web resource cannot be opened", async () => {
    vi.mocked(api.openResource).mockRejectedValueOnce(new Error("open failed"));
    await renderRoute("#/dashboard");

    fireEvent.click(await screen.findByRole("button", { name: /Resume thread/ }));

    expect(await screen.findByText("Could not open the latest available resource.")).toBeInTheDocument();
  });

  it("lists reconstructed work threads and their evidence boundary", async () => {
    await renderRoute("#/dashboard");

    expect(await screen.findByRole("heading", { name: "Active threads" })).toBeInTheDocument();
    expect(screen.getByText("Product planning")).toBeInTheDocument();
    expect(screen.getByText("Desktop development")).toBeInTheDocument();
    expect(screen.getByText("Why this thread?")).toBeInTheDocument();
    expect(screen.getByText("Detailed activity stays local")).toBeInTheDocument();
  });

  it("shows cross-app Snowflake activity as one subject thread", async () => {
    const startedAt = new Date().toISOString();
    const snowflakeActivity: ActivityEvent[] = [
      { id: "snow-1", appName: "Google Chrome", pageTitle: "Snowflake tutorial — YouTube", url: "https://youtube.com/watch?v=snow", startedAt, durationSeconds: 600, topic: "Snowflake", source: "history" },
      { id: "snow-2", appName: "Google Chrome", pageTitle: "snowflake architecture — Google Search", searchQuery: "snowflake architecture", url: "https://google.com/search?q=snowflake", startedAt, durationSeconds: 0, topic: "Snowflake", source: "history" },
      { id: "snow-3", appName: "Google Chrome", pageTitle: "Snowsight", url: "https://app.snowflake.com/example", startedAt, durationSeconds: 300, topic: "Snowflake", source: "chrome" },
      { id: "snow-4", appName: "Preview", pageTitle: "Snowflake migration notes.pdf", startedAt, durationSeconds: 180, topic: "Snowflake", source: "collector" },
      { id: "snow-5", appName: "Cursor", pageTitle: "src/snowflake_client.rs", startedAt, durationSeconds: 0, topic: "Snowflake", source: "editor" },
    ];
    vi.mocked(api.dashboard).mockResolvedValue({
      ...clone(mockDashboard),
      activeTopics: [{ name: "Snowflake", count: 5 }],
      recentActivity: snowflakeActivity,
      recommendations: [],
    });

    await renderRoute("#/dashboard");

    expect(await screen.findByRole("heading", { name: "Snowflake" })).toBeInTheDocument();
    expect(screen.getByText("5 signals")).toBeInTheDocument();
    expect(screen.getAllByText(/across Google Chrome, Preview, Cursor/).length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: /Video research/ })).not.toBeInTheDocument();
  });

  it("formats foreground application percentages to one decimal place", async () => {
    vi.mocked(api.dashboard).mockResolvedValue({
      ...clone(mockDashboard),
      appUsage: [
        { name: "Code", seconds: 60, percentage: 58.333333333333336, color: "#58c7ff" },
        { name: "Google Chrome", seconds: 60, percentage: 41.666666666666664, color: "#adff2f" },
        { name: "Other", seconds: 0, percentage: 0, color: "#78828f" },
      ],
      siteUsage: [
        { name: "example.com", seconds: 60, percentage: 66.66666666666667, color: "#58c7ff" },
        { name: "Other", seconds: 30, percentage: 33.333333333333336, color: "#78828f" },
      ],
    });

    await renderRoute("#/dashboard");

    expect(await screen.findByText("58.3%")).toBeInTheDocument();
    expect(screen.getByText("41.7%")).toBeInTheDocument();
    expect(screen.getByText("0.0%")).toBeInTheDocument();
    expect(screen.getByText("66.7%")).toBeInTheDocument();
    expect(screen.getByText("33.3%")).toBeInTheDocument();
  });

  it("labels observed activity separately from cautious inferences", async () => {
    await renderRoute("#/dashboard");

    expect(await screen.findByText("Observed facts from this Mac")).toBeInTheDocument();
    expect(screen.getByText("Cautious inferences, not conclusions")).toBeInTheDocument();
    expect(screen.getByText("Supporting evidence, not a productivity score")).toBeInTheDocument();
  });

  it("shows website favicons instead of letter placeholders", async () => {
    await renderRoute("#/dashboard");

    await waitFor(() => {
      const favicon = document.querySelector<HTMLImageElement>(".activity-row .app-token img");
      expect(favicon?.src).toBe("https://v2.tauri.app/favicon.ico");
    });
  });

  it("requests new dashboard data when the range changes", async () => {
    const dashboardSpy = vi.mocked(api.dashboard);
    await renderRoute("#/dashboard");

    fireEvent.click(await screen.findByRole("button", { name: "7 days" }));

    await waitFor(() => expect(dashboardSpy).toHaveBeenCalledWith("7d"));
  });
});

describe("dashboard activity preview", () => {
  beforeEach(stubApi);

  it("requests a preview for the latest URL in the selected thread", async () => {
    const newestUrl = "https://www.youtube.com/watch?v=moonlight";
    dashboardWithPreviewEvents([
      previewEvent("latest-no-url", "2026-08-07T18:00:00.000Z"),
      previewEvent("latest-url", "2026-08-07T17:00:00.000Z", newestUrl, "Moonlight Sonata"),
      previewEvent("older-url", "2026-08-07T16:00:00.000Z", "https://example.com/classical"),
    ]);
    const activityPreview = vi.spyOn(api, "activityPreview").mockResolvedValue({
      kind: "youtube",
      title: "Moonlight Sonata",
      url: newestUrl,
      thumbnailDataUrl: "data:image/jpeg;base64,thumbnail",
      embedUrl: "https://www.youtube-nocookie.com/embed/moonlight",
    });

    await renderRoute("#/dashboard");

    await waitFor(() => expect(activityPreview).toHaveBeenCalledWith(newestUrl));
    expect(activityPreview).toHaveBeenCalledTimes(1);
  });

  it("renders YouTube preview metadata returned by the preview API", async () => {
    const url = "https://www.youtube.com/watch?v=moonlight";
    dashboardWithPreviewEvents([
      previewEvent("youtube", "2026-08-07T17:00:00.000Z", url, "YouTube page title"),
    ]);
    vi.spyOn(api, "activityPreview").mockResolvedValue({
      kind: "youtube",
      title: "Beethoven — Moonlight Sonata",
      url,
      thumbnailDataUrl: "data:image/jpeg;base64,thumbnail",
      embedUrl: "https://www.youtube-nocookie.com/embed/moonlight",
    });

    await renderRoute("#/dashboard");

    expect(await screen.findByRole("heading", { name: "Beethoven — Moonlight Sonata" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Beethoven — Moonlight Sonata preview" })).toHaveAttribute(
      "src",
      "data:image/jpeg;base64,thumbnail",
    );
  });

  it("replaces the YouTube thumbnail with an embedded player when activated", async () => {
    const url = "https://www.youtube.com/watch?v=moonlight";
    dashboardWithPreviewEvents([
      previewEvent("youtube", "2026-08-07T17:00:00.000Z", url, "Moonlight Sonata"),
    ]);
    vi.spyOn(api, "activityPreview").mockResolvedValue({
      kind: "youtube",
      title: "Moonlight Sonata",
      url,
      thumbnailDataUrl: "data:image/jpeg;base64,thumbnail",
      embedUrl: "https://www.youtube-nocookie.com/embed/moonlight",
    });
    await renderRoute("#/dashboard");
    await screen.findByRole("img", { name: "Moonlight Sonata preview" });

    fireEvent.click(screen.getByRole("button", { name: "Play Moonlight Sonata preview" }));

    expect(screen.getByTitle("Moonlight Sonata")).toHaveAttribute(
      "src",
      "https://www.youtube-nocookie.com/embed/moonlight?autoplay=1",
    );
    expect(screen.queryByRole("img", { name: "Moonlight Sonata preview" })).not.toBeInTheDocument();
  });

  it("keeps generic site previews non-embedded and openable", async () => {
    const url = "https://example.com/classical-music";
    dashboardWithPreviewEvents([
      previewEvent("website", "2026-08-07T17:00:00.000Z", url, "A guide to classical music"),
    ]);
    vi.spyOn(api, "activityPreview").mockResolvedValue({
      kind: "link",
      title: "A guide to classical music",
      url,
    });

    await renderRoute("#/dashboard");

    const openResource = await screen.findByRole("link", { name: "Open resource" });
    expect(openResource).toHaveAttribute("href", url);
    expect(openResource).toHaveAttribute("target", "_blank");
    expect(document.querySelector("iframe")).not.toBeInTheDocument();
  });

  it("preserves an openable resource when preview loading fails", async () => {
    const url = "https://example.com/classical-music";
    dashboardWithPreviewEvents([
      previewEvent("website", "2026-08-07T17:00:00.000Z", url, "A guide to classical music"),
    ]);
    vi.spyOn(api, "activityPreview").mockRejectedValue(new Error("preview unavailable"));

    await renderRoute("#/dashboard");

    expect(await screen.findByText(/Preview unavailable/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Open resource" })).toHaveAttribute("href", url);
  });
});

describe("profile corrections", () => {
  beforeEach(stubApi);

  it("saves a correction as authoritative user-authored truth", async () => {
    const correctedProfile: ProfileData = {
      ...clone(mockProfile),
      sections: [
        ...clone(mockProfile.sections),
        {
          id: "new-truth",
          title: "Authoritative truth",
          items: [
            {
              id: "correction-2",
              label: "Project Atlas is complete",
              description: "Do not infer that it is active.",
              provenance: "user",
            },
          ],
        },
      ],
    };
    const saveCorrection = vi.spyOn(api, "saveCorrection").mockResolvedValue(correctedProfile);
    await renderRoute("#/profile");
    await screen.findByText(mockProfile.summary);

    fireEvent.click(screen.getByRole("button", { name: "Add correction" }));
    const dialog = screen.getByRole("dialog", { name: "Add authoritative correction" });
    fireEvent.change(within(dialog).getByLabelText("What should KnowU know?"), {
      target: { value: "Project Atlas is complete" },
    });
    fireEvent.change(within(dialog).getByLabelText("Optional context"), {
      target: { value: "Do not infer that it is active." },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save as truth" }));

    await waitFor(() =>
      expect(saveCorrection).toHaveBeenCalledWith(
        "Project Atlas is complete",
        "Do not infer that it is active.",
      ),
    );
    expect(await screen.findByText("Project Atlas is complete")).toBeInTheDocument();
    expect(screen.getAllByText("user").length).toBeGreaterThan(0);
  });

  it("removes a correction by its stable identifier", async () => {
    const updatedProfile: ProfileData = {
      ...clone(mockProfile),
      sections: mockProfile.sections.map((section) => ({
        ...section,
        items: section.items.filter((item) => item.id !== "truth-local"),
      })),
    };
    const removeCorrection = vi.spyOn(api, "removeCorrection").mockResolvedValue(updatedProfile);
    await renderRoute("#/profile");
    await screen.findByText("KnowU is local-first");

    fireEvent.click(screen.getByRole("button", { name: "Remove correction" }));

    await waitFor(() => expect(removeCorrection).toHaveBeenCalledWith("truth-local"));
    await waitFor(() => expect(screen.queryByText("KnowU is local-first")).not.toBeInTheDocument());
  });
});

describe("settings privacy disclosures", () => {
  beforeEach(stubApi);

  it("states that provider keys travel directly from the Mac", async () => {
    await renderRoute("#/settings");

    expect(await screen.findByText("Your key goes directly from this Mac to the selected provider.")).toBeInTheDocument();
  });

  it("states which metadata collection includes", async () => {
    await renderRoute("#/settings");

    expect(
      await screen.findByText(
        "Foreground app, window title, selected Chrome history, and editor file-save metadata.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/never opens saved code snapshots or source contents/i),
    ).toBeInTheDocument();
  });

  it("re-imports Chrome history and refreshes the profile as one operation", async () => {
    const reimport = vi.mocked(api.reimportChromeHistory);
    const refresh = vi.mocked(api.refreshProfile);
    await renderRoute("#/settings");

    fireEvent.click(await screen.findByRole("button", { name: "Re-import Chrome history" }));

    await waitFor(() => expect(reimport).toHaveBeenCalledOnce());
    expect(refresh).not.toHaveBeenCalled();
    expect(
      await screen.findByText("Chrome durations imported and your profile was refreshed."),
    ).toBeInTheDocument();
  });

  it("discloses the full scope of permanent deletion", async () => {
    await renderRoute("#/settings");

    expect(
      await screen.findByText(
        "Permanently removes local activity, profiles, corrections, recommendations, telemetry, settings, and provider credentials.",
      ),
    ).toBeInTheDocument();
  });
});

describe("assistant chat", () => {
  beforeEach(stubApi);

  it("sends the full visible conversation and renders the assistant response", async () => {
    const response: ChatMessage = {
      id: "assistant-response",
      role: "assistant",
      content: "Continue validating the macOS permission bridge.",
      createdAt: "2026-07-27T17:05:00.000Z",
    };
    const chat = vi.spyOn(api, "chat").mockResolvedValue(chatRun(response));
    await renderRoute("#/assistant");

    fireEvent.change(screen.getByPlaceholderText(/What should I prioritize/), {
      target: { value: "What should I work on?" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Send/ }));

    await waitFor(() => expect(chat).toHaveBeenCalledTimes(1));
    expect(chat.mock.calls[0][1]).toBe("optimized");
    const sentMessages = chat.mock.calls[0][0];
    expect(sentMessages.map(({ role, content }) => ({ role, content }))).toEqual([
      {
        role: "assistant",
        content: "Ask anything. I’ll retrieve only the approved memories relevant to this request.",
      },
      { role: "user", content: "What should I work on?" },
    ]);
    expect(await screen.findByText(response.content)).toBeInTheDocument();
    expect(screen.getByText("75.0%")).toBeInTheDocument();
    expect(screen.getAllByText("Privacy is more important than feature count.").length).toBeGreaterThan(0);
    expect(screen.getByText("synced-to-snowflake")).toBeInTheDocument();
  });

  it("uses one query-complete answer path while preserving the full-context comparison", async () => {
    const response: ChatMessage = {
      id: "baseline-response",
      role: "assistant",
      content: "Baseline answer.",
      createdAt: "2026-07-27T17:05:00.000Z",
    };
    const chat = vi.spyOn(api, "chat").mockResolvedValue(chatRun(response));
    await renderRoute("#/assistant");

    fireEvent.change(screen.getByPlaceholderText(/What should I prioritize/), {
      target: { value: "Compare this." },
    });
    fireEvent.click(screen.getByRole("button", { name: /Send/ }));

    await waitFor(() => expect(chat).toHaveBeenCalled());
    expect(chat.mock.calls[0][1]).toBe("optimized");
    expect(screen.queryByRole("button", { name: "Full Context" })).not.toBeInTheDocument();
    expect(await screen.findByText("Baseline answer.")).toBeInTheDocument();
    fireEvent.click(screen.getByText("Compare context payloads"));
    expect(screen.getByText(/QUERY-SPECIFIC LOCAL ACTIVITY FACTS/)).toBeInTheDocument();
  });

  it("passes the selected thread as an inspectable provisional context brief", async () => {
    const response: ChatMessage = {
      id: "context-response",
      role: "assistant",
      content: "I used the selected thread.",
      createdAt: "2026-07-27T17:05:00.000Z",
    };
    const chat = vi.spyOn(api, "chat").mockResolvedValue(chatRun(response));
    await renderRoute("#/dashboard");

    fireEvent.click(await screen.findByRole("link", { name: /Ask with context/ }));
    expect(await screen.findByText("Review high-level thread brief")).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText(/What should I prioritize/), { target: { value: "What next?" } });
    fireEvent.click(screen.getByRole("button", { name: /Send/ }));

    await waitFor(() => expect(chat).toHaveBeenCalledOnce());
    const sentMessages = chat.mock.calls[0][0];
    expect(sentMessages[sentMessages.length - 1]?.content).toBe("What next?");
    expect(chat.mock.calls[0][2]).toContain("Context brief: KnowU implementation");
    expect(chat.mock.calls[0][2]).toContain("Treat this as provisional behavioral context");
    expect(chat.mock.calls[0][2]).toContain("Duration is omitted here");
    expect(chat.mock.calls[0][2]).not.toContain("Recorded duration");
    expect(chat.mock.calls[0][2]).not.toContain("Tauri 2 — Security Capabilities");
    expect(chat.mock.calls[0][2]).not.toContain("https://");
  });

  it("sends a message when Enter is pressed", async () => {
    const response: ChatMessage = {
      id: "assistant-response",
      role: "assistant",
      content: "Sent from the keyboard.",
      createdAt: "2026-07-27T17:05:00.000Z",
    };
    const chat = vi.spyOn(api, "chat").mockResolvedValue(chatRun(response));
    await renderRoute("#/assistant");

    const composer = screen.getByPlaceholderText(/What should I prioritize/);
    fireEvent.change(composer, { target: { value: "Send this with Enter" } });
    fireEvent.keyDown(composer, { key: "Enter", code: "Enter" });

    await waitFor(() => expect(chat).toHaveBeenCalledTimes(1));
    const sentMessages = chat.mock.calls[0][0];
    expect(sentMessages[sentMessages.length - 1]?.content).toBe("Send this with Enter");
    expect(await screen.findByText(response.content)).toBeInTheDocument();
  });

  it("inserts a newline with Shift+Enter instead of sending", async () => {
    const chat = vi.spyOn(api, "chat");
    await renderRoute("#/assistant");

    const composer = screen.getByPlaceholderText(/What should I prioritize/);
    fireEvent.change(composer, { target: { value: "First line" } });
    fireEvent.keyDown(composer, { key: "Enter", code: "Enter", shiftKey: true });

    expect(chat).not.toHaveBeenCalled();
  });

  it("does not send whitespace-only messages", async () => {
    const chat = vi.spyOn(api, "chat");
    await renderRoute("#/assistant");

    fireEvent.change(screen.getByPlaceholderText(/What should I prioritize/), {
      target: { value: "   " },
    });

    expect(screen.getByRole("button", { name: /Send/ })).toBeDisabled();
    expect(chat).not.toHaveBeenCalled();
  });

  it("renders assistant Markdown as structured content", async () => {
    const response: ChatMessage = {
      id: "markdown-response",
      role: "assistant",
      content: [
        "Here are the **next steps**:",
        "",
        "## Immediate",
        "",
        "- Create a short TODO list",
        "- Update the README",
        "",
        "1. Run the app",
        "2. Check the tests",
      ].join("\n"),
      createdAt: "2026-07-27T17:05:00.000Z",
    };
    vi.spyOn(api, "chat").mockResolvedValue(chatRun(response));
    await renderRoute("#/assistant");

    fireEvent.change(screen.getByPlaceholderText(/What should I prioritize/), {
      target: { value: "What should I work on?" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Send/ }));

    expect(await screen.findByRole("heading", { name: "Immediate" })).toBeInTheDocument();
    expect(screen.getByText("next steps").tagName).toBe("STRONG");

    const lists = screen.getAllByRole("list");
    expect(lists).toHaveLength(2);
    expect(within(lists[0]).getAllByRole("listitem")).toHaveLength(2);
    expect(within(lists[1]).getAllByRole("listitem")).toHaveLength(2);
  });
});
