<script>
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { registerSocket as socket } from "$lib/socket.js";
  import { SOCKET_EVENTS } from "$lib/socket-events.js";
  import "../../style.css";

  let username = "";
  let email = "";
  let password = "";
  let confirmpassword = "";

  let message = "";
  let loading = false;
  let showPasswords = false;

  function isValidEmail(value) {
    return /^\S+@\S+\.\S+$/.test(value);
  }

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

    if (!email) {
      stepOneErrors.push("Please enter an email address.");
    } else if (!isValidEmail(email)) {
      stepOneErrors.push("Please enter a valid email address.");
    }

    if (stepOneErrors.length > 0) {
      showPasswords = false;
      message = stepOneErrors.join(" ");
      return;
    }

    if (!showPasswords) {
      showPasswords = true;
      return;
    }

    const stepTwoErrors = [];

    if (!password) {
      stepTwoErrors.push("Please enter a password.");
    }

    if (!confirmpassword) {
      stepTwoErrors.push("Please confirm your password.");
    }

    if (password && confirmpassword && password !== confirmpassword) {
      stepTwoErrors.push("Passwords do not match.");
    }

    if (stepTwoErrors.length > 0) {
      message = stepTwoErrors.join(" ");
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
      <label for="email">Email:</label>
      <input id="email" type="email" bind:value={email} />
    </div>

    {#if showPasswords}
      <div class="field">
        <label for="password">Password:</label>
        <input id="password" type="password" bind:value={password} />
      </div>

      <div class="field">
        <label for="confirmpassword">Confirm Password:</label>
        <input id="confirmpassword" type="password" bind:value={confirmpassword} />
      </div>
    {/if}

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

  {#if message}
    <h3>{message}</h3>
  {/if}

  <div class="footer">
    <button class="button-return" onclick={() => goto("/")}>Go back</button>
  </div>
</main>