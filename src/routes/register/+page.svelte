<script>
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { registerSocket as socket } from "$lib/socket.js";
  import { SOCKET_EVENTS } from "$lib/socket-events.js";
  import "../../style.css";

  let username = "";
  let password = "";
  let confirmpassword = "";

  let message = "";
  let loading = false;
  let showPasswords = false;

  function resetMessage() {
    message = "";
  }

  onMount(() => {
    socket.connect();

    socket.on(SOCKET_EVENTS.CONNECT, () => {
      console.log("Socket connected:", socket.id);
    });

    socket.on(SOCKET_EVENTS.CONNECT_ERROR, (error) => {
      loading = false;
      message = error?.message || "Could not connect to server.";
    });

    return () => {
      socket.off(SOCKET_EVENTS.CONNECT_ERROR);
    };
  });

  function handleRegister() {
    resetMessage();

    const stepOneErrors = [];

    if (!username) {
      stepOneErrors.push("Please enter a username.");
    }

    if (!password) {
      stepOneErrors.push("Please enter a password.");
    }

    if (!confirmpassword) {
      stepOneErrors.push("Please confirm your password.");
    }

    if (password && confirmpassword && password !== confirmpassword) {
      stepOneErrors.push("Passwords do not match.");
    }

    if (stepOneErrors.length > 0) {
      message = stepOneErrors.join(" ");
      return;
    }

    loading = true;

    // ← alles hier DRIN in der Funktion!
    console.log("Sende Event:", SOCKET_EVENTS.REGISTER_REQUEST);
    console.log("Socket connected?", socket.connected);

    socket.emit(
      SOCKET_EVENTS.REGISTER_REQUEST,
      { username, password },
      (response) => {
        console.log("ACK bekommen:", response);
        loading = false;
        if (response === "Error") {
          message = "Registration failed.";
        } else {
          message = "Account created! ✅";
        }
      }
    );
  }
</script>

<main class="container">
  <h1 class="h1">Register</h1>

  <form class="register-form" novalidate onsubmit={handleRegister}>
    <div class="field">
      <label for="username">Username:</label>
      <input id="username" type="text" bind:value={username} />
    </div>

      <div class="field">
        <label for="password">Password:</label>
        <input id="password" type="password" bind:value={password} />
      </div>

      <div class="field">
        <label for="confirmpassword">Confirm Password:</label>
        <input id="confirmpassword" type="password" bind:value={confirmpassword} />
      </div>

    <button class="button-main" type="submit" disabled={loading}>
      {#if loading}
        Loading...
      {:else if showPasswords}
        Register
      {:else}
        Continue
      {/if}
    </button>
    
  </form>
  <button class="button-return" onclick={() => goto("/")}>Go back</button>

  {#if message}
    <h3>{message}</h3>
  {/if}

  
</main>