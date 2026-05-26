<script>
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { slide } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { isLoggedIn } from '$lib/auth.js';
  import { goto } from '$app/navigation';
  import { blur } from 'svelte/transition';
  import { friends } from "$lib/mock/friends";
  import { messages } from "$lib/mock/messages";


  let openAttachment = false;
  let selectedFriend = friends[0];

  /** @type {File[]} */
  let files = [];
  /** @type { { name: string; size: number; type: string; url: string }[] } */
  let previews = [];

  /** @type { { name: string; size: number; type: string; url: string } | null } */
  let lightbox = null;

  function openLightbox(preview) { lightbox = preview; }
  function closeLightbox() { lightbox = null; }

  /** @type {HTMLDivElement | undefined} */
  let attachmentMenu;

  /** @type {HTMLInputElement | undefined} */
  let fileInput;

  let searchOpen = false;

  function selectFriend(friend) {
    selectedFriend = friend;
  }

  function toggleAttachmentMenu() {
    openAttachment = !openAttachment;
  }

  function closeAttachmentMenu() {
    openAttachment = false;
  }

  /**
   * @param {string} accept 
   */
  function openFilePicker(accept) {
    closeAttachmentMenu();
    fileInput.accept = accept;
    fileInput.click();
  }

  function toggleSearchBar() {
    searchOpen = !searchOpen;
  }

  function handleFileChange(event) {
  const selected = Array.from(event.target.files);

  selected.forEach((file) => {
    const url = URL.createObjectURL(file);
    previews = [...previews, { name: file.name, size: file.size, type: file.type, url }];
    files = [...files, file];
  });
}

  function removePreview(index) {
    URL.revokeObjectURL(previews[index].url);
    previews = previews.filter((_, i) => i !== index);
    files = files.filter((_, i) => i !== index);
  }

  function getIcon(type) {
    if (type.includes('word') || type.includes('document')) return '📝';
    if (type.includes('excel') || type.includes('sheet')) return '📊';
    if (type.includes('zip') || type.includes('compressed')) return '🗜️';
    if (type.includes('pdf')) return '📕';
    return '📄';
  }

  function formatSize(bytes) {
    if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    return (bytes / 1024).toFixed(1) + ' KB';
  }

  onMount(() => {
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

    document.addEventListener('click', handleClick);
    return () => document.removeEventListener('click', handleClick);
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
{#if $isLoggedIn}
  <div class:attachment={openAttachment} class="chat-layout">
    <aside class="friends">
    <div class="friends-header">
      <h2>Friends</h2>
      <button on:click={toggleSearchBar} class="user-button">
        <img src="src//img/search.png" class="icons-search">
      </button>
    </div>

    <div class="friends-list">

  {#each friends as friend}

    <div
      class:selected={selectedFriend.id === friend.id}
      class="friend-item"
      on:click={() => selectFriend(friend)}
    >

      <img
        class="friend-avatar"
        src={friend.avatar}
        alt={friend.username}
      />

      <div class="friend-info">

        <div class="friend-top">

          <h3>{friend.username}</h3>

          {#if friend.online}
            <span class="online-dot"></span>
          {/if}

        </div>

      </div>

      

    </div>

  {/each}

</div>

     {#if searchOpen}
        <div class="search-bar" transition:slide>
          <input type="text" placeholder="Search friends..." class="search-input">
        </div>
     {/if}

    </aside>

    <section class="chat-history">

  <div class="chat-header">

  <h2>{selectedFriend.username}</h2>

  <span class="chat-status">
    {selectedFriend.online ? "online" : "offline"}
  </span>

  </div>

  <div class="messages">

  {#each selectedFriend.messages as message}

    <div
    class:own={message.own}
    class="message-row"
  >
    <div class="message-timeandauthor">
    <span class="message-author">
      {message.own ? "You" : selectedFriend.username}
    </span>

    <span class="message-time">
      {message.time}
    </span>
    </div>

    <span class="message-text">
      {message.text}
    </span>

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
      {#if preview.type.startsWith('image/')}
        <div class="preview-thumb">
          <img
            src={preview.url}
            alt={preview.name}
            on:click={() => openLightbox(preview)}
          />
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
        placeholder="Type your message..."
        id="message-input"
      />
    </div>

    <div class="chat-buttons">
      <button class="button-chat">Send</button>

      <div class="attachment-menu-wrapper" bind:this={attachmentMenu}>
        {#if openAttachment}
          <div class="attachment-panel" transition:fly={{ y: 20, duration: 350 }}>
            <button class="attachment-option" on:click={() => openFilePicker('*')}>File</button>
          </div>
        {/if}

        <button class="button-chat" on:click|stopPropagation={toggleAttachmentMenu}>
          +
        </button>
      </div>
    </div>
  </div>

  {#if lightbox}
    <div class="lightbox-overlay" on:click={closeLightbox} transition:fly={{ duration: 150 }}>
      <img class="lightbox-img" src={lightbox.url} alt={lightbox.name} on:click|stopPropagation />
      <button class="lightbox-close" on:click={closeLightbox}>✕</button>
    </div>
  {/if}

{:else}
  <div class="popup-overlay2" transition:blur={{ duration: 400 }}>
    <h2>Please log in or register to access the chat</h2>
    <div class="popup-buttons">
    <button class="button-main2" on:click={() => goto("/register")}>Register</button>
    <button class="button-main2" on:click={() => goto("/login")}>Login</button>
    </div>
  </div>
{/if}
</main>

