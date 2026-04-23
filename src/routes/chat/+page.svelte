<script>
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';

  let openAttachment = false;

  /** @type {HTMLDivElement | undefined} */
  let attachmentMenu;

  function toggleAttachmentMenu() {
    openAttachment = !openAttachment;
  }

  function closeAttachmentMenu() {
    openAttachment = false;
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

    return () => {
      document.removeEventListener('click', handleClick);
    };
  });
</script>

<main class="site-main chat-page">
  <div class:attachment={openAttachment} class="chat-layout">
    <aside class="friends">
      <h2>Freunde</h2>
    </aside>

    <section class="chat-history">
      <h2>Username</h2>
    </section>

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
          <div
            class="attachment-panel"
            transition:fly={{ y: 20, duration: 350 }}
          >
            <button class="attachment-option">Bild</button>
            <button class="attachment-option">Datei</button>
            <button class="attachment-option">Video</button>
          </div>
        {/if}

        <button
          class="button-chat"
          on:click|stopPropagation={toggleAttachmentMenu}
        >
          +
        </button>
      </div>
    </div>
  </div>
</main>