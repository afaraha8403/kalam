<script lang="ts">
  import { onMount } from 'svelte'
  import Settings from './pages/Settings.svelte'
  import History from './pages/History.svelte'
  import Snippets from './pages/Snippets.svelte'
  import Onboarding from './pages/Onboarding.svelte'

  let currentPage = 'settings'
  let isFirstRun = false

  onMount(async () => {
    // Check if first run
    // isFirstRun = await invoke('is_first_run')
  })

  function navigate(page: string) {
    currentPage = page
  }
</script>

{#if isFirstRun}
  <Onboarding />
{:else}
  <main class="app">
    <nav class="sidebar">
      <div class="logo">
        <h1>Kalam Voice</h1>
      </div>
      <ul class="nav-links">
        <li class:active={currentPage === 'settings'}>
          <button on:click={() => navigate('settings')}>Settings</button>
        </li>
        <li class:active={currentPage === 'history'}>
          <button on:click={() => navigate('history')}>History</button>
        </li>
        <li class:active={currentPage === 'snippets'}>
          <button on:click={() => navigate('snippets')}>Snippets</button>
        </li>
      </ul>
      <div class="footer">
        <p>Press <kbd>Ctrl+Win</kbd> to dictate</p>
      </div>
    </nav>

    <div class="content">
      {#if currentPage === 'settings'}
        <Settings />
      {:else if currentPage === 'history'}
        <History />
      {:else if currentPage === 'snippets'}
        <Snippets />
      {/if}
    </div>
  </main>
{/if}

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: #1a1a1a;
    color: #e0e0e0;
  }

  .app {
    display: flex;
    height: 100vh;
  }

  .sidebar {
    width: 240px;
    background: #252525;
    border-right: 1px solid #333;
    display: flex;
    flex-direction: column;
    padding: 20px;
  }

  .logo h1 {
    font-size: 20px;
    font-weight: 600;
    color: #4fc1ff;
    margin-bottom: 30px;
  }

  .nav-links {
    list-style: none;
    flex: 1;
  }

  .nav-links li {
    margin-bottom: 8px;
  }

  .nav-links button {
    width: 100%;
    padding: 12px 16px;
    background: transparent;
    border: none;
    border-radius: 8px;
    color: #b0b0b0;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s;
  }

  .nav-links li.active button,
  .nav-links button:hover {
    background: #333;
    color: #fff;
  }

  .nav-links li.active button {
    background: #4fc1ff;
    color: #1a1a1a;
  }

  .footer {
    padding-top: 20px;
    border-top: 1px solid #333;
    font-size: 12px;
    color: #666;
  }

  .footer kbd {
    background: #333;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 30px;
  }
</style>
