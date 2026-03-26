<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '$lib/backend'
  import { listen } from '@tauri-apps/api/event'
  import Icon from '@iconify/svelte'

  /** When true, use prototype settings About layout (version + updates bar, two cards, license). */
  export let embeddedInSettings = false

  let appVersion = ''
  let licenseOpen = false
  let legalOpen = false
  let updateChecking = false
  let updateStatus: 'idle' | 'up-to-date' | 'available' | 'error' = 'idle'
  let updateVersion = ''
  let updateError = ''
  let updateChannel: 'stable' | 'beta' = 'stable'
  let updateInstalling = false
  /** When true, we are only downloading for next launch (no immediate restart). */
  let updateInstallingDeferred = false
  let updateDownloadPercent: number | null = null
  /** Set after a successful "update on next start" download+install (no restart). */
  let updateStagedMessage: string | null = null

  /** Embedded Settings → About: same collapsible sections as other settings tabs. */
  let aboutUpdatesCollapsed = false
  let aboutCommunityCollapsed = false

  const GITHUB_REPO_URL = 'https://github.com/afaraha8403/kalam'

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

  onMount(() => {
    let unlisten: (() => void) | null = null
    void (async () => {
      try {
        appVersion = (await invoke('get_app_version')) as string
      } catch {
        appVersion = '—'
      }
      try {
        const settings = (await invoke('get_settings')) as { update_channel?: 'stable' | 'beta' }
        updateChannel = settings?.update_channel === 'beta' ? 'beta' : 'stable'
      } catch {
        updateChannel = 'stable'
      }
      unlisten = await listen<[number, number | null, number | null]>(
        'update-download-progress',
        (e) => {
          const [, , percent] = e.payload
          updateDownloadPercent = percent != null ? Math.round(percent) : null
        }
      )
    })()
    return () => {
      unlisten?.()
    }
  })

  async function onChannelChange() {
    try {
      const config = (await invoke('get_settings')) as import('../types').AppConfig
      await invoke('save_settings', { newConfig: { ...config, update_channel: updateChannel } })
    } catch (e) {
      console.error('Failed to save update channel:', e)
    }
  }

  async function checkUpdates() {
    updateStagedMessage = null
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

  /**
   * @param restart - If true, restart immediately after install. If false, stage update for next launch.
   */
  async function downloadAndInstall(restart: boolean) {
    if (updateInstalling) return
    updateInstalling = true
    updateInstallingDeferred = !restart
    updateStagedMessage = null
    updateDownloadPercent = 0
    updateError = ''
    try {
      await invoke('download_and_install_update', { restart })
      // Immediate restart path: we never reach here
      if (!restart) {
        updateStagedMessage =
          'Update downloaded and installed. It will be active the next time you start Kalam.'
        // Keep user on a neutral state; message also shown under "Up to date" below.
        updateStatus = 'up-to-date'
      }
    } catch (e) {
      updateError = e instanceof Error ? e.message : String(e)
      updateStatus = 'error'
    } finally {
      updateInstalling = false
      updateInstallingDeferred = false
      updateDownloadPercent = null
    }
  }
</script>

{#if embeddedInSettings}
  <div class="settings-tab-content about-content">
    <section class="settings-section" class:collapsed={aboutUpdatesCollapsed}>
      <button
        type="button"
        class="section-header"
        on:click={() => (aboutUpdatesCollapsed = !aboutUpdatesCollapsed)}
        aria-expanded={!aboutUpdatesCollapsed}
      >
        <h3>Version &amp; updates</h3>
        <Icon icon={aboutUpdatesCollapsed ? 'ph:caret-down' : 'ph:caret-up'} />
      </button>
      {#if !aboutUpdatesCollapsed}
        <div class="section-content">
          <div class="setting-row">
            <div class="setting-label">
              <span class="setting-name">Current version</span>
              <span class="setting-desc">{appVersion || '…'}</span>
            </div>
            <div class="setting-control">
              <button type="button" class="btn-check" disabled={updateChecking} on:click={checkUpdates}>
                {#if updateChecking}
                  <Icon icon="ph:spinner-gap-duotone" class="spin" /> Checking…
                {:else}
                  <Icon icon="ph:magnifying-glass-duotone" /> Check now
                {/if}
              </button>
            </div>
          </div>

          <div class="setting-row">
            <div class="setting-label">
              <span class="setting-name">Channel</span>
              <span class="setting-desc">Stable or Beta releases</span>
            </div>
            <div class="setting-control">
              <select
                class="form-select"
                bind:value={updateChannel}
                on:change={onChannelChange}
                aria-label="Update channel"
              >
                <option value="stable">Stable</option>
                <option value="beta">Beta</option>
              </select>
            </div>
          </div>

          {#if updateStatus === 'up-to-date'}
            <div class="status-msg success about-status"><Icon icon="ph:check-circle-duotone" /> Up to date</div>
            {#if updateStagedMessage}
              <div class="status-msg success staged-hint"><Icon icon="ph:info-duotone" /> {updateStagedMessage}</div>
            {/if}
          {:else if updateStatus === 'available'}
            <div class="status-msg available about-status"><Icon icon="ph:sparkle-duotone" /> Update {updateVersion} available!</div>
            <div class="update-install-actions update-install-actions--split about-actions">
              <button
                type="button"
                class="btn-install"
                disabled={updateInstalling}
                on:click={() => downloadAndInstall(true)}
              >
                {#if updateInstalling && !updateInstallingDeferred}
                  {#if updateDownloadPercent != null}
                    <Icon icon="ph:spinner-gap-duotone" class="spin" /> Updating… {updateDownloadPercent}%
                  {:else}
                    <Icon icon="ph:spinner-gap-duotone" class="spin" /> Downloading &amp; installing…
                  {/if}
                {:else}
                  <Icon icon="ph:arrow-clockwise-duotone" /> Update now
                {/if}
              </button>
              <button
                type="button"
                class="btn-install-secondary"
                disabled={updateInstalling}
                on:click={() => downloadAndInstall(false)}
                title="Download and install in the background; apply when you quit and open Kalam again"
              >
                {#if updateInstalling && updateInstallingDeferred}
                  {#if updateDownloadPercent != null}
                    <Icon icon="ph:spinner-gap-duotone" class="spin" /> Downloading… {updateDownloadPercent}%
                  {:else}
                    <Icon icon="ph:spinner-gap-duotone" class="spin" /> Preparing…
                  {/if}
                {:else}
                  <Icon icon="ph:moon-stars-duotone" /> Update on next start
                {/if}
              </button>
            </div>
          {:else if updateStatus === 'error'}
            <div class="status-msg error about-status"><Icon icon="ph:warning-circle-duotone" /> {updateError}</div>
          {/if}
        </div>
      {/if}
    </section>

    <section class="settings-section" class:collapsed={aboutCommunityCollapsed}>
      <button
        type="button"
        class="section-header"
        on:click={() => (aboutCommunityCollapsed = !aboutCommunityCollapsed)}
        aria-expanded={!aboutCommunityCollapsed}
      >
        <h3>Community &amp; support</h3>
        <Icon icon={aboutCommunityCollapsed ? 'ph:caret-down' : 'ph:caret-up'} />
      </button>
      {#if !aboutCommunityCollapsed}
        <div class="section-content">
          <p class="byline about-byline-embedded">
            Brought to you by <a href="https://github.com/afaraha8403" target="_blank" rel="noopener noreferrer">Ali Farahat</a>
            at <a href="https://balacode.io" target="_blank" rel="noopener noreferrer">Balacode.io</a>.
          </p>
          <div class="action-grid two-col about-action-grid-embedded">
            <a href={GITHUB_REPO_URL} target="_blank" rel="noopener noreferrer" class="action-link">
              <Icon icon="ph:github-logo-duotone" />
              <span>GitHub Repository</span>
            </a>
            <a href="https://github.com/sponsors/afaraha8403" target="_blank" rel="noopener noreferrer" class="action-link highlight-link">
              <Icon icon="ph:heart-straight-duotone" />
              <span>Sponsor on GitHub</span>
            </a>
          </div>
          <div class="setting-row about-commercial-row">
            <div class="setting-label">
              <span class="setting-name">Commercial license</span>
              <span class="setting-desc">Licensing for businesses and resale</span>
            </div>
            <div class="setting-control">
              <a
                href="https://kalam.stream/business.html"
                target="_blank"
                rel="noopener noreferrer"
                class="btn-check about-commercial-link"
              >
                <Icon icon="ph:briefcase-duotone" />
                Open
                <Icon icon="ph:arrow-up-right-duotone" class="about-external-inline" />
              </a>
            </div>
          </div>
        </div>
      {/if}
    </section>

    <section class="settings-section" class:collapsed={!licenseOpen}>
      <button type="button" class="section-header" on:click={() => (licenseOpen = !licenseOpen)} aria-expanded={licenseOpen}>
        <h3>License</h3>
        <Icon icon={licenseOpen ? 'ph:caret-up' : 'ph:caret-down'} />
      </button>
      {#if licenseOpen}
        <div class="section-content">
          <pre class="license-text">{LICENSE_TEXT}</pre>
        </div>
      {/if}
    </section>

    <section class="settings-section" class:collapsed={!legalOpen}>
      <button type="button" class="section-header" on:click={() => (legalOpen = !legalOpen)} aria-expanded={legalOpen}>
        <h3>Legal</h3>
        <Icon icon={legalOpen ? 'ph:caret-up' : 'ph:caret-down'} />
      </button>
      {#if legalOpen}
        <div class="section-content">
          <div class="legal-links">
            <a href="https://kalam.stream/terms.html" target="_blank" rel="noopener noreferrer" class="legal-link">
              <Icon icon="ph:file-text-duotone" />
              <span>Terms &amp; Conditions</span>
            </a>
            <a href="https://kalam.stream/privacy.html" target="_blank" rel="noopener noreferrer" class="legal-link">
              <Icon icon="ph:lock-key-duotone" />
              <span>Privacy Policy</span>
            </a>
          </div>
        </div>
      {/if}
    </section>
  </div>
{:else}
<div class="about-container">
  <!-- Updates Section -->
  <section class="about-header animate-in" style="--delay: 0.1s">
    <div class="setting-row">
      <div class="setting-label">
        <span class="setting-name">Current version</span>
        <span class="setting-desc">{appVersion || '…'}</span>
      </div>
      <div class="setting-control">
        <button type="button" class="btn-check" disabled={updateChecking} on:click={checkUpdates}>
          {#if updateChecking}
            <Icon icon="ph:spinner-gap-duotone" class="spin" /> Checking…
          {:else}
            <Icon icon="ph:magnifying-glass-duotone" /> Check now
          {/if}
        </button>
      </div>
    </div>

    <div class="setting-row">
      <div class="setting-label">
        <span class="setting-name">Channel</span>
        <span class="setting-desc">Stable or Beta releases</span>
      </div>
      <div class="setting-control">
        <select
          class="form-select"
          bind:value={updateChannel}
          on:change={onChannelChange}
          aria-label="Update channel"
        >
          <option value="stable">Stable</option>
          <option value="beta">Beta</option>
        </select>
      </div>
    </div>

    <!-- Status messages -->
    {#if updateStatus === 'up-to-date'}
      <div class="status-msg success standalone-status"><Icon icon="ph:check-circle-duotone" /> Up to date</div>
      {#if updateStagedMessage}
        <div class="status-msg success staged-hint"><Icon icon="ph:info-duotone" /> {updateStagedMessage}</div>
      {/if}
    {:else if updateStatus === 'available'}
      <div class="status-msg available standalone-status"><Icon icon="ph:sparkle-duotone" /> Update {updateVersion} available!</div>
      <div class="update-install-actions update-install-actions--split standalone-actions">
        <button
          type="button"
          class="btn-install"
          disabled={updateInstalling}
          on:click={() => downloadAndInstall(true)}
        >
          {#if updateInstalling && !updateInstallingDeferred}
            {#if updateDownloadPercent != null}
              <Icon icon="ph:spinner-gap-duotone" class="spin" /> Updating… {updateDownloadPercent}%
            {:else}
              <Icon icon="ph:spinner-gap-duotone" class="spin" /> Downloading &amp; installing…
            {/if}
          {:else}
            <Icon icon="ph:arrow-clockwise-duotone" /> Update now
          {/if}
        </button>
        <button
          type="button"
          class="btn-install-secondary"
          disabled={updateInstalling}
          on:click={() => downloadAndInstall(false)}
          title="Download and install in the background; apply when you quit and open Kalam again"
        >
          {#if updateInstalling && updateInstallingDeferred}
            {#if updateDownloadPercent != null}
              <Icon icon="ph:spinner-gap-duotone" class="spin" /> Downloading… {updateDownloadPercent}%
            {:else}
              <Icon icon="ph:spinner-gap-duotone" class="spin" /> Preparing…
            {/if}
          {:else}
            <Icon icon="ph:moon-stars-duotone" /> Update on next start
          {/if}
        </button>
      </div>
    {:else if updateStatus === 'error'}
      <div class="status-msg error standalone-status"><Icon icon="ph:warning-circle-duotone" /> {updateError}</div>
    {/if}
  </section>

  <!-- Community & Support Section -->
  <section class="about-community-section animate-in" style="--delay: 0.2s">
    <div class="about-section-header standalone-header">
      <Icon icon="ph:users-three-duotone" />
      <h3>Community &amp; Support</h3>
    </div>
    <div class="section-content standalone-content">
      <p class="byline">
        Brought to you by <a href="https://github.com/afaraha8403" target="_blank" rel="noopener noreferrer">Ali Farahat</a>
        at <a href="https://balacode.io" target="_blank" rel="noopener noreferrer">Balacode.io</a>.
      </p>
      <div class="action-grid two-col">
        <a href={GITHUB_REPO_URL} target="_blank" rel="noopener noreferrer" class="action-link">
          <Icon icon="ph:github-logo-duotone" />
          <span>GitHub Repository</span>
        </a>
        <a href="https://github.com/sponsors/afaraha8403" target="_blank" rel="noopener noreferrer" class="action-link highlight-link">
          <Icon icon="ph:heart-straight-duotone" />
          <span>Sponsor on GitHub</span>
        </a>
      </div>
    </div>
  </section>

  <!-- Commercial License Button -->
  <a href="https://kalam.stream/business.html" target="_blank" rel="noopener noreferrer" class="commercial-license-btn animate-in" style="--delay: 0.3s">
    <span class="commercial-license-content">
      <Icon icon="ph:briefcase-duotone" />
      <span>Commercial License</span>
    </span>
    <Icon icon="ph:arrow-up-right-duotone" class="external-icon" />
  </a>

  <!-- License Section -->
  <section class="license-section animate-in" style="--delay: 0.4s">
    <button type="button" class="accordion" class:open={licenseOpen} on:click={() => (licenseOpen = !licenseOpen)} aria-expanded={licenseOpen}>
      <span class="accordion-title"><Icon icon="ph:scroll-duotone" /> License</span>
      <Icon icon="ph:caret-down-bold" class="accordion-caret" />
    </button>
    {#if licenseOpen}
      <div class="license-content">
        <pre class="license-text">{LICENSE_TEXT}</pre>
      </div>
    {/if}
  </section>

  <!-- Legal Section -->
  <section class="legal-section animate-in" style="--delay: 0.5s">
    <button type="button" class="accordion" class:open={legalOpen} on:click={() => (legalOpen = !legalOpen)} aria-expanded={legalOpen}>
      <span class="accordion-title"><Icon icon="ph:shield-check-duotone" /> Legal</span>
      <Icon icon="ph:caret-down-bold" class="accordion-caret" />
    </button>
    {#if legalOpen}
      <div class="legal-content">
        <div class="legal-links">
          <a href="https://kalam.stream/terms.html" target="_blank" rel="noopener noreferrer" class="legal-link">
            <Icon icon="ph:file-text-duotone" />
            <span>Terms &amp; Conditions</span>
          </a>
          <a href="https://kalam.stream/privacy.html" target="_blank" rel="noopener noreferrer" class="legal-link">
            <Icon icon="ph:lock-key-duotone" />
            <span>Privacy Policy</span>
          </a>
        </div>
      </div>
    {/if}
  </section>
</div>
{/if}

<style>
  .about-container {
    max-width: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-bottom: 24px;
  }

  .animate-in {
    opacity: 0;
    animation: slideUp 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
    animation-delay: var(--delay, 0s);
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .about-header {
    display: flex;
    flex-direction: column;
    padding: var(--space-md) var(--space-lg) var(--space-lg);
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
  }

  .about-header .setting-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .standalone-status {
    margin: var(--space-md) 0 0;
  }

  .standalone-actions {
    margin: var(--space-md) 0 0;
  }

  .version-label {
    font-size: 15px;
    color: var(--text-secondary);
  }

  .version-label strong {
    color: var(--navy-deep);
    font-weight: 600;
    font-family: 'Google Sans', ui-monospace, monospace;
  }

  .about-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 20px;
  }

  .about-card {
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-subtle);
    padding: 28px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: var(--shadow-sm);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
    position: relative;
    overflow: hidden;
  }

  .about-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-md);
  }

  .about-card.highlight {
    background: linear-gradient(to bottom right, var(--bg-card), var(--primary-alpha-light));
    border-color: var(--primary-alpha);
  }

  .card-icon {
    width: 40px;
    height: 40px;
    background: var(--bg-input);
    color: var(--navy-deep);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    margin-bottom: 4px;
  }

  .highlight .card-icon {
    background: var(--primary);
    color: var(--white);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  /* Settings → About (embedded): spacing comes from global .settings-section margins + .section-content padding */
  .about-content {
    padding: 0;
  }

  .about-content .section-content > .about-byline-embedded {
    margin: 0 0 var(--space-md);
  }

  /* Primary action link row sits above the commercial license setting-row */
  .about-content .section-content > .about-action-grid-embedded {
    margin-bottom: 0;
  }

  .about-commercial-link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    text-decoration: none;
  }

  .about-commercial-link :global(.about-external-inline) {
    font-size: 1em;
    opacity: 0.85;
  }

  .about-status {
    margin: var(--space-md) 0 0;
  }

  .about-actions {
    margin: var(--space-md) 0 0;
  }

  /* Standalone About page: card-style community header + body */
  .about-section-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-subtle);
  }

  .about-section-header :global(svg) {
    font-size: 20px;
    color: var(--primary);
  }

  .about-section-header h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }

  .standalone-header {
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    border: 1px solid var(--border-subtle);
    border-bottom: none;
  }

  .standalone-content {
    background: var(--bg-card);
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
    border: 1px solid var(--border-subtle);
    border-top: none;
    box-shadow: var(--shadow-sm);
  }

  /* Commercial License Button */
  .commercial-license-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: var(--bg-card);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    text-decoration: none;
    font-size: 15px;
    font-weight: 600;
    transition: all 0.2s ease;
    box-shadow: var(--shadow-sm);
  }

  .commercial-license-btn:hover {
    background: var(--bg-input);
    border-color: var(--border-visible);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .commercial-license-content {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .commercial-license-content :global(svg) {
    font-size: 20px;
    color: var(--primary);
  }

  .external-icon {
    font-size: 18px;
    color: var(--text-muted);
    transition: transform 0.2s ease;
  }

  .commercial-license-btn:hover .external-icon {
    transform: translate(2px, -2px);
    color: var(--primary);
  }

  /* Community & Support combined card */
  .community-support-card {
    grid-column: 1 / -1;
    background: linear-gradient(135deg, var(--bg-card) 0%, var(--primary-alpha-light) 100%);
    border-color: var(--primary-alpha);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 4px;
  }

  .card-header .card-icon {
    width: 44px;
    height: 44px;
    font-size: 24px;
    background: var(--primary);
    color: var(--white);
    box-shadow: 0 4px 14px var(--primary-alpha);
  }

  .card-header h3 {
    font-size: 20px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .action-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
    margin-top: auto;
  }

  .action-grid.two-col {
    grid-template-columns: repeat(2, 1fr);
  }

  .action-link.highlight-link {
    background: linear-gradient(135deg, var(--primary-alpha-light), var(--bg-card));
    border-color: var(--primary-alpha);
    color: var(--primary-dark);
  }

  .action-link.highlight-link:hover {
    background: var(--primary-alpha);
    border-color: var(--primary);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .about-card h3 {
    font-size: 18px;
    font-weight: 700;
    color: var(--navy-deep);
    margin: 0;
  }

  .byline, .card-text {
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-secondary);
    margin: 0;
    flex-grow: 1;
  }

  .byline a {
    color: var(--primary-dark);
    font-weight: 600;
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.2s;
  }

  .byline a:hover {
    border-color: var(--primary-dark);
  }

  .action-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
    color: var(--navy-deep);
    text-decoration: none;
    padding: 10px 16px;
    background: var(--bg-input);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
    border: 1px solid transparent;
    width: 100%;
  }

  .action-link:hover {
    background: var(--bg-card);
    border-color: var(--border-visible);
    box-shadow: var(--shadow-sm);
  }

  .action-link.secondary {
    background: transparent;
    border-color: var(--border-subtle);
  }

  .action-link.secondary:hover {
    background: var(--bg-input);
    border-color: var(--border-visible);
  }

  .action-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: auto;
  }

  .btn-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--primary);
    color: var(--white);
    font-size: 14px;
    font-weight: 600;
    border-radius: var(--radius-md);
    text-decoration: none;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px var(--primary-alpha);
    width: 100%;
  }

  .btn-primary:hover {
    background: var(--primary-dark);
    transform: translateY(-2px);
    box-shadow: 0 6px 16px var(--primary-alpha);
  }

  /* Update Controls */
  .update-controls {
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--bg-input);
    padding: 16px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    margin-top: auto;
  }

  .channel-selector {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .channel-selector label {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .channel-select.channel-select,
  .channel-selector .channel-select {
    width: 100%;
    padding: 14px 16px;
    padding-right: 48px;
    font-size: 15px;
    font-weight: 500;
    background: var(--bg-input);
    border: 2px solid transparent;
    border-radius: var(--radius-md);
    color: var(--text-primary);
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    box-shadow: var(--shadow-inner);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='%2364748B' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 16px center;
    background-size: 16px;
    transition: all 0.2s ease;
  }

  .channel-select:focus {
    outline: none;
    background-color: var(--bg-card);
    border-color: var(--primary);
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .channel-select:hover {
    background-color: var(--bg-input-hover, var(--bg-card));
  }

  .btn-check {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 10px;
    background: var(--bg-card);
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    color: var(--navy-deep);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-check:hover:not(:disabled) {
    background: var(--primary-alpha-light);
    border-color: var(--primary-alpha);
    color: var(--primary-dark);
  }

  .btn-check:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Embedded About: control-column link should not stretch like full-width “Check now” */
  .about-commercial-link.btn-check {
    width: auto;
    white-space: nowrap;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .status-msg {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    margin-top: 4px;
  }

  .status-msg.success {
    background: rgba(16, 185, 129, 0.1);
    color: var(--success, #10b981);
  }

  .status-msg.available {
    background: var(--primary-alpha-light);
    color: var(--primary-dark);
    border: 1px solid var(--primary-alpha);
  }

  .update-install-actions {
    margin-top: 10px;
  }

  .update-install-actions--split {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .staged-hint {
    margin-top: 8px;
    font-size: 12px;
    line-height: 1.45;
  }

  .btn-install-secondary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-input);
    color: var(--navy-deep);
    border: 1px solid var(--border-visible);
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-install-secondary:hover:not(:disabled) {
    background: var(--primary-alpha-light);
    border-color: var(--primary-alpha);
    color: var(--primary-dark);
  }

  .btn-install-secondary:disabled {
    opacity: 0.9;
    cursor: wait;
  }

  .btn-install {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 12px 16px;
    background: var(--primary);
    color: var(--white);
    border: none;
    border-radius: var(--radius-md);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 4px 12px var(--primary-alpha);
  }

  .btn-install:hover:not(:disabled) {
    background: var(--primary-dark);
    transform: translateY(-2px);
    box-shadow: 0 6px 16px var(--primary-alpha);
  }

  .btn-install:disabled {
    opacity: 0.9;
    cursor: wait;
  }

  .status-msg.error {
    background: rgba(239, 68, 68, 0.1);
    color: var(--error, #ef4444);
  }

  /* Legal Section */
  .legal-section {
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-subtle);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  .legal-content {
    border-top: 1px solid var(--border-subtle);
    padding: 20px 24px;
    background: var(--bg-input);
  }

  .legal-links {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }

  .legal-link {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    text-decoration: none;
    padding: 14px 16px;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    transition: all 0.2s ease;
  }

  .legal-link:hover {
    background: var(--bg-input);
    border-color: var(--border-visible);
    box-shadow: var(--shadow-sm);
  }

  .legal-link :global(svg) {
    font-size: 18px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  /* License Section */
  .license-section {
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-subtle);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
  }

  .accordion {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 0.2s ease;
  }

  .accordion:hover {
    background: var(--bg-input);
  }

  .accordion-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    color: var(--navy-deep);
  }

  .accordion-caret {
    font-size: 16px;
    color: var(--text-muted);
    transition: transform 0.3s ease;
  }

  .accordion.open .accordion-caret {
    transform: rotate(180deg);
  }

  .license-content {
    border-top: 1px solid var(--border-subtle);
    padding: 24px;
    background: var(--bg-input);
  }

  .license-text {
    margin: 0;
    font-family: 'Google Sans', ui-monospace, monospace;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 400px;
    overflow-y: auto;
    padding-right: 12px;
  }

  .license-text::-webkit-scrollbar {
    width: 6px;
  }
  .license-text::-webkit-scrollbar-track {
    background: transparent;
  }
  .license-text::-webkit-scrollbar-thumb {
    background: var(--border-visible);
    border-radius: 3px;
  }

  @media (max-width: 640px) {
    .about-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 16px;
    }

    .about-grid {
      grid-template-columns: 1fr;
    }

    .action-grid {
      grid-template-columns: 1fr;
    }

    .legal-links {
      grid-template-columns: 1fr;
    }

    .card-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 8px;
    }
  }
</style>
