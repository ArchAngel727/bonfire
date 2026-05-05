<script>
  import { io } from "socket.io-client";
  import { onMount, onDestroy } from "svelte";
  import "../../style.css";

  let username = "";
  let password = "";
  let error = "";
  let socket = io("ws://127.0.0.1:3000/login");;
  let connected = false;

  onMount(() => {

    socket.on("connect", () => {
      console.log("Connected with server /login!");
      connected = true;
    });

    socket.on("connect_error", (err) => {
      console.log("❌ Fehler:", err.message);
      error = "Server nicht erreichbar!";
    });

    socket.on("disconnect", () => {
      connected = false;
    });
  });

  onDestroy(() => {
    socket?.disconnect();
  });

  function login() {
  if (!username || !password) {
    error = "Username or Password missing!";
    return;
  }

  if (!connected) {
    error = "Can't reach server";
    return;
  }

  console.log("Login with:", { username, password });

  // ← Timeout: wenn Server in 5 Sek nicht antwortet, Fehler!
  const timeout = setTimeout(() => {
    console.log("⏱️ Server antwortet nicht!");
    error = "Server antwortet nicht (Timeout)";
  }, 5000);

  socket.emit("login", { username, password }, (response) => {
    clearTimeout(timeout);
    console.log("Server:", response);

    if (response?.session_id && response.signature) {
      error = "";
      alert("Login succesfull!");
    } else {
      error = response?.message || "Wrong Username oder Password";
    }
  });
}

  function handleKey(e) {
    if (e.key === "Enter") {
      login();
    }
  }
</script>

<main>
  <div class="card">
    <h1>Login</h1>

    <input
      type="text"
      placeholder="username"
      bind:value={username}
    />

    <input
      type="password"
      placeholder="Passwort"
      bind:value={password}
    />

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <button class="loginButton" on:click={login}> Einloggen </button>
    <div class="containerLinks">
      <a href="/register" class="link">Register</a>
      <a href="/" class="link">Home</a>
    </div>
  </div>
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    background: #2f2f2f;
    font-family: Arial, sans-serif;
  }

  .card {
    background: rgb(67, 65, 65);
    padding: 2rem;
    border-radius: 16px;
    width: 320px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    text-align: center;
    color: rgb(255, 255, 255);
  }

  h1 {
    margin-bottom: 1.5rem;
  }

  input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.75rem;
    margin-bottom: 1rem;
    border-radius: 8px;
    border: 1px solid #ccc;
    font-size: 1rem;
    color: black;
  }

  input:focus {
    outline: none;
    border-color: black;
  }

  .loginButton {
    width: 100%;
    padding: 0.75rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1rem;
    transition: 0.2s;
    margin-top: 12px;
  }

  .loginButton:hover {
    background: #2563eb;
  }

  .error {
    color: red;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  .link {
    display: block;
    margin-top: 1rem;
    font-size: 0.9rem;
    color: #3b82f6;
    text-decoration: none;
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
