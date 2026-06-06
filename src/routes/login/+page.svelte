<script>
  import { io } from "socket.io-client";
  import { onMount, onDestroy } from "svelte";
  import "../../style.css";
  import { goto } from "$app/navigation";

  let username = "";
  let password = "";
  let error = "";
  let socket = io("ws://127.0.0.1:3000/login");
  let connected = false;
  let isLoggedIn = false;
  let cookie = null;

  onMount(() => {
    const saved = localStorage.getItem("session");

    if (saved) {
      try {
        cookie = JSON.parse(saved);
        isLoggedIn = true;
        goto("/chat");
      } catch (e) {
        localStorage.removeItem("session");
      }
    }

    socket.on("connect", () => {
      connected = true;
    });

    socket.on("connect_error", (err) => {
      console.log("Error:", err.message);
      error = "Can't reach server";
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

    const timeout = setTimeout(() => {
      console.log("Server Timeout");
      error = "Server Timeout";
    }, 5000);

    socket.emit("login", { username, password }, (response) => {
      clearTimeout(timeout);
      console.log("Server:", response);

      if (response?.session_id && response.signature) {
        cookie = {
          session_id: Array.from(response.session_id),
          signature: Array.from(response.signature),
        };

        localStorage.setItem("session", JSON.stringify(cookie));
        isLoggedIn = true;
        error = "";
        goto("/chat");
        alert("Login succesfull!");
      } else {
        error = response?.message || "Wrong Username oder Password";
      }
    });
  }

  function logout() {
    localStorage.removeItem("session");
    cookie = null;
    isLoggedIn = false;
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

    <input type="text" placeholder="Username" bind:value={username} />

    <input type="password" placeholder="Passwort" bind:value={password} />

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
