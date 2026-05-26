<script>
    import "../../style.css";

    /** @type {string} */
    let avatarUrl = '';

    /** @type {HTMLInputElement | undefined} */
    let avatarInput;

    function openAvatarPicker() {
        avatarInput.click();
    }

    function handleAvatarChange(event) {
        const file = event.target.files[0];
        if (!file) return;
        if (avatarUrl) URL.revokeObjectURL(avatarUrl);
        avatarUrl = URL.createObjectURL(file);
    }
</script>

<input
  bind:this={avatarInput}
  type="file"
  accept="image/*"
  hidden
  on:change={handleAvatarChange}
/>

<main class="container">
  <div class="profile-wrap">
    <h1>Dein Profil</h1>

    <div class="profile-card">

    <!-- Avatar mit Upload -->
    <div class="avatar-wrap" on:click={openAvatarPicker}>
    <div class="avatar-circle">
        {#if avatarUrl}
        <img src={avatarUrl} alt="Profilbild" />
        {:else}
        P
        {/if}
    </div>
    <div class="avatar-overlay">
        <i class="ti ti-pencil"></i>
    </div>
    <div class="status-dot"></div>
    </div>

      <div class="profile-info">
        <p class="profile-name">Philipp</p>
        <p class="profile-handle">@philipp</p>

        <div class="divider"></div>

        <div class="username-row">
          <div class="info-row">
            <i class="ti ti-user info-icon"></i>
            <div class="info-col">
              <span class="info-label">Username</span>
              <span class="info-value">Philipp</span>
            </div>
          </div>

          <button class="button-main3">
            Bearbeiten
          </button>
        </div>
      </div>

    </div>
  </div>
</main>