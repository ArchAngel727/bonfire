<script>
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

    //wenn backend fertig ist dann function onMount save und avatar format ändern


  let myUsername = null;
  let bio = "";
  let avatarUrl = "";   // später fürs backend um das bild hochzulanden

  let saved = false;    //gespeichert von user
  let fileInput;

  onMount(() => {
    if (!localStorage.getItem("session")) {
      goto("/login");
      return;
    }
    myUsername = localStorage.getItem("username");
    bio = localStorage.getItem("profileBio") ?? "";
    avatarUrl = localStorage.getItem("profileAvatar") ?? "";
  });

  function handleAvatarChange(event) {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      alert("Bitte ein Bild auswählen.");
      return;
    }
    // In Data-URL umwandeln, damit wir es in localStorage ablegen können.
    const reader = new FileReader();
    reader.onload = () => {
      avatarUrl = reader.result;
    };
    reader.readAsDataURL(file);
  }

  function removeAvatar() {
    avatarUrl = "";
    if (fileInput) fileInput.value = "";
  }


//nur zum testen hier noch backend  
  function save() {
    try {
      localStorage.setItem("profileBio", bio);
      if (avatarUrl) {
        localStorage.setItem("profileAvatar", avatarUrl);
      } else {
        localStorage.removeItem("profileAvatar");
      }
      saved = true;
      setTimeout(() => (saved = false), 1500);
    } catch (e) {
      alert("Speichern fehlgeschlagen (Bild evtl. zu groß).");
      console.error(e);
    }
  }
</script>

<main class="profilePage">
  <a href="/chat" class="back">← Zurück zum Chat</a>

  <h1>Mein Profil</h1>

  <div class="profilePicture">
    {#if avatarUrl}
      <img class="avatar" src={avatarUrl} alt="Avatar" />
    {:else}
      <div class="avatar avatar-placeholder">
        {myUsername ? myUsername[0].toUpperCase() : "?"}
      </div>
    {/if}

    <div class="profilePictureButtons">
      <button on:click={() => fileInput.click()}>Bild auswählen</button>
      {#if avatarUrl}
        <button class="secondary" on:click={removeAvatar}>Entfernen</button>
      {/if}
    </div>
    <input
      bind:this={fileInput}
      type="file"
      accept="image/*"
      hidden
      on:change={handleAvatarChange}
    />
  </div>

  <label class="fieldInput">
    <span>Username</span>
    <input type="text" value={myUsername ?? ""} disabled />
  </label>

  <label class="fieldInput">
    <span>Bio</span>
    <textarea
      bind:value={bio}
      rows="4"
      maxlength="280"
      placeholder="Erzähl was über dich…"
    ></textarea>
    <small class="counter">{bio.length} / 280</small>
  </label>

  <button class="save" on:click={save}>Speichern</button>
  {#if saved}
    <p class="messageSaved">✓ Gespeichert</p>
  {/if}

  
</main>




<style>
  .profilePage {
    max-width: 480px;
    margin: 2rem auto;
    padding: 1.5rem;
    color: #eee;
    font-family: Arial, sans-serif;
  }
  .back {
    color: #ff8c32;
    text-decoration: none;
    font-size: 0.9rem;
  }
  .back:hover {
    text-decoration: underline;
  }
  h1 {
    margin: 1rem 0 1.5rem;
  }

  .profilePicture {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }
  .avatar {
    width: 128px;
    height: 128px;
    border-radius: 50%;
    object-fit: cover;
    background: #333;
  }
  .avatar-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 3rem;
    color: #888;
    background: #2a2a2a;
  }
  .profilePictureButtons {
    display: flex;
    gap: 0.5rem;
  }

  .fieldInput {
    display: block;
    margin-bottom: 1rem;
  }
  .fieldInput span {
    display: block;
    font-size: 0.85rem;
    color: #aaa;
    margin-bottom: 0.3rem;
  }
  .fieldInput input,
  .fieldInput textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 0.6rem;
    border-radius: 6px;
    border: 1px solid #444;
    background: #1e1e1e;
    color: #eee;
    font-size: 1rem;
    font-family: inherit;
    resize: vertical;
  }
  .fieldInput input:disabled {
    opacity: 0.6;
  }
  .counter {
    display: block;
    text-align: right;
    color: #777;
    font-size: 0.75rem;
    margin-top: 0.2rem;
  }

  button {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: none;
    background: #ff8c32;
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
  }
  button:hover {
    background: #e87a20;
  }
  button.secondary {
    background: transparent;
    border: 1px solid #555;
    color: #ccc;
  }
  button.secondary:hover {
    background: #2a2a2a;
  }

  .save {
    width: 100%;
    padding: 0.75rem;
    font-size: 1rem;
    margin-top: 0.5rem;
  }
  .messageSaved {
    text-align: center;
    color: #6ad06a;
    margin-top: 0.5rem;
  }
</style>