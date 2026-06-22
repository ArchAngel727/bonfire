<script>
  import '../style.css';
  import { page } from '$app/stores';

  $: isAppPage = $page.url.pathname === '/chat'||$page.url.pathname === '/profile';;

</script>

<div class="app-shell">
  {#if !isAppPage}
    <header class="site-header">
    
      <div class="header-inner">
        <div id="logoandtitle">
        <img id="logo-header" src="src//img/bonfire_icon_4k.png" alt="Bonfire logo" class="logo" />
        <a href="/" class="brand">Bonfire</a>
        </div>
        <nav class="main-nav">
          <a href="/">Home</a>
          <a href="/register">Register</a>
          <a href="/login">Login</a>
        </nav>
      </div>
    </header>
  {/if}
  <main class="site-main" class:chat-page={$page.url.pathname === '/chat'}>
    <slot />
  </main>

  {#if !isAppPage}
    <footer class="site-footer">
      <p>© 2026 Bonfire</p>
    </footer>
  {/if}
</div>