<script>
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { io } from "socket.io-client";
  import { goto } from "$app/navigation";

  let channelSock;
  let adminSock;

  let myUserId = null;
  let isAdmin = false;
  let isMod = false;

  let dms = [];
  let textChannels = [];
  let currentChannelId = null;
  let messagesByChannel = new Map(); // channel_id → ChannelMessage[]
  let unread = new Set();
  let messageText = "";

  $: messages = messagesByChannel.get(currentChannelId) ?? [];

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
    return new Date(iso).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function openChannel(channel_id) {
    currentChannelId = channel_id;
    clearUnread(channel_id);
    stickToBottom = true;
    channelSock.emit("channel_sync", { channel_id }, (res) => {
      if (res.status !== "ok") return console.error(res.reason);
      if (currentChannelId !== channel_id) return;
      setMessages(channel_id, res.messages);
    });
  }

  function handleSend() {
    const text = messageText.trim();
    if (!text || !currentChannelId) return;
    const content = Array.from(new TextEncoder().encode(text));
    messageText = "";
    channelSock.emit(
      "channel_send",
      { channel_id: currentChannelId, content },
      (res) => {
        if (res.status !== "ok") return console.error(res.reason);
        appendMessage(res.message);
      },
    );
  }

  function newDm() {
    const username = window.prompt("Username to DM:");
    if (!username) return;
    channelSock.emit("lookup_user", { username: username.trim() }, (lookup) => {
      if (lookup.status !== "ok")
        return alert(`user not found: ${lookup.reason}`);
      channelSock.emit(
        "channel_create",
        { kind: "dm", other: lookup.user_id },
        (res) => {
          if (res.status !== "ok")
            return alert(`create DM failed: ${res.reason}`);
          openChannel(res.channel.channel_id);
        },
      );
    });
  }

  function newTextChannel() {
    const name = window.prompt("Channel name:");
    if (!name) return;
    channelSock.emit(
      "channel_create",
      { kind: "text", name: name.trim() },
      (res) => {
        if (res.status !== "ok") return alert(`create failed: ${res.reason}`);
        openChannel(res.channel.channel_id);
      },
    );
  }

  function logout() {
    localStorage.removeItem("session");
    channelSock?.disconnect();
    adminSock?.disconnect();
    goto("/login");
  }

  function openLightbox(p) {
    lightbox = p;
  }
  function closeLightbox() {
    lightbox = null;
  }
  function toggleAttachmentMenu() {
    openAttachment = !openAttachment;
  }
  function closeAttachmentMenu() {
    openAttachment = false;
  }
  function openFilePicker(accept) {
    closeAttachmentMenu();
    fileInput.accept = accept;
    fileInput.click();
  }
  function handleFileChange(event) {
    const selected = Array.from(event.target.files);
    selected.forEach((file) => {
      const url = URL.createObjectURL(file);
      previews = [
        ...previews,
        { name: file.name, size: file.size, type: file.type, url },
      ];
      files = [...files, file];
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
    if (
      openAttachment &&
      attachmentMenu &&
      target instanceof Node &&
      !attachmentMenu.contains(target)
    ) {
      closeAttachmentMenu();
    }
  }

  onMount(() => {
    const saved = localStorage.getItem("session");
    if (!saved) {
      goto("/login");
      return;
    }
    let cookie;
    try {
      cookie = JSON.parse(saved);
    } catch {
      localStorage.removeItem("session");
      goto("/login");
      return;
    }

    window.dms = () => dms;
    window.me = () => myUserId;

    channelSock = io("http://localhost:3000/channel", { auth: cookie });
    adminSock = io("http://localhost:3000/admin", { auth: cookie });

    channelSock.on("connect_error", (e) => {
      const msg = e.message ?? "";
      console.error("channel connect failed:", msg);
      if (/AUTH_REQUIRED|SESSION_INVALID|SESSION_EXPIRED/.test(msg)) {
        localStorage.removeItem("session");
        goto("/login");
      }
    });

    adminSock.on("connect_error", (e) =>
      console.error("admin connect failed:", e.message),
    );

    channelSock.on("connect", () => {
      channelSock.emit("channel_list", {}, (res) => {
        if (res.status !== "ok") return console.error(res.reason);
        dms = res.channels.filter((c) => c.kind === "dm");
        textChannels = res.channels.filter((c) => c.kind === "text");
      });
    });

    adminSock.on("connect", () => {
      adminSock.emit("my_role", {}, (res) => {
        if (res.status !== "ok") return console.error(res.reason);
        myUserId = res.user_id;
        isAdmin = res.is_admin;
        isMod = res.is_mod;
      });
    });

    channelSock.on("channel_message", (msg) => {
      appendMessage(msg);
      if (msg.channel_id !== currentChannelId) {
        markUnread(msg.channel_id);
      }
    });

    channelSock.on("channel_created", (ch) => {
      if (ch.kind === "dm") dms = [...dms, ch];
      else if (ch.kind === "text") textChannels = [...textChannels, ch];
    });

    document.addEventListener("click", handleClick);

    return () => {
      channelSock?.disconnect();
      adminSock?.disconnect();
      document.removeEventListener("click", handleClick);
    };
  });
</script>

<input
  bind:this={fileInput}
  type="file"
  multiple
  hidden
  on:change={handleFileChange}
/>

<main class="site-main chat-page">
  <div class:attachment={openAttachment} class="chat-layout">
    <aside class="friends">
      <div class="sidebar-header">
        <h2>Text</h2>
        {#if isAdmin}
          <button
            class="add-btn"
            on:click={newTextChannel}
            title="New text channel">+</button
          >
        {/if}
      </div>
      {#each textChannels as ch (ch.channel_id)}
        <button
          class="dm-item"
          class:active={currentChannelId === ch.channel_id}
          class:has-unread={unread.has(ch.channel_id)}
          on:click={() => openChannel(ch.channel_id)}
        >
          #{ch.name}
          {#if unread.has(ch.channel_id)}<span class="unread-dot">●</span>{/if}
        </button>
      {/each}

      <div class="sidebar-header">
        <h2>Freunde</h2>
        <button class="add-btn" on:click={newDm} title="New DM">+</button>
      </div>
      {#each dms as ch (ch.channel_id)}
        <button
          class="dm-item"
          class:active={currentChannelId === ch.channel_id}
          class:has-unread={unread.has(ch.channel_id)}
          on:click={() => openChannel(ch.channel_id)}
        >
          @{(myUserId, dmLabel(ch))}
          {#if unread.has(ch.channel_id)}<span class="unread-dot">●</span>{/if}
        </button>
      {/each}
      <button class="logout-btn" on:click={logout}>Log out</button>
    </aside>

    <section class="chat-history">
      <h2>{currentChannelLabel()}</h2>
      <div class="messages" bind:this={messagesEl} on:scroll={onMessagesScroll}>
        {#each messages as msg (msg.message_id)}
          <div class="message" class:mine={msg.author_id === myUserId}>
            <div class="message-meta">
              <span class="message-author">
                {msg.author_id === myUserId ? "Me" : msg.author_username}
              </span>
              <span class="message-time">{shortTime(msg.created_at)}</span>
            </div>
            <div class="message-body">{decodeContent(msg.content)}</div>
          </div>
        {/each}
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
              <img
                src={preview.url}
                alt={preview.name}
                on:click={() => openLightbox(preview)}
              />
              <button class="remove-btn" on:click={() => removePreview(i)}
                >✕</button
              >
            </div>
          {:else}
            <div class="file-chip">
              <span class="file-chip-icon">{getIcon(preview.type)}</span>
              <span class="file-chip-meta">
                <span class="file-chip-name">{preview.name}</span>
                <span class="file-chip-size">{formatSize(preview.size)}</span>
              </span>
              <button class="file-chip-remove" on:click={() => removePreview(i)}
                >✕</button
              >
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <div class="chat-input">
      <input
        class="chat-input-field"
        type="text"
        placeholder={currentChannelId
          ? "Type your message..."
          : "Select a chat first"}
        id="message-input"
        bind:value={messageText}
        on:keydown={(e) => {
          if (e.key === "Enter") handleSend();
        }}
        disabled={!currentChannelId}
      />
    </div>

    <div class="chat-buttons">
      <button
        class="button-chat"
        on:click={handleSend}
        disabled={!currentChannelId || !messageText.trim()}
      >
        Send
      </button>
      <div class="attachment-menu-wrapper" bind:this={attachmentMenu}>
        {#if openAttachment}
          <div
            class="attachment-panel"
            transition:fly={{ y: 20, duration: 350 }}
          >
            <button
              class="attachment-option"
              on:click={() => openFilePicker("*")}>File</button
            >
          </div>
        {/if}
        <button
          class="button-chat"
          on:click|stopPropagation={toggleAttachmentMenu}>+</button
        >
      </div>
    </div>
  </div>

  {#if lightbox}
    <div
      class="lightbox-overlay"
      on:click={closeLightbox}
      transition:fly={{ duration: 150 }}
    >
      <img
        class="lightbox-img"
        src={lightbox.url}
        alt={lightbox.name}
        on:click|stopPropagation
      />
      <button class="lightbox-close" on:click={closeLightbox}>✕</button>
    </div>
  {/if}
</main>

<!-- -->
