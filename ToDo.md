# Talos — ToDo (Pre-Submission Fixes)

## 🔴 Backend — Must Fix

- [ ] **#1 — No AI backend spawns** (`core/src/lib.rs`)
  - `UserPreferences` has no `#[serde(default)]` on fields. Runner loads with `"{}"`, deserialization fails, falls back to `default()` where `backend = ""`. None of `"API"`, `"AGY"`, `"LOCAL"` match so no AI backend starts.
  - **Fix:** Add `#[serde(default)]` to each field in `UserPreferences`

- [ ] **#2 — Dead AGY PTY crashes Gemini path** (`ai/src/lib.rs:328`)
  - `agy_session = AgySession::new(...)` is created but never used (the lines that used it were deleted). It spawns a PTY running `agy` — crashes if `agy` isn't installed.
  - **Fix:** Delete line 328 (`let agy_session = AgySession::new(tx_in.clone())?;`)

- [ ] **#3 — Gemini hangs after tool call** (`ai/src/lib.rs:338-369`)
  - When a `ToolCallResult` arrives, the tool response is sent to Gemini, but `speech` is empty → hits `continue` at line 369 → loops back to `rx_out.recv()` without ever reading Gemini's response via `session.next_event()`.
  - **Fix:** After `session.send_tool_response()`, jump to the `while let Some(event) = session.next_event().await` loop instead of continuing

- [ ] **#4 — `save_chats` kills persistence on unknown events** (`ai/src/lib.rs:649-653`)
  - The `_ =>` catch-all returns `Err(...)`, stopping chat persistence if `RenderWidget`, `PluginData`, etc. arrive.
  - **Fix:** Change `_ => { return Err(...); }` to `_ => { continue; }`

- [ ] **#5 — STT model path missing `Talos/`** (`audio/src/lib.rs:~20`)
  - Path is `data_local_dir/models/moonshine-streaming-medium-onnx` — missing `Talos/` prefix.
  - **Fix:** Change to `.join("Talos").join("models").join("moonshine-streaming-medium-onnx")`

- [ ] **#6 — TTS chipmunk audio** (`audio/src/lib.rs`, tts function)
  - Synthesized audio at ~24kHz is played at the device's native rate (44.1/48kHz) with no resampling. Audio plays at ~2x speed with high pitch.
  - **Fix:** Add resampling (e.g. with Rubato) from model sample rate to device sample rate

- [ ] **#7 — Client exits on send error instead of reconnecting** (`runner/src/main.rs:251,273,276`)
  - `conn.send_to_server(...).await?` — any transient send error returns `Err` and exits the whole client process instead of breaking to the reconnection loop.
  - **Fix:** Change `?` to `if let Err(e) = ... { break; }` to trigger reconnection

- [ ] **#8 — WebSocket route blocked by auth middleware** (`ui/src/lib.rs:403,408`)
  - `/api/talosbus` is inside `private_router` behind `axum_auth` middleware. Browser WebSocket APIs can't send `Authorization` headers → always 401. The handler already checks token via query param.
  - **Fix:** Move `/api/talosbus` route to `public_router`

- [ ] **#9 — Dashboard plugin DB tables never created** (`ui/src/lib.rs:386-392`)
  - `server_dashboard` connects to `plugins.db` but never runs `CREATE TABLE IF NOT EXISTS`. Plugin config/permissions endpoints panic via `.expect()`.
  - **Fix:** Add table creation SQL after `db.connect()`

- [ ] **#10 — Background tasks query non-existent tables** (`ai/src/lib.rs:790-860`)
  - `manage_soul()` queries `profile` table, `self_improvement()` queries `chats` table. `save_chats()` is never started in runner, so neither table exists. Errors every 10 minutes.
  - **Fix:** Start `save_chats` in runner's server loop, or guard the queries with table existence checks

---

## 🔴 Frontend — Most Visible to Reviewers

- [ ] **#11 — Sign up flow broken** (`SignUp.tsx:231-249`)
  - After 2FA verify succeeds, the response token is never stored in `authStore`. User enters dashboard unauthenticated — all protected API calls fail with 401.
  - **Fix:** Parse response body, call `addAccount()` with token and username

- [ ] **#12 — WebSocket never connects after login** (`App.tsx:174-243`)
  - `useEffect` has empty dependency array `[]`. Runs once on mount when user isn't logged in, never re-runs after login. WebSocket only works after a full page reload.
  - **Fix:** Add `token` to the dependency array

- [ ] **#13 — Pages 1, 3, 4 are unclickable** (`App.tsx:413` + `Page1.tsx`, `Page3.tsx`, `Page4.tsx`)
  - Parent div has `pointer-events-none` but these pages don't add `pointer-events-auto`. Can't scroll, click, or select text.
  - **Fix:** Add `pointer-events-auto` class to those page root elements

- [ ] **#14 — Save settings crashes** (`Settings.tsx:230-240`)
  - Backend `update_server_config` returns empty body (`StatusCode::OK`), but frontend calls `response.json()` → throws `SyntaxError: Unexpected end of JSON input`.
  - **Fix:** Check `response.ok` instead of parsing JSON, or return JSON from the backend

- [ ] **#15 — User prefs never saved/loaded correctly** (`Settings.tsx:90-110`)
  - Settings page reads/writes everything to `/api/config` (ServerConfig only). User preferences like `backend`, `model`, `max_output_tokens` require `/api/user/prefs` which is never called.
  - **Fix:** Split config fetch/save to use both `/api/config` and `/api/user/prefs`

- [ ] **#16 — Build breaks on clean install** (`alert.tsx:3`)
  - Imports from `framer-motion` but the actual dependency is `motion`. Other files correctly import from `motion/react`.
  - **Fix:** Change import to `from "motion/react"`

---

## 🟡 Frontend — Less Critical But Noticeable

- [ ] **#17 — Plugin pages always fail** (`Page2.tsx:18,31`)
  - Fetches `/api/plugins` (list) and POSTs to `/api/v1/plugins/install` — neither route exists in backend. Missing auth header on install too.

- [ ] **#18 — Event feed broken over HTTPS** (`EventFeed.tsx:20`)
  - Uses `ws://` hardcoded — browser blocks mixed content when accessed over HTTPS Cloudflare tunnel.
  - **Fix:** Use `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}`

- [ ] **#19 — File type filter broken** (`Settings.tsx:286`)
  - `accept="image//*"` has a double slash. Should be `image/*`.

- [ ] **#20 — Peer client cards show local user data** (`Page4.tsx:36,42`)
  - Each connected client card renders `{username}` and `{email}` (local user) instead of the peer's actual metadata.

- [ ] **#21 — Resource Usage page shows splash** (`App.tsx:63-66`)
  - Sidebar item "Resource Usage" sets `activeIndex = 4` but `renderContent()` has no `case 4:` → falls through to `default:` → renders unauthenticated splash page inside dashboard.

- [ ] **#22 — Template literal renders as text** (`App.tsx:114`)
  - `Server Error ${response.status}` inside JSX renders as the literal string `"${response.status}"` instead of the actual value.

- [ ] **#23 — False auth warning on landing page** (`App.tsx:95-144`)
  - `fetchConfig` runs on mount with `[]` deps, fires "Authentication Required" warning before user has attempted to sign in.

- [ ] **#24 — Sign up form validation deadlock** (`SignUp.tsx`)
  - Submit button is disabled until validation passes, but validation only triggers on submit. Fields don't reset `isValid` to `false` when cleared.

---

## ⏳ Deferred

- [ ] **Installer download URLs** — will update when binaries are published
- [ ] **README.md** — write project description, build instructions, screenshots
- [ ] **LICENSE** — add MIT or Apache-2.0
- [ ] **Build/setup docs** — explain dependencies (ONNX models, Turso, WASM plugins, etc.)
