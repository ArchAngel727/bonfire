<script>
  import '../style.css';
  import { page } from '$app/stores';
  import { isLoggedIn, logout } from '$lib/auth.js';
  import { fly } from 'svelte/transition';

  let showUserMenu = false;
  let userMenuWrapper;

  function toggleUserMenu() {
    showUserMenu = !showUserMenu;
  }

  function closeUserMenu() {
    showUserMenu = false;
  }
</script>

<div class="app-shell">
  <header class="site-header">
    <div class="header-inner">
      <div id="logoandtitle">
      <img id="logo-header" src="src//img/bonfire_icon_4k.png" alt="Bonfire logo" class="logo" />
      <a href="/" class="brand">Bonfire</a>
      </div>
      <nav class="main-nav">
        <a href="/">Home</a>
        <a href="/register">Register</a>
        {#if !$isLoggedIn}
          <a href="/login">Login</a>
        {/if}
        <a href="/chat">Chat</a>
      </nav>
        {#if $isLoggedIn}
        <div class="user-menu-wrapper" bind:this={userMenuWrapper}>
          <button
            class="user-button"
            on:click|stopPropagation={toggleUserMenu}
            aria-label="User-Menü öffnen"
          >
            <img src="src//img/user.png" alt="" class="icons" />
          </button>

          {#if showUserMenu}
            <div class="user-menu" transition:fly={{ y: -10, duration: 250 }}>
              <button class="user-menu-item" on:click={() => { logout(); closeUserMenu(); }}>
                Logout
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </header>

  <main class="site-main" class:chat-page={$page.url.pathname === '/chat'}>
    <slot />
  </main>


  <footer class="site-footer">
    <p>© 2026 Bonfire</p>
  </footer>
</div>