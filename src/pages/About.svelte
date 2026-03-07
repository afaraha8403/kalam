<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import Icon from '@iconify/svelte'

  let appVersion = ''
  let licenseOpen = false
  let updateChecking = false
  let updateStatus: 'idle' | 'up-to-date' | 'available' | 'error' = 'idle'
  let updateVersion = ''
  let updateError = ''

  // Set when repo is public, e.g. 'https://github.com/kalam-voice/kalam'
  const GITHUB_REPO_URL = ''

  const LICENSE_TEXT = `Dual License: MIT (Noncommercial) + Commercial by Permission

Copyright (c) 2026 Kalam Voice Contributors

----------------------------------------------------------------------
NONCOMMERCIAL USE — MIT License
----------------------------------------------------------------------

This software is licensed under the MIT License for noncommercial use only.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

  (1) Use is limited to NONCOMMERCIAL purposes. "Noncommercial" means
      personal use, educational use, or use by a nonprofit organization
      that is not primarily intended for or directed toward commercial
      advantage or monetary compensation.

  (2) COMMERCIAL USE, and any use or modification of the Software or
      derivative works for RESALE or for use by a business (including
      sole proprietorships, companies, and other for-profit entities),
      is NOT permitted under this license. Such use requires a separate
      written license from the copyright holder.

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

----------------------------------------------------------------------
COMMERCIAL USE
----------------------------------------------------------------------

For commercial use, resale, or use by a business, you must obtain a
separate license from the project owner. Please reach out to the project
maintainers to request a commercial license.`

  onMount(async () => {
    try {
      appVersion = (await invoke('get_app_version')) as string
    } catch {
      appVersion = '—'
    }
  })

  async function checkUpdates() {
    updateChecking = true
    updateStatus = 'idle'
    updateVersion = ''
    updateError = ''
    try {
      const result = (await invoke('check_for_updates')) as string | null
      if (result != null && result !== '') {
        updateStatus = 'available'
        updateVersion = result
      } else {
        updateStatus = 'up-to-date'
      }
    } catch (e) {
      updateStatus = 'error'
      updateError = e instanceof Error ? e.message : String(e)
    } finally {
      updateChecking = false
    }
  }
</script>

<div class="about">
  <header>
    <h2>About Kalam</h2>
    <p class="version">Version {appVersion || '…'}</p>
  </header>

  <section class="creator">
    <p class="byline">By <a href="https://github.com/afaraha8403" target="_blank" rel="noopener noreferrer">Ali Farahat</a>, founder of <a href="https://balacode.io" target="_blank" rel="noopener noreferrer">Balacode.io</a>.</p>
  </section>

  <section class="github-section" class:placeholder={!GITHUB_REPO_URL}>
    {#if GITHUB_REPO_URL}
      <a
        href={GITHUB_REPO_URL}
        target="_blank"
        rel="noopener noreferrer"
        class="github-link"
      >
        <Icon icon="ph:github-logo-duotone" class="github-icon" />
        <span>Report issues · Learn more</span>
      </a>
    {:else}
      <span class="github-link">
        <Icon icon="ph:github-logo-duotone" class="github-icon" />
        <span>Report issues · Learn more <em>(link coming soon)</em></span>
      </span>
    {/if}
  </section>

  <section class="updates">
    <button
      type="button"
      class="btn-check"
      disabled={updateChecking}
      on:click={checkUpdates}
    >
      {#if updateChecking}
        <Icon icon="ph:spinner-gap-duotone" class="spin" />
        Checking…
      {:else}
        <Icon icon="ph:arrow-square-in-duotone" />
        Check for updates
      {/if}
    </button>
    {#if updateStatus === 'up-to-date'}
      <p class="update-msg success">You're up to date.</p>
    {:else if updateStatus === 'available'}
      <p class="update-msg available">Update {updateVersion} available. Restart the app to install.</p>
    {:else if updateStatus === 'error'}
      <p class="update-msg error">{updateError}</p>
    {/if}
  </section>

  <section class="license-section">
    <button
      type="button"
      class="accordion"
      class:open={licenseOpen}
      on:click={() => (licenseOpen = !licenseOpen)}
      aria-expanded={licenseOpen}
    >
      <Icon icon={licenseOpen ? 'ph:caret-down-duotone' : 'ph:caret-right-duotone'} class="accordion-caret" />
      License
    </button>
    {#if licenseOpen}
      <div class="license-content">
        <pre class="license-text">{LICENSE_TEXT}</pre>
      </div>
    {/if}
  </section>
</div>

<style>
  .about {
    max-width: 560px;
  }

  header {
    margin-bottom: 28px;
  }

  header h2 {
    font-family: 'Syne', sans-serif;
    font-size: 22px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0 0 4px 0;
    letter-spacing: -0.02em;
  }

  .version {
    font-size: 14px;
    color: var(--text-muted);
    margin: 0;
  }

  .creator {
    margin-bottom: 28px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .byline {
    margin: 0 0 6px 0;
    font-size: 15px;
    color: var(--text-primary);
  }

  .creator a {
    color: var(--primary-dark);
    font-weight: 600;
    text-decoration: none;
  }

  .creator a:hover {
    text-decoration: underline;
  }

  .github-section {
    margin-bottom: 24px;
    padding: 12px 16px;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .github-section.placeholder .github-link {
    color: var(--text-muted);
    cursor: default;
  }

  .github-link {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    color: var(--primary-dark);
    text-decoration: none;
    font-weight: 500;
  }

  .github-link:hover:not(span) {
    text-decoration: underline;
  }

  .github-link em {
    font-style: normal;
    font-size: 13px;
    color: var(--text-muted);
    font-weight: 400;
  }

  .github-icon {
    font-size: 22px;
    color: var(--text-secondary);
  }

  .updates {
    margin-bottom: 24px;
  }

  .btn-check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    font-size: 14px;
    font-weight: 500;
    color: var(--primary-dark);
    background: var(--primary-alpha);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }

  .btn-check:hover:not(:disabled) {
    background: var(--primary);
    color: white;
  }

  .btn-check:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .btn-check :global(.spin) {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .update-msg {
    margin: 12px 0 0 0;
    font-size: 14px;
  }

  .update-msg.success {
    color: var(--text-secondary);
  }

  .update-msg.available {
    color: var(--primary-dark);
    font-weight: 500;
  }

  .update-msg.error {
    color: var(--text-secondary);
  }

  .license-section {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-card);
  }

  .accordion {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.2s, color 0.2s;
  }

  .accordion:hover {
    background: var(--bg-input);
    color: var(--navy-deep);
  }

  .accordion :global(.accordion-caret) {
    font-size: 16px;
    color: var(--text-muted);
  }

  .license-content {
    border-top: 1px solid var(--border-subtle);
    padding: 16px;
    max-height: 320px;
    overflow-y: auto;
  }

  .license-text {
    margin: 0;
    font-family: 'DM Sans', ui-monospace, monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
