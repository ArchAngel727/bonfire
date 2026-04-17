<script>
    import { goto } from "$app/navigation";
    import { show } from "@tauri-apps/api/app";
    import { invoke } from "@tauri-apps/api/core";
    import { on } from "svelte/events";
    import "../../style.css";
</script>

<main class="container">

<h1>Register</h1>


<button id="button-return" onclick={() => goto("/")}>Go back</button>
</main>
