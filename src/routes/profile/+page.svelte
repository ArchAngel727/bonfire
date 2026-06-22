<script>
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { io } from "socket.io-client";

  let profile = null;
  let loading = true;
  let loadError = null;

  let oldPassword = "";
  let newPassword = "";
  let confirmPassword = "";
  let pwMessage = null; // { kind: "ok" | "error", text: string }
  let pwSubmitting = false;

  // Mod management — only used when profile.is_admin
  let modUsername = "";
  let foundUser = null; // { user_id, username, is_admin, is_mod }
  let modMessage = null;
  let modSubmitting = false;

  let adminSock;

  function readCookie() {
    if (typeof localStorage === "undefined") return null;
    const raw = localStorage.getItem("session");
    if (!raw) return null;
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }

  function roleLabel(p) {
    if (!p) return "";
    if (p.is_admin) return "Admin";
    if (p.is_mod) return "Moderator";
    return "User";
  }

  function loadProfile() {
    adminSock.emit("get_profile", {}, (res) => {
      if (res.status !== "ok") {
        loadError = res.reason ?? "failed to load profile";
        loading = false;
        return;
      }
      profile = res;
      loading = false;
    });
  }

  function handleChangePassword() {
    pwMessage = null;
    if (!oldPassword || !newPassword) {
      pwMessage = { kind: "error", text: "fill in both password fields" };
      return;
    }
    if (newPassword !== confirmPassword) {
      pwMessage = { kind: "error", text: "new passwords don't match" };
      return;
    }
    if (newPassword.length < 6) {
      pwMessage = { kind: "error", text: "new password must be at least 6 characters" };
      return;
    }

    pwSubmitting = true;
    adminSock.emit(
      "change_password",
      { old_password: oldPassword, new_password: newPassword },
      (res) => {
        pwSubmitting = false;
        if (res.status !== "ok") {
          pwMessage = { kind: "error", text: res.reason ?? "change failed" };
          return;
        }
        pwMessage = {
          kind: "ok",
          text: "password updated — logging you out for security",
        };
        oldPassword = "";
        newPassword = "";
        confirmPassword = "";
        // Backend killed all sessions for this user. Clear our cookie and
        // bounce to login after a beat so the message is readable.
        setTimeout(() => {
          localStorage.removeItem("session");
          adminSock?.disconnect();
          goto("/login");
        }, 1500);
      }
    );
  }

  function lookupModUser() {
    modMessage = null;
    const name = modUsername.trim();
    if (!name) {
      modMessage = { kind: "error", text: "enter a username" };
      return;
    }
    adminSock.emit("find_user", { username: name }, (res) => {
      if (res.status !== "ok") {
        foundUser = null;
        modMessage = { kind: "error", text: res.reason ?? "lookup failed" };
        return;
      }
      foundUser = res;
    });
  }

  function toggleMod() {
    if (!foundUser) return;
    if (foundUser.is_admin) {
      modMessage = { kind: "error", text: "cannot change admin's role" };
      return;
    }
    const target = !foundUser.is_mod;
    modSubmitting = true;
    adminSock.emit(
      "set_mod",
      { user_id: foundUser.user_id, is_mod: target },
      (res) => {
        modSubmitting = false;
        if (res.status !== "ok") {
          modMessage = { kind: "error", text: res.reason ?? "update failed" };
          return;
        }
        foundUser = { ...foundUser, is_mod: target };
        modMessage = {
          kind: "ok",
          text: target
            ? `${foundUser.username} is now a moderator`
            : `${foundUser.username} is now a regular user`,
        };
      }
    );
  }

  onMount(() => {
    const cookie = readCookie();
    if (!cookie) {
      goto("/login");
      return;
    }

    adminSock = io("http://localhost:3000/admin", { auth: cookie });

    adminSock.on("connect_error", (e) => {
      const msg = e.message ?? "";
      if (/AUTH_REQUIRED|SESSION_INVALID|SESSION_EXPIRED/.test(msg)) {
        localStorage.removeItem("session");
        goto("/login");
      } else {
        loadError = msg || "connection failed";
        loading = false;
      }
    });

    adminSock.on("connect", () => {
      loadProfile();
    });
  });

  onDestroy(() => {
    adminSock?.disconnect();
  });
</script>

<main class="profile-page">
  <header class="profile-header">
    <button class="back-btn" on:click={() => goto("/")}>← Back to chat</button>
    <h1>Profile</h1>
  </header>

  {#if loading}
    <p class="status">Loading…</p>
  {:else if loadError}
    <p class="status error">{loadError}</p>
  {:else if profile}
    <section class="card">
      <h2>Account</h2>
      <dl class="info">
        <dt>Username</dt>
        <dd>{profile.username}</dd>

        <dt>Role</dt>
        <dd>{roleLabel(profile)}</dd>

        <dt>User ID</dt>
        <dd class="mono">{profile.user_id}</dd>
      </dl>
    </section>

    <section class="card">
      <h2>Change password</h2>
      <div class="form">
        <label>
          Current password
          <input
            type="password"
            bind:value={oldPassword}
            autocomplete="current-password"
          />
        </label>
        <label>
          New password
          <input
            type="password"
            bind:value={newPassword}
            autocomplete="new-password"
          />
        </label>
        <label>
          Confirm new password
          <input
            type="password"
            bind:value={confirmPassword}
            autocomplete="new-password"
          />
        </label>
        <button class="primary" disabled={pwSubmitting} on:click={handleChangePassword}>
          {pwSubmitting ? "Updating…" : "Update password"}
        </button>
        {#if pwMessage}
          <p class="form-message {pwMessage.kind}">{pwMessage.text}</p>
        {/if}
      </div>
    </section>

    {#if profile.is_admin}
      <section class="card">
        <h2>Moderator management</h2>
        <div class="form">
          <label>
            Username
            <input
              type="text"
              bind:value={modUsername}
              placeholder="exact username"
              on:keydown={(e) => { if (e.key === "Enter") lookupModUser(); }}
            />
          </label>
          <button class="secondary" on:click={lookupModUser}>Look up user</button>

          {#if foundUser}
            <div class="found-user">
              <div><strong>{foundUser.username}</strong></div>
              <div class="found-user-role">
                Current role:
                {#if foundUser.is_admin}
                  Admin
                {:else if foundUser.is_mod}
                  Moderator
                {:else}
                  User
                {/if}
              </div>
              {#if !foundUser.is_admin}
                <button
                  class="primary"
                  disabled={modSubmitting}
                  on:click={toggleMod}
                >
                  {modSubmitting
                    ? "Working…"
                    : foundUser.is_mod
                    ? "Demote from Mod"
                    : "Promote to Mod"}
                </button>
              {:else}
                <p class="form-message error">Admin's role can't be changed here.</p>
              {/if}
            </div>
          {/if}

          {#if modMessage}
            <p class="form-message {modMessage.kind}">{modMessage.text}</p>
          {/if}
        </div>
      </section>
    {/if}
  {/if}
</main>

<style>
  .profile-page {
    max-width: 640px;
    margin: 0 auto;
    padding: 2rem 1.5rem;
    color: #eee;
  }

  .profile-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
  }
  .profile-header h1 {
    margin: 0;
    font-size: 1.6rem;
  }

  .back-btn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #ccc;
    padding: 0.4rem 0.8rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s, color 0.15s;
  }
  .back-btn:hover {
    background: rgba(255, 140, 50, 0.12);
    color: #ff8c32;
  }

  .card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    padding: 1.25rem 1.5rem;
    margin-bottom: 1.25rem;
  }
  .card h2 {
    margin: 0 0 1rem 0;
    font-size: 1.05rem;
    color: #ff8c32;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .info {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: 0.5rem 1rem;
    margin: 0;
  }
  .info dt {
    color: #888;
    font-size: 0.9rem;
  }
  .info dd {
    margin: 0;
    color: #eee;
  }
  .mono {
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 0.85rem;
    color: #bbb;
    word-break: break-all;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .form label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.9rem;
    color: #aaa;
  }
  .form input {
    padding: 0.5rem 0.7rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.3);
    color: #eee;
    font-size: 0.95rem;
  }
  .form input:focus {
    outline: none;
    border-color: #ff8c32;
  }

  .primary {
    margin-top: 0.5rem;
    padding: 0.6rem 1rem;
    background: #ff8c32;
    border: none;
    border-radius: 6px;
    color: #1a1a1a;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .primary:hover {
    opacity: 0.9;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .secondary {
    margin-top: 0.5rem;
    padding: 0.55rem 1rem;
    background: transparent;
    border: 1px solid rgba(255, 140, 50, 0.4);
    border-radius: 6px;
    color: #ff8c32;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }
  .secondary:hover {
    background: rgba(255, 140, 50, 0.1);
  }

  .found-user {
    margin-top: 1rem;
    padding: 0.85rem 1rem;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .found-user-role {
    color: #aaa;
    font-size: 0.9rem;
  }

  .form-message {
    margin: 0.5rem 0 0;
    font-size: 0.9rem;
  }
  .form-message.ok {
    color: #6fcf6f;
  }
  .form-message.error {
    color: #ff8888;
  }

  .status {
    color: #888;
    font-size: 0.95rem;
  }
  .status.error {
    color: #ff8888;
  }
</style>
