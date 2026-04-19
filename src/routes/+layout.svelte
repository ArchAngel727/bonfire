<script>
  import '../style.css';
</script>

<div class="app-shell">
  <header class="site-header">
    <div class="header-inner">
      <div id="logoandtitle">
      <a href="/" class="brand">Bonfire</a>
      <img id="logo-header" src="src//img/bonfire_icon_4k.png" alt="Bonfire logo" class="logo" />
      </div>
      <nav class="main-nav">
        <a href="/">Home</a>
        <a href="/register">Register</a>
        <a href="/login">Login</a>
      </nav>
    </div>
  </header>

  <main class="site-main">
    <slot />
  </main>

  <footer class="site-footer">
    <p>© 2026 Bonfire. All rights reserved.</p>
  </footer>
</div>