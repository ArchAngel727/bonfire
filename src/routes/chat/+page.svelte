<script>
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { io } from "socket.io-client";
  import { goto } from "$app/navigation";
  import {
    initCrypto,
    sendDM,
    decryptIncomingDM,
    peerOf,
    cachePlaintext,
    getPlaintext,
  } from "$lib/cryptoService";
  import { detachCrypto } from "$lib/crypto";

  let channelSock;
  let adminSock;
  let keysSock;

  // initCrypto needs BOTH the user_id (from my_role) and the keys socket
  // connected. Each side calls maybeInitCrypto when its prerequisite lands.
  let cryptoReady = false;
  async function maybeInitCrypto() {
    if (cryptoReady) return;
    if (!myUserId) return;
    if (!keysSock || !keysSock.connected) return;
    cryptoReady = true;
    try {
      await initCrypto(keysSock, myUserId);
    } catch (e) {
      console.error("crypto init failed:", e);
      cryptoReady = false;
      alert(`crypto init failed: ${e.message ?? e}`);
    }
  }

  let myUserId = null;
  let isAdmin = false;
  let isMod = false;

  let dms = [];
  let textChannels = [];
  let currentChannelId = null;
  let messagesByChannel = new Map(); // channel_id → ChannelMessage[]
  let unread = new Set();
  let messageText = "";

  // "connecting" | "connected" | "reconnecting" | "disconnected"
  let connectionState = "connecting";

  $: messages = messagesByChannel.get(currentChannelId) ?? [];

  // Reactive labels — Svelte 4 needs explicit reads of myUserId here so the
  // template re-runs when my_role lands after channel_list.
  $: dmsLabeled = dms.map((ch) => {
    let label = "??";
    if (ch.dm_user_low === myUserId) label = ch.dm_user_high_username ?? "??";
    else if (ch.dm_user_high === myUserId) label = ch.dm_user_low_username ?? "??";
    return { ...ch, _label: label };
  });

  $: currentLabel = (() => {
    if (!currentChannelId) return "Select a chat";
    const tc = textChannels.find((c) => c.channel_id === currentChannelId);
    if (tc) return `#${tc.name}`;
    const dm = dmsLabeled.find((c) => c.channel_id === currentChannelId);
    if (dm) return `@${dm._label}`;
    return "??";
  })();

  // attachment ui state
  let openAttachment = false;
  let files = [];
  let previews = [];
  let lightbox = null;
  let attachmentMenu;
  let fileInput;
  let messagesEl;

  // Auto-scroll to bottom whenever the visible message list changes.
  // Only when the user is already near the bottom — if they've scrolled up
  // to read history, don't yank them back down.
  let stickToBottom = true;
  function onMessagesScroll() {
    if (!messagesEl) return;
    const distFromBottom =
      messagesEl.scrollHeight - messagesEl.scrollTop - messagesEl.clientHeight;
    stickToBottom = distFromBottom < 50;
  }
  $: if (messages && messagesEl && stickToBottom) {
    tick().then(() => {
      messagesEl?.scrollTo({ top: messagesEl.scrollHeight });
    });
  }

  function decodeContent(bytes) {
    return new TextDecoder().decode(new Uint8Array(bytes));
  }

  // Returns either { kind: "image", dataUrl } or { kind: "text", text }.
  // IMG: prefix means the rest is base64 JPEG.
  function parseContent(bytes) {
    const decoded = decodeContent(bytes);
    if (decoded.startsWith("IMG:")) {
      return { kind: "image", dataUrl: "data:image/jpeg;base64," + decoded.slice(4) };
    }
    return { kind: "text", text: decoded };
  }

  // ─── DM helpers ─────────────────────────────────────────────────────

  function isDMChannel(channel_id) {
    return dms.some((c) => c.channel_id === channel_id);
  }

  function getDMPeer(channel_id) {
    const ch = dms.find((c) => c.channel_id === channel_id);
    if (!ch) return null;
    return peerOf(ch, myUserId);
  }

  // Wrap arbitrary text in a ChannelMessage's `content` (number[]) field —
  // used when we've decrypted a DM and want it to flow through the same
  // parseContent path as plaintext text-channel messages.
  function encodeText(text) {
    return Array.from(new TextEncoder().encode(text));
  }

  function appendMessage(msg) {
    const list = messagesByChannel.get(msg.channel_id) ?? [];
    if (list.some((m) => m.message_id === msg.message_id)) return; // dedupe
    messagesByChannel.set(msg.channel_id, [...list, msg]);
    messagesByChannel = messagesByChannel; // trigger Svelte reactivity
  }

  function setMessages(channel_id, msgs) {
    messagesByChannel.set(channel_id, msgs);
    messagesByChannel = messagesByChannel;
  }

  function markUnread(channel_id) {
    unread.add(channel_id);
    unread = unread;
  }

  function clearUnread(channel_id) {
    if (unread.delete(channel_id)) unread = unread;
  }

  function addChannel(ch) {
    if (ch.kind === "dm") {
      if (!dms.some((c) => c.channel_id === ch.channel_id)) {
        dms = [...dms, ch];
      }
    } else if (ch.kind === "text") {
      if (!textChannels.some((c) => c.channel_id === ch.channel_id)) {
        textChannels = [...textChannels, ch];
      }
    }
  }

  function removeChannel(channel_id) {
    textChannels = textChannels.filter((c) => c.channel_id !== channel_id);
    dms = dms.filter((c) => c.channel_id !== channel_id);
    messagesByChannel.delete(channel_id);
    messagesByChannel = messagesByChannel;
    clearUnread(channel_id);
    if (currentChannelId === channel_id) currentChannelId = null;
  }

  function removeMessage(channel_id, message_id) {
    const list = messagesByChannel.get(channel_id);
    if (!list) return;
    const filtered = list.filter((m) => m.message_id !== message_id);
    if (filtered.length === list.length) return; // not present
    messagesByChannel.set(channel_id, filtered);
    messagesByChannel = messagesByChannel;
  }

  // Admin-only: delete a text channel after confirm.
  function deleteChannel(ch) {
    if (!confirm(`Delete #${ch.name}? This removes all its messages.`)) return;
    channelSock.emit("channel_delete", { channel_id: ch.channel_id }, (res) => {
      if (res.status !== "ok") return alert(`delete failed: ${res.reason}`);
      removeChannel(ch.channel_id); // server broadcasts to others; remove locally too
    });
  }

  function renameChannel(ch) {
    const next = window.prompt(`Rename #${ch.name} to:`, ch.name);
    if (!next) return;
    const trimmed = next.trim();
    if (!trimmed || trimmed === ch.name) return;
    channelSock.emit(
      "channel_rename",
      { channel_id: ch.channel_id, name: trimmed },
      (res) => {
        if (res.status !== "ok") return alert(`rename failed: ${res.reason}`);
        applyRename(res.channel ?? { channel_id: ch.channel_id, name: trimmed });
      }
    );
  }

  function applyRename(updated) {
    textChannels = textChannels.map((c) =>
      c.channel_id === updated.channel_id ? { ...c, name: updated.name } : c
    );
  }

  // Can the current user delete this message?
  function canDeleteMessage(msg) {
    if (msg.author_id === myUserId) return true;
    // Text channels: admin/mod can delete anyone's. DMs: only the author.
    const isTextChannel = textChannels.some((c) => c.channel_id === msg.channel_id);
    return isTextChannel && (isAdmin || isMod);
  }

  function deleteMessage(msg) {
    if (!confirm("Delete this message?")) return;
    channelSock.emit(
      "channel_delete_message",
      { channel_id: msg.channel_id, message_id: msg.message_id },
      (res) => {
        if (res.status !== "ok") return alert(`delete failed: ${res.reason}`);
        removeMessage(msg.channel_id, msg.message_id);
      }
    );
  }

  // Can the current user ban this message's author? Admin and mod can ban
  // (in text channels only — DMs are private and not a moderation surface).
  function canBan(msg) {
    if (msg.author_id === myUserId) return false;
    if (!isAdmin && !isMod) return false;
    return textChannels.some((c) => c.channel_id === msg.channel_id);
  }

  function banUser(msg) {
    const reason = window.prompt(
      `Ban ${msg.author_username}? Optionally enter a reason:`,
      ""
    );
    if (reason === null) return; // cancel
    adminSock.emit(
      "ban_user",
      { user_id: msg.author_id, reason: reason || null },
      (res) => {
        if (res.status !== "ok") return alert(`ban failed: ${res.reason}`);
        alert(`${msg.author_username} has been banned.`);
      }
    );
  }

  function dmLabel(ch) {
    if (ch.dm_user_low === myUserId) return ch.dm_user_high_username ?? "??";
    if (ch.dm_user_high === myUserId) return ch.dm_user_low_username ?? "??";
    return "??";
  }

  function currentChannelLabel() {
    if (!currentChannelId) return "Select a chat";
    const tc = textChannels.find((c) => c.channel_id === currentChannelId);
    if (tc) return `#${tc.name}`;
    const dm = dms.find((c) => c.channel_id === currentChannelId);
    if (dm) return `@${dmLabel(dm)}`;
    return "??";
  }

  function shortTime(iso) {
    return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  async function openChannel(channel_id) {
    currentChannelId = channel_id;
    clearUnread(channel_id);
    stickToBottom = true;
    const isDM = isDMChannel(channel_id);

    channelSock.emit("channel_sync", { channel_id }, async (res) => {
      if (res.status !== "ok") return console.error(res.reason);
      if (currentChannelId !== channel_id) return;

      if (!isDM) {
        setMessages(channel_id, res.messages);
        return;
      }

      // DM history: check the plaintext cache first for every message
      // (own messages can't be decrypted because Olm is forward-secret;
      // incoming messages can't be decrypted twice because the Olm ratchet
      // advances on every decrypt). Only fall through to crypto_decrypt
      // for messages we've never successfully decrypted on this install.
      const decrypted = [];
      for (const msg of res.messages) {
        const cached = getPlaintext(myUserId, msg.message_id);
        if (cached !== null) {
          decrypted.push({ ...msg, content: encodeText(cached) });
          continue;
        }
        if (msg.author_id === myUserId) {
          decrypted.push({
            ...msg,
            content: encodeText("[sent from another device]"),
          });
          continue;
        }
        try {
          const plaintext = await decryptIncomingDM(keysSock, myUserId, msg);
          if (plaintext === null) continue;
          cachePlaintext(myUserId, msg.message_id, plaintext);
          decrypted.push({ ...msg, content: encodeText(plaintext) });
        } catch (e) {
          console.error(`decrypt ${msg.message_id} failed:`, e);
          decrypted.push({ ...msg, content: encodeText("[decryption failed]") });
        }
      }
      // If the user has navigated away while we were decrypting, drop.
      if (currentChannelId !== channel_id) return;
      setMessages(channel_id, decrypted);
    });
  }

  async function handleSend() {
    if (!currentChannelId) return;
    const text = messageText.trim();
    const queuedFiles = files;
    if (!text && queuedFiles.length === 0) return;

    // Clear input state immediately so the user can keep typing while images upload.
    messageText = "";
    files = [];
    previews.forEach((p) => URL.revokeObjectURL(p.url));
    previews = [];

    const isDM = isDMChannel(currentChannelId);
    const peerId = isDM ? getDMPeer(currentChannelId) : null;
    if (isDM && !peerId) {
      console.error("DM channel without resolvable peer");
      alert("could not resolve DM peer");
      return;
    }

    // Send each image as its own message, then the text if any.
    for (const file of queuedFiles) {
      try {
        const b64 = await downscaleImageToBase64(file);
        const payload = "IMG:" + b64;

        if (isDM) {
          try {
            const message = await sendDM(
              channelSock,
              keysSock,
              currentChannelId,
              peerId,
              payload
            );
            cachePlaintext(myUserId, message.message_id, payload);
            // Server stores/broadcasts the ciphertext; we render the local
            // plaintext by swapping content before appending.
            appendMessage({ ...message, content: encodeText(payload) });
          } catch (e) {
            console.error("DM image send failed:", e);
            alert(`image send failed: ${e.message}`);
          }
        } else {
          const content = Array.from(new TextEncoder().encode(payload));
          await new Promise((resolve) => {
            channelSock.emit(
              "channel_send",
              { channel_id: currentChannelId, content },
              (res) => {
                if (res.status !== "ok") {
                  console.error(res.reason);
                  alert(`image send failed: ${res.reason}`);
                } else {
                  appendMessage(res.message);
                }
                resolve();
              }
            );
          });
        }
      } catch (e) {
        console.error("image processing failed:", e);
        alert(`image send failed: ${e.message}`);
      }
    }

    if (text) {
      if (isDM) {
        try {
          const message = await sendDM(
            channelSock,
            keysSock,
            currentChannelId,
            peerId,
            text
          );
          cachePlaintext(myUserId, message.message_id, text);
          appendMessage({ ...message, content: encodeText(text) });
        } catch (e) {
          console.error("DM send failed:", e);
          alert(`send failed: ${e.message}`);
        }
      } else {
        const content = Array.from(new TextEncoder().encode(text));
        channelSock.emit("channel_send", { channel_id: currentChannelId, content }, (res) => {
          if (res.status !== "ok") return console.error(res.reason);
          appendMessage(res.message);
        });
      }
    }
  }

  function newDm() {
    const username = window.prompt("Username to DM:");
    if (!username) return;
    channelSock.emit("lookup_user", { username: username.trim() }, (lookup) => {
      if (lookup.status !== "ok") return alert(`user not found: ${lookup.reason}`);
      channelSock.emit("channel_create", { kind: "dm", other: lookup.user_id }, (res) => {
        if (res.status !== "ok") return alert(`create DM failed: ${res.reason}`);
        addChannel(res.channel);
        openChannel(res.channel.channel_id);
      });
    });
  }

  function newTextChannel() {
    const name = window.prompt("Channel name:");
    if (!name) return;
    channelSock.emit("channel_create", { kind: "text", name: name.trim() }, (res) => {
      if (res.status !== "ok") return alert(`create failed: ${res.reason}`);
      addChannel(res.channel);
      openChannel(res.channel.channel_id);
    });
  }

  async function logout() {
    cryptoReady = false;
    try {
      await detachCrypto();
    } catch (e) {
      console.error("crypto detach failed:", e);
      // proceed with logout anyway
    }
    localStorage.removeItem("session");
    channelSock?.disconnect();
    adminSock?.disconnect();
    keysSock?.disconnect();
    channelSock = null;
    adminSock = null;
    keysSock = null;
    myUserId = null;
    isAdmin = false;
    isMod = false;
    dms = [];
    textChannels = [];
    currentChannelId = null;
    messagesByChannel = new Map();
    unread = new Set();
    messageText = "";
    goto("/login");
  }

  function openLightbox(p) { lightbox = p; }
  function closeLightbox() { lightbox = null; }
  function toggleAttachmentMenu() { openAttachment = !openAttachment; }
  function closeAttachmentMenu() { openAttachment = false; }
  function openFilePicker(accept) {
    closeAttachmentMenu();
    fileInput.accept = accept;
    fileInput.click();
  }
  function handleFileChange(event) {
    const MAX_BYTES = 5 * 1024 * 1024;
    const selected = Array.from(event.target.files);
    for (const file of selected) {
      if (!file.type.startsWith("image/")) {
        alert(`${file.name}: only images are supported`);
        continue;
      }
      if (file.size > MAX_BYTES) {
        alert(`${file.name}: image too large (max 5 MB)`);
        continue;
      }
      const url = URL.createObjectURL(file);
      previews = [...previews, { name: file.name, size: file.size, type: file.type, url }];
      files = [...files, file];
    }
    event.target.value = ""; // allow picking the same file again
  }

  // Downscale a File/Blob to JPEG, max 800px on the long edge, quality 0.85.
  // Returns a base64 string (no data: prefix).
  async function downscaleImageToBase64(file) {
    const MAX_DIM = 800;
    const QUALITY = 0.85;

    const img = await new Promise((resolve, reject) => {
      const el = new Image();
      el.onload = () => resolve(el);
      el.onerror = () => reject(new Error("failed to decode image"));
      el.src = URL.createObjectURL(file);
    });

    let { width, height } = img;
    if (width > MAX_DIM || height > MAX_DIM) {
      if (width >= height) {
        height = Math.round((height * MAX_DIM) / width);
        width = MAX_DIM;
      } else {
        width = Math.round((width * MAX_DIM) / height);
        height = MAX_DIM;
      }
    }

    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    ctx.drawImage(img, 0, 0, width, height);

    const blob = await new Promise((resolve) =>
      canvas.toBlob(resolve, "image/jpeg", QUALITY)
    );
    URL.revokeObjectURL(img.src);

    return await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        // reader.result is "data:image/jpeg;base64,<...>"
        const comma = reader.result.indexOf(",");
        resolve(reader.result.slice(comma + 1));
      };
      reader.onerror = () => reject(new Error("base64 encode failed"));
      reader.readAsDataURL(blob);
    });
  }
  function removePreview(index) {
    URL.revokeObjectURL(previews[index].url);
    previews = previews.filter((_, i) => i !== index);
    files = files.filter((_, i) => i !== index);
  }
  function getIcon(type) {
    if (type.includes("word") || type.includes("document")) return "📝";
    if (type.includes("excel") || type.includes("sheet")) return "📊";
    if (type.includes("zip") || type.includes("compressed")) return "🗜️";
    if (type.includes("pdf")) return "📕";
    return "📄";
  }
  function formatSize(bytes) {
    if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
    return (bytes / 1024).toFixed(1) + " KB";
  }
  function handleClick(event) {
    const target = event.target;
    if (openAttachment && attachmentMenu && target instanceof Node && !attachmentMenu.contains(target)) {
      closeAttachmentMenu();
    }
  }

  onMount(() => {
    const saved = localStorage.getItem("session");
    if (!saved) { goto("/login"); return; }
    let cookie;
    try { cookie = JSON.parse(saved); }
    catch { localStorage.removeItem("session"); goto("/login"); return; }

    channelSock = io("http://localhost:3000/channel", { auth: cookie });
    adminSock = io("http://localhost:3000/admin", { auth: cookie });
    keysSock = io("http://localhost:3000/keys", { auth: cookie });

    channelSock.on("connect_error", (e) => {
      const msg = e.message ?? "";
      console.error("channel connect failed:", msg);
      if (/AUTH_REQUIRED|SESSION_INVALID|SESSION_EXPIRED/.test(msg)) {
        localStorage.removeItem("session");
        goto("/login");
      } else {
        connectionState = "reconnecting";
      }
    });

    channelSock.on("disconnect", (reason) => {
      // "io client disconnect" means we called .disconnect() ourselves (logout) — don't show banner.
      if (reason !== "io client disconnect") connectionState = "reconnecting";
    });

    channelSock.io.on("reconnect_attempt", () => {
      connectionState = "reconnecting";
    });

    adminSock.on("connect_error", (e) => console.error("admin connect failed:", e.message));
    keysSock.on("connect_error", (e) => console.error("keys connect failed:", e.message));

    channelSock.on("connect", () => {
      connectionState = "connected";
      channelSock.emit("channel_list", {}, (res) => {
        if (res.status !== "ok") return console.error(res.reason);
        dms = res.channels.filter((c) => c.kind === "dm");
        textChannels = res.channels.filter((c) => c.kind === "text");
      });
    });

    adminSock.on("connect", () => {
      adminSock.emit("my_role", {}, async (res) => {
        if (res.status !== "ok") return console.error(res.reason);
        myUserId = res.user_id;
        isAdmin = res.is_admin;
        isMod = res.is_mod;
        await maybeInitCrypto();
      });
    });

    keysSock.on("connect", async () => {
      await maybeInitCrypto();
    });

    channelSock.on("channel_message", async (msg) => {
      const isDM = isDMChannel(msg.channel_id);
      if (isDM) {
        try {
          const plaintext = await decryptIncomingDM(keysSock, myUserId, msg);
          if (plaintext === null) return; // own echo — already appended via sendDM
          cachePlaintext(myUserId, msg.message_id, plaintext);
          appendMessage({ ...msg, content: encodeText(plaintext) });
        } catch (e) {
          console.error(`decrypt incoming ${msg.message_id} failed:`, e);
          appendMessage({ ...msg, content: encodeText("[decryption failed]") });
        }
      } else {
        appendMessage(msg);
      }
      if (msg.channel_id !== currentChannelId) {
        markUnread(msg.channel_id);
      }
    });

    channelSock.on("channel_created", (ch) => {
      addChannel(ch);
    });

    channelSock.on("channel_deleted", ({ channel_id }) => {
      removeChannel(channel_id);
    });

    channelSock.on("channel_renamed", (ch) => {
      applyRename(ch);
    });

    channelSock.on("channel_message_deleted", ({ channel_id, message_id }) => {
      removeMessage(channel_id, message_id);
    });

    document.addEventListener("click", handleClick);

    return () => {
      channelSock?.disconnect();
      adminSock?.disconnect();
      keysSock?.disconnect();
      document.removeEventListener("click", handleClick);
    };
  });
</script>

<input bind:this={fileInput} type="file" multiple hidden on:change={handleFileChange} />

<main class="site-main chat-page">
  {#if connectionState === "reconnecting"}
    <div class="connection-banner">Reconnecting…</div>
  {/if}
  <div class:attachment={openAttachment} class="chat-layout">
    <aside class="friends">
      <div class="sidebar-header">
        <h2>Text</h2>
        {#if isAdmin}
          <button class="add-btn" on:click={newTextChannel} title="New text channel">+</button>
        {/if}
      </div>
      {#each textChannels as ch (ch.channel_id)}
        <div class="channel-row">
          <button
            class="dm-item"
            class:active={currentChannelId === ch.channel_id}
            class:has-unread={unread.has(ch.channel_id)}
            on:click={() => openChannel(ch.channel_id)}
          >
            #{ch.name}
            {#if unread.has(ch.channel_id)}<span class="unread-dot">●</span>{/if}
          </button>
          {#if isAdmin}
            <button
              class="channel-edit-btn"
              title="Rename channel"
              on:click|stopPropagation={() => renameChannel(ch)}
            >✎</button>
            <button
              class="channel-delete-btn"
              title="Delete channel"
              on:click|stopPropagation={() => deleteChannel(ch)}
            >×</button>
          {/if}
        </div>
      {/each}

      <div class="sidebar-header">
        <h2>Freunde</h2>
        <button class="add-btn" on:click={newDm} title="New DM">+</button>
      </div>
      {#each dmsLabeled as ch (ch.channel_id)}
        <button
          class="dm-item"
          class:active={currentChannelId === ch.channel_id}
          class:has-unread={unread.has(ch.channel_id)}
          on:click={() => openChannel(ch.channel_id)}
        >
          @{ch._label}
          {#if unread.has(ch.channel_id)}<span class="unread-dot">●</span>{/if}
        </button>
      {/each}

      <button class="logout-btn" on:click={() => goto("/profile")}>Profile</button>
      <button class="logout-btn" on:click={logout}>Log out</button>
    </aside>

    <section class="chat-history">
      <h2>{currentLabel}</h2>
      <div class="messages" bind:this={messagesEl} on:scroll={onMessagesScroll}>
        {#if !currentChannelId}
          <div class="empty-state">
            <p class="empty-state-title">No chat selected</p>
            <p class="empty-state-hint">
              Pick a conversation from the sidebar, or click + to start a new one.
            </p>
          </div>
        {:else}
          {#each messages as msg (msg.message_id)}
          {@const parsed = parseContent(msg.content)}
          <div class="message" class:mine={msg.author_id === myUserId}>
            <div class="message-meta">
              <span class="message-author">
                {msg.author_id === myUserId ? "Me" : msg.author_username}
              </span>
              <span class="message-time">{shortTime(msg.created_at)}</span>
              {#if canDeleteMessage(msg)}
                <button
                  class="message-delete-btn"
                  title="Delete message"
                  on:click={() => deleteMessage(msg)}
                >×</button>
              {/if}
              {#if canBan(msg)}
                <button
                  class="message-ban-btn"
                  title="Ban user"
                  on:click={() => banUser(msg)}
                >⊘</button>
              {/if}
            </div>
            {#if parsed.kind === "image"}
              <img
                class="message-image"
                src={parsed.dataUrl}
                alt="attachment"
                on:click={() => openLightbox({ url: parsed.dataUrl, name: "attachment" })}
              />
            {:else}
              <div class="message-body">{parsed.text}</div>
            {/if}
          </div>
        {/each}
        {/if}
      </div>
    </section>

    <div class="preview-bar">
      {#each previews as preview, i (preview.url)}
        <div
          animate:flip={{ duration: 300 }}
          in:fly={{ y: 30, duration: 300 }}
          out:fly={{ y: -30, duration: 200 }}
        >
          {#if preview.type.startsWith("image/")}
            <div class="preview-thumb">
              <img src={preview.url} alt={preview.name} on:click={() => openLightbox(preview)} />
              <button class="remove-btn" on:click={() => removePreview(i)}>✕</button>
            </div>
          {:else}
            <div class="file-chip">
              <span class="file-chip-icon">{getIcon(preview.type)}</span>
              <span class="file-chip-meta">
                <span class="file-chip-name">{preview.name}</span>
                <span class="file-chip-size">{formatSize(preview.size)}</span>
              </span>
              <button class="file-chip-remove" on:click={() => removePreview(i)}>✕</button>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <div class="chat-input">
      <input
        class="chat-input-field"
        type="text"
        placeholder={currentChannelId ? "Type your message..." : "Select a chat first"}
        id="message-input"
        bind:value={messageText}
        on:keydown={(e) => { if (e.key === "Enter") handleSend(); }}
        disabled={!currentChannelId}
      />
    </div>

    <div class="chat-buttons">
      <button
        class="button-chat"
        on:click={handleSend}
        disabled={!currentChannelId || (!messageText.trim() && files.length === 0)}
      >
        Send
      </button>
      <div class="attachment-menu-wrapper" bind:this={attachmentMenu}>
        {#if openAttachment}
          <div class="attachment-panel" transition:fly={{ y: 20, duration: 350 }}>
            <button class="attachment-option" on:click={() => openFilePicker("image/*")}>Image</button>
          </div>
        {/if}
        <button class="button-chat" on:click|stopPropagation={toggleAttachmentMenu}>+</button>
      </div>
    </div>
  </div>

  {#if lightbox}
    <div class="lightbox-overlay" on:click={closeLightbox} transition:fly={{ duration: 150 }}>
      <img class="lightbox-img" src={lightbox.url} alt={lightbox.name} on:click|stopPropagation />
      <button class="lightbox-close" on:click={closeLightbox}>✕</button>
    </div>
  {/if}
</main>

<style>
  /* Channel row contains the channel button + an optional admin delete button. */
  .channel-row {
    display: flex;
    align-items: center;
  }
  .channel-row .dm-item {
    flex: 1 1 auto;
    min-width: 0;
  }
  .channel-delete-btn {
    background: transparent;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 1rem;
    padding: 0 0.5rem;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .channel-row:hover .channel-delete-btn {
    opacity: 1;
  }
  .channel-delete-btn:hover {
    color: #ff8888;
  }

  .channel-edit-btn {
    background: transparent;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 0.3rem;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .channel-row:hover .channel-edit-btn {
    opacity: 1;
  }
  .channel-edit-btn:hover {
    color: #ff8c32;
  }

  .message-delete-btn {
    background: transparent;
    border: none;
    color: #666;
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
    padding: 0 0.3rem;
    margin-left: 0.4rem;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .message:hover .message-delete-btn {
    opacity: 1;
  }
  .message-delete-btn:hover {
    color: #ff8888;
  }

  .message-ban-btn {
    background: transparent;
    border: none;
    color: #666;
    cursor: pointer;
    font-size: 0.85rem;
    line-height: 1;
    padding: 0 0.3rem;
    margin-left: 0.2rem;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .message:hover .message-ban-btn {
    opacity: 1;
  }
  .message-ban-btn:hover {
    color: #ff6666;
  }

  .message-image {
    max-width: 360px;
    max-height: 360px;
    border-radius: 6px;
    cursor: zoom-in;
    display: block;
    margin-top: 0.3rem;
  }
  /* ─── Sidebar — section headers and add buttons (new elements) ─── */
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 0.5rem 0.25rem;
    margin-top: 0.5rem;
  }
  .sidebar-header h2 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #888;
  }

  .add-btn {
    background: transparent;
    border: none;
    color: #aaa;
    cursor: pointer;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    transition: background 0.15s, color 0.15s;
  }
  .add-btn:hover {
    background: rgba(255, 140, 50, 0.15);
    color: #ff8c32;
  }

  /* ─── Channel/DM items — additive on top of existing .dm-item ─── */
  .dm-item.active {
    background: rgba(255, 140, 50, 0.18);
    color: #ff8c32;
  }
  .dm-item.has-unread:not(.active) {
    color: #fff;
    font-weight: 600;
  }
  .unread-dot {
    color: #ff8c32;
    margin-left: 0.4rem;
    font-size: 0.65rem;
  }

  /* ─── Logout button (new element) ─── */
  .logout-btn {
    margin: auto 0.5rem 0.5rem;
    padding: 0.5rem;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #888;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .logout-btn:hover {
    background: rgba(255, 80, 80, 0.1);
    border-color: rgba(255, 80, 80, 0.3);
    color: #ff8888;
  }

  /* ─── Messages — internal scroll, visual tint only, no layout fights ─── */
  .messages {
    max-height: calc(100vh - 320px);
    overflow-y: auto;
  }

  /* Bubble shape — explicit flex column so meta + body are guaranteed
     inside the bubble, regardless of what global CSS does to children. */
  .message {
    display: flex !important;
    flex-direction: column !important;
    width: fit-content !important;
    max-width: 70% !important;
    margin: 0.25rem 0.75rem !important;
    padding: 0.5rem 0.8rem !important;
    border-radius: 10px !important;
    background: rgba(255, 255, 255, 0.04) !important;
  }
  .message.mine {
    background: rgba(255, 140, 50, 0.15) !important;
    align-self: flex-end !important;
    margin-left: auto !important;
  }
  .message:not(.mine) {
    align-self: flex-start !important;
    margin-right: auto !important;
  }

  /* The container needs flex column for align-self to work on bubbles. */
  .messages {
    display: flex !important;
    flex-direction: column !important;
    gap: 0.1rem;
  }

  .message-author {
    color: #ff8c32;
    font-weight: 600;
  }
  .message-time {
    color: #777;
    font-size: 0.75rem;
    margin-left: 0.5rem;
  }

  /* ─── Empty state (new element) ─── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 240px;
    text-align: center;
    color: #888;
    padding: 2rem;
  }
  .empty-state::before {
    content: "🔥";
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.4;
  }
  .empty-state-title {
    font-size: 1.1rem;
    margin: 0 0 0.5rem 0;
    color: #aaa;
    font-weight: 600;
  }
  .empty-state-hint {
    margin: 0;
    font-size: 0.9rem;
    max-width: 28rem;
  }

  /* ─── Reconnect banner (new element) ─── */
  .connection-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    padding: 0.4rem;
    text-align: center;
    background: #c97a2a;
    color: white;
    font-size: 0.9rem;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }
  .connection-banner::before {
    content: "⟳ ";
    display: inline-block;
    animation: spin 1.5s linear infinite;
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* ─── Disabled states (visual hint only, no layout change) ─── */
  .chat-input-field:disabled,
  .button-chat:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
