# Bonfire

Self-hosted chat server. End-to-end encryption for DMs, server-side
encryption at rest for group channels.

## Features

- User registration and login (argon2 password hashing)
- HMAC-signed session cookies
- 1:1 DMs and group text channels
- Roles: admin (env-bootstrapped), mod, user
- Live message delivery via socket.io
- Image attachments (client-side downscale, embedded as base64)
- Server-side AES-256-GCM encryption at rest for text channels
- End-to-end encryption for DMs via Olm/Double Ratchet (vodozemac)
- Moderation: message delete, user ban (terminates all sessions)
- Channel rename, channel delete
- Profile page with password change (invalidates all sessions)

## Architecture

### Stack
- Backend: Rust, socketioxide, sqlx, SQLite, axum, argon2, aes-gcm
- Frontend: SvelteKit, socket.io-client
- Shell: Tauri (enables native vodozemac on the client)

### Three cryptographic systems

**Session authentication — HMAC-SHA256.** On login the server generates a
random session_id, signs it with a server-side secret, and returns both as
a cookie. Every reconnect: server verifies the signature, looks up the
session, attaches the user_id to the socket. Without HMAC, session_ids
would be forgeable.

**Text channel encryption at rest — AES-256-GCM.** Server holds a 32-byte
key in `./data/message.key` (auto-generated on first boot). Each message
gets a fresh 96-bit nonce. Stored blob is `nonce || ciphertext || tag`.
Protects against an attacker with disk access; the server itself can read
text channels by design (acceptable because group membership is open and
client-side group encryption would require nontrivial key management).

**DM end-to-end encryption — Olm/Double Ratchet via vodozemac.** Each
client generates an Olm identity on first run and uploads a key bundle to
the server (long-term identity + signing key + signed prekey + 10 one-time
prekeys). Sender fetches the recipient's bundle, performs X3DH key
agreement, derives a shared secret, and uses Double Ratchet for subsequent
messages. Server stores opaque ciphertext it cannot decrypt. Sessions
persist in the Tauri data dir per DM channel.

### Data model

- `users` — credentials and mod flag
- `sessions` — active session cookies
- `channels` — DM or text, with participants stored inline for DMs
- `messages` — channel_id, author, monotonic seq, content as opaque bytes
- `banned_users` — banned_at, banned_by, optional reason
- `key_bundles` — public identity material per user
- `one_time_prekeys` — pool consumed atomically on bundle fetch

## Setup

### Prerequisites
- Rust 1.75+
- Node.js 20+
- `cargo install sqlx-cli --no-default-features --features sqlite`

The first registered user is promoted to admin automatically and their UUID
is appended to `.env` as `ADMIN_USER_ID`.

### Frontend
```bash
cd frontend
bun install
bun run tauri dev
```

## Threat model

### What is protected
- Session forgery (HMAC signing on cookies)
- Text channel content at rest (server-side AES)
- DM content end-to-end (server cannot decrypt)
- Wire transport (WSS/TLS)
- Password storage (argon2id)

### What is not
- The server can read text channels by design (key is on disk next to DB)
- One device per user (Olm identities don't sync across logins)
- No login rate limiting
- No key rotation
- Banned users with open sockets stay connected until next reconnect
  (sessions are deleted but live sockets aren't force-disconnected)

## Known limitations
- `seq` race in `channel_send` under high concurrent writes is caught by
  a UNIQUE constraint but not preempted with `BEGIN IMMEDIATE`
- Frontend hardcodes `localhost:3000` — production needs env-driven config
- Old plaintext DMs from before phase 3 deployment are unreadable (recommend
  DB reset before upgrade)
- No prekey replenishment — once a user runs out of one-time prekeys, new
  sessions fall back to using the signed prekey only

## Project scope

Semester project. Three phases shipped:
1. Plaintext foundation: auth, channels, messages, fan-out, roles
2. Server-side AES at rest for text channels
3. Prekey infrastructure + Olm/Double Ratchet E2E for DMs

Out of scope: multi-device sync, key rotation, federation, voice/video,
mobile clients, group session encryption for text channels, attachments
beyond images.

## Simple development setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
