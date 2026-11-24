# StoryBox3 changelog

## Canonical release log

The project maintains a detailed running log at
[`../CHANGELOG.md`](../CHANGELOG.md). It is newer and more specific than the
summary below; use it for per-file change lists.

## Release history summary

| Entry | Date in log | Focus |
| --- | --- | --- |
| v3.1.0 | 2025-06-15 | Two-stage semaphore-throttled processing pipeline, `ImageCache`, `FolderWatcher`, EXIF module, five React hooks, structured AI prompts, pin/favorite, automation, paginated feed, mobile SAF scanning |
| v3.0.2 | 2025-06-14 | Architecture and mobile reliability: cache/watcher, startup DB count, filesystem capabilities, SAF-based scanning |
| v3.0.1 | 2025-06-01 | Scanned-folder management and `folder_child_overrides` subfolder toggles |
| v3.0.2 | 2025-05-25 | Prompt upgrades, UI polish, TypeScript/icon build fixes |
| v3.0.1 | 2025-05-25 | Android project initialization and documentation |
| v3.0.0 | 2025-05-24 | StoryBox3 rename/identity, OpenRouter cloud AI, Supabase proxy, cyber UI, health banner |
| v2.1.1 | — (collapsed entry) | iOS Photos plugin wiring, docs, `Info.plist` |
| v2.1.0 | — (collapsed entry) | Automation timer, vibe parsing fix, CSS utility fixes, dead-code removal |

Observations from the log itself:

- Version labels are not unique: there are two `v3.0.2` entries and two
  `v3.0.1` entries with different dates and contents.
- The current manifest and Tauri config both report `3.1.0`, matching the
  newest entry.
- Some historical claims in the log describe code that is no longer present.
  For example, v3.0.0 records identifier `com.storybox3.app`, while
  [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) now uses
  `com.ok.storybox3`; v3.0.0 mentions 15-second health polling, while
  [`../src/hooks/useAiHealth.ts`](../src/hooks/useAiHealth.ts) uses 30 seconds.
- The changelog is historical narrative, not a machine-verifiable test report.

## Sources

- [`../CHANGELOG.md`](../CHANGELOG.md)
- [`../package.json`](../package.json)
- [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
- [`../src/hooks/useAiHealth.ts`](../src/hooks/useAiHealth.ts)
