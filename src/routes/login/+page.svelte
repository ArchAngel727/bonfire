<script>
  //import "../../style.css";

 

  let email = "";
  let password = "";
  let error = "";

 

  function login() {
    if (!email || !password) {
      error = "Fill everything out";
      return;
    }

 

    // 🧪 MOCK Backend Simulation
    setTimeout(() => {
      if (email === "test@test.com" && password === "1234") {
        error = "";
        alert("Login erfolgreich (MOCK)");
      } else {
        error = "Falsche E-Mail oder Passwort";
      }
    }, 500);
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
      type="email"
      placeholder="E-Mail"
      bind:value={email}
      on:keydown={handleKey}
    />

 

    <input
      type="password"
      placeholder="Passwort"
      bind:value={password}
      on:keydown={handleKey}
    />

 

    {#if error}
      <p class="error">{error}</p>
    {/if}

 

    <button class="loginButton" on:click={login}> Log in </button>

 

    <div class="containerButton">
      <a href="/" class="link">Home</a>
      <a href="/register" class="link">Register</a>
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
  .containerButton {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 15px;
    gap: 50px;
  }

 

  .card {
    background: rgb(67, 65, 65);
    padding: 2rem;
    border-radius: 16px;
    width: 340px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    text-align: center;
  }

 

  h1 {
    margin-bottom: 1.5rem;
    margin-bottom: 48px;
    font-size: 62px;
  }

 

  input {
    width: 100%;
    box-sizing: border-box;
    padding: 0.75rem;
    margin-bottom: 1.75rem;
    border-radius: 8px;
    border: 2px solid #000000;
    font-size: 1rem;
  }

 

  input:focus {
    outline: none;
    border-color: #3b82f6;
  }

 

  .loginButton {
    width: 100%;
    margin-top: 10px;
    padding: 0.75rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 1rem;
    transition: 0.2s;
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
    margin-top: 0.5rem;
    font-size: 0.9rem;
    color: #3b82f6;
    text-decoration: none;
  }

 

  .link:hover {
    text-decoration: underline;
  }
</style>