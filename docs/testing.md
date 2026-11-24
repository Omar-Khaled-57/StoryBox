# StoryBox3 testing

## JavaScript/TypeScript test tooling

[`../package.json`](../package.json) defines four scripts: `dev`, `build`,
`preview`, and `tauri`. There is **no `test` script** and no JavaScript test
runner or test configuration in the manifest. A repository search found no
`*.test.*` or `*.spec.*` files under `src/`. No frontend coverage report is
committed.

```bash
npm run build
```

This runs `tsc && vite build`: it type-checks `src/` using
[`../tsconfig.json`](../tsconfig.json) (`strict`, `noUnusedLocals`,
`noUnusedParameters`) and builds the frontend into `dist/`. It does not run
tests.

## Rust tests

[`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs) contains a `#[cfg(test)]`
module with nine `#[tokio::test]` functions:

| Test | What it asserts |
| --- | --- |
| `test_ai_engine_mock` | Mock analysis returns non-empty tags and vibe |
| `test_proxy_health_check` | Proxy health check reports available, mode `proxy`, provider `openrouter` |
| `test_proxy_caption_generation` | Proxy caption generation returns a non-empty string |
| `test_proxy_vision_analysis` | Writes a 1×1 JPEG to the temp dir, sends it through the proxy, expects tags and vibe |
| `test_proxy_unavailable_graceful` | An unreachable proxy yields `available: false` or an error, without panicking |
| `test_openrouter_no_credentials` | Empty key/proxy path returns a message mentioning proxy mode |
| `test_proxy_generate_openrouter_edge` | Empty prompt does not panic |
| `test_proxy_check_availability_parse_response` | Response contains `available`, `provider`, model fields, `message`; panics after retries if the proxy stays down |
| `test_mock_caption_generation` | Mock provider returns `None` for captions |

These are Cargo-discovered unit tests; no npm script wraps them. Several tests
(`test_proxy_health_check`, `test_proxy_caption_generation`,
`test_proxy_vision_analysis`, `test_proxy_check_availability_parse_response`)
depend on the hosted Supabase proxy and live OpenRouter access, so they are not
hermetic and can fail when the proxy is down or rate-limited. No coverage
report or CI result for them is tracked.

## Manual verification surfaces

The UI includes an **AI Health Check** section in
[`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx)
with a "Refresh Health Status" button, and
[`../src/hooks/useAiHealth.ts`](../src/hooks/useAiHealth.ts) polls availability
every 30 seconds. `test_ai_generation` is a registered Tauri command for an
end-to-end AI diagnostic. These are diagnostics, not automated test suites.

## Known gaps

- No frontend unit, component, or end-to-end tests.
- No tests for SQLite migrations, story generation fallbacks, cleanup cutoff,
  folder overrides, or the IPC command surface.
- Rust tests are network-dependent and lack mocking of the proxy.
- No CI workflow that runs type-checking or tests.
- [`../dev/complexity-report.md`](../dev/complexity-report.md) is a local
  complexity analysis, not a test report; `dev/` is ignored by
  [`../.gitignore`](../.gitignore).

## Sources

- [`../package.json`](../package.json)
- [`../tsconfig.json`](../tsconfig.json)
- [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs)
- [`../src/hooks/useAiHealth.ts`](../src/hooks/useAiHealth.ts)
- [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx)
- [`../dev/complexity-report.md`](../dev/complexity-report.md)
- [`../.gitignore`](../.gitignore)
