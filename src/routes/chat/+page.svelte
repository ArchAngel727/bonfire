<script>
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';

  let openAttachment = false;

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
  <div class:attachment={openAttachment} class="chat-layout">
    <aside class="friends">
      <h2>Freunde</h2>
    </aside>

    <section class="chat-history">
      <h2>Username</h2>
    </section>

{#if previews.length > 0}
  <div class="preview-bar">
    {#each previews as preview, i}

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
        <!-- Alles andere: Chip mit Icon + Name + Größe -->
        <div class="file-chip">
          <span class="file-chip-icon">{getIcon(preview.type)}</span>
          <span class="file-chip-meta">
            <span class="file-chip-name">{preview.name}</span>
            <span class="file-chip-size">{formatSize(preview.size)}</span>
          </span>
          <button class="file-chip-remove" on:click={() => removePreview(i)}>✕</button>
        </div>
      {/if}

    {/each}
  </div>
{/if}

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
            <button class="attachment-option" on:click={() => openFilePicker('*')}>Datei</button>
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

</main>