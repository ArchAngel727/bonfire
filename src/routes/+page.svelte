<script>
    import { goto } from "$app/navigation";
    import { show } from "@tauri-apps/api/app";
    import { invoke } from "@tauri-apps/api/core";
    import { on } from "svelte/events";
    import "../style.css";

  let showPopup = false;

  function openPopup() {
    showPopup = true;
  }

  function closePopup() {
    showPopup = false;
  }
</script>

<main class="container">
  <section class="hero">
    <div class="title-group">
      <img src="src//img/bonfire_icon_4k.png" alt="Bonfire logo" class="logo" />
      <h1 id="titletext">Bonfire</h1>
    </div>

    <p>Welcome to Bonfire! Bonfire is a modern Discord-like platform designed for real-time conversation, shared spaces, and meaningful connections. Create servers, join communities, chat with friends, and stay connected through a fast, simple, and welcoming experience.</p>

    <button class="button-main" onclick={() => openPopup()}>Get started</button>
   {#if showPopup}
  <div class="popup-overlay" onclick={closePopup}>
    <div
      class="popup"
      onclick={(event) => {
        event.stopPropagation();
      }}
    > 
      <div class ="popup-header">
      <h2>Welcome to Bonfire</h2>
      <p>Start by creating an account or logging in.</p>
      </div>

      <div class="popup-buttons">
      <button class="button-main" onclick={() => goto("/register")}>Register</button>
      <button class="button-main" onclick={() => goto("/login")}>Login</button>
      <button class="top-left-btn" onclick={closePopup}>X</button>
      </div>
    </div>
  </div>
<<<<<<< HEAD
{/if}
  </section>
=======
  <p>Click on the Tauri, Vite, and SvelteKit logos to learn more.</p>

  <form class="row" onsubmit={greet}>
    <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
    <button type="submit">Greet</button>
  </form>
  <p>{greetMsg}</p>
  <a href="/login">
    <button>Zum Login</button>
  </a>
>>>>>>> login
</main>

