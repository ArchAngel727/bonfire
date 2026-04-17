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

    <nav class="bar">
      <a href="/register">Register</a>
      <a href="/login">Login</a>
    </nav>

    <p>Welcome to Bonfire! Bonfire is a modern Discord-like platform designed for real-time conversation, shared spaces, and meaningful connections. Create servers, join communities, chat with friends, and stay connected through a fast, simple, and welcoming experience.</p>

    <button id="goto-btn" onclick={() => openPopup()}>Get started</button>
   {#if showPopup}
  <div class="popup-overlay" onclick={closePopup}>
    <div
      class="popup"
      onclick={(event) => {
        event.stopPropagation();
      }}
    >
      <h2>Welcome to Bonfire</h2>
      <p>Start by creating an account or logging in.</p>
      <button id="button-main" onclick={() => goto("/register")}>Register</button>
      <button id="button-main" onclick={() => goto("/login")}>Login</button>
      <div class="close-btn">
        <button id="close-btn" onclick={closePopup}>Close</button>
      </div>
    </div>
  </div>
{/if}
  </section>
</main>

