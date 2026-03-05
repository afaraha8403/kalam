<script lang="ts">
  import { onMount } from 'svelte'

  let step = 1
  const totalSteps = 5

  function nextStep() {
    if (step < totalSteps) step++
  }

  function prevStep() {
    if (step > 1) step--
  }

  function finish() {
    // Mark onboarding as complete and close
  }
</script>

<div class="onboarding">
  <div class="progress">
    {#each Array(totalSteps) as _, i}
      <div class="dot" class:active={i + 1 === step}></div>
    {/each}
  </div>

  {#if step === 1}
    <div class="step">
      <h1>Welcome to Kalam Voice</h1>
      <p class="subtitle">Speak your thoughts freely — fast, private, and accessible anywhere.</p>
      
      <div class="features">
        <div class="feature">
          <span class="icon">⚡</span>
          <div>
            <h3>4x Faster</h3>
            <p>Than typing</p>
          </div>
        </div>
        <div class="feature">
          <span class="icon">🔒</span>
          <div>
            <h3>Privacy First</h3>
            <p>Local mode available</p>
          </div>
        </div>
        <div class="feature">
          <span class="icon">🌍</span>
          <div>
            <h3>Cross-Platform</h3>
            <p>Windows, macOS, Linux</p>
          </div>
        </div>
      </div>
    </div>

  {:else if step === 2}
    <div class="step">
      <h1>Permissions</h1>
      <p class="subtitle">Kalam needs a few permissions to work properly</p>
      
      <div class="permissions">
        <div class="permission">
          <span class="icon">🎤</span>
          <div>
            <h3>Microphone Access</h3>
            <p>To capture your voice</p>
          </div>
        </div>
        <div class="permission">
          <span class="icon">⌨️</span>
          <div>
            <h3>Accessibility</h3>
            <p>To type text into any app</p>
          </div>
        </div>
      </div>

      <div class="test-mic">
        <p>Test your microphone:</p>
        <button class="btn-primary">Start Test</button>
      </div>
    </div>

  {:else if step === 3}
    <div class="step">
      <h1>Choose Your Mode</h1>
      <p class="subtitle">How would you like to transcribe?</p>
      
      <div class="modes">
        <div class="mode-card">
          <h3>☁️ Cloud Mode</h3>
          <p>Fastest transcription using Groq API</p>
          <ul>
            <li>~300ms latency</li>
            <li>99 languages</li>
            <li>Requires internet</li>
          </ul>
          <input type="text" placeholder="Enter Groq API key" />
          <a href="https://console.groq.com" target="_blank">Get API key →</a>
        </div>
        
        <div class="mode-card recommended">
          <div class="badge">Recommended</div>
          <h3>🔄 Hybrid Mode</h3>
          <p>Best of both worlds</p>
          <ul>
            <li>Cloud when available</li>
            <li>Local when offline</li>
            <li>Auto-switching</li>
          </ul>
          <button class="btn-primary">Select Hybrid</button>
        </div>
        
        <div class="mode-card">
          <h3>💻 Local Mode</h3>
          <p>100% private, works offline</p>
          <ul>
            <li>No internet needed</li>
            <li>50+ languages</li>
            <li>Download required</li>
          </ul>
          <button class="btn-secondary">Download Model</button>
        </div>
      </div>
    </div>

  {:else if step === 4}
    <div class="step">
      <h1>Set Your Hotkey</h1>
      <p class="subtitle">Choose how you'll activate dictation</p>
      
      <div class="hotkey-config">
        <div class="hotkey-display">
          <kbd>Ctrl</kbd> + <kbd>Win</kbd>
        </div>
        <p class="hint">Press your desired key combination</p>
        
        <div class="mode-select">
          <label>
            <input type="radio" name="mode" value="hold" checked />
            Hold to record (release to stop)
          </label>
          <label>
            <input type="radio" name="mode" value="toggle" />
            Toggle mode (press to start/stop)
          </label>
        </div>
      </div>
    </div>

  {:else if step === 5}
    <div class="step">
      <h1>You're Ready!</h1>
      <p class="subtitle">Try it out now</p>
      
      <div class="demo">
        <p>Press <kbd>Ctrl+Win</kbd> and say:</p>
        <blockquote>"Hello, this is a test of Kalam Voice!"</blockquote>
        <textarea placeholder="Your text will appear here..." readonly></textarea>
      </div>
    </div>
  {/if}

  <div class="actions">
    {#if step > 1}
      <button class="btn-secondary" on:click={prevStep}>Back</button>
    {/if}
    
    {#if step < totalSteps}
      <button class="btn-primary" on:click={nextStep}>Next</button>
    {:else}
      <button class="btn-primary" on:click={finish}>Get Started</button>
    {/if}
  </div>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    background: #1a1a1a;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px;
  }

  .progress {
    display: flex;
    gap: 8px;
    margin-bottom: 40px;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #333;
    transition: background 0.3s;
  }

  .dot.active {
    background: #4fc1ff;
  }

  .step {
    text-align: center;
    max-width: 800px;
  }

  h1 {
    font-size: 36px;
    margin-bottom: 12px;
  }

  .subtitle {
    font-size: 18px;
    color: #666;
    margin-bottom: 40px;
  }

  .features {
    display: flex;
    justify-content: center;
    gap: 40px;
  }

  .feature {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .feature .icon {
    font-size: 32px;
  }

  .feature h3 {
    font-size: 16px;
    margin-bottom: 4px;
  }

  .feature p {
    font-size: 14px;
    color: #666;
  }

  .permissions {
    display: flex;
    justify-content: center;
    gap: 40px;
    margin-bottom: 40px;
  }

  .permission {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 20px;
    background: #252525;
    border-radius: 12px;
  }

  .permission .icon {
    font-size: 32px;
  }

  .modes {
    display: flex;
    gap: 20px;
    justify-content: center;
  }

  .mode-card {
    width: 240px;
    padding: 24px;
    background: #252525;
    border-radius: 12px;
    text-align: left;
    position: relative;
  }

  .mode-card.recommended {
    border: 2px solid #4fc1ff;
  }

  .badge {
    position: absolute;
    top: -10px;
    left: 50%;
    transform: translateX(-50%);
    background: #4fc1ff;
    color: #1a1a1a;
    padding: 4px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 600;
  }

  .mode-card h3 {
    margin-bottom: 8px;
  }

  .mode-card p {
    color: #666;
    font-size: 14px;
    margin-bottom: 16px;
  }

  .mode-card ul {
    list-style: none;
    margin-bottom: 20px;
  }

  .mode-card li {
    padding: 4px 0;
    color: #999;
    font-size: 14px;
  }

  .mode-card li::before {
    content: "✓ ";
    color: #4caf50;
  }

  .mode-card input {
    width: 100%;
    padding: 10px;
    background: #333;
    border: 1px solid #444;
    border-radius: 6px;
    color: #e0e0e0;
    margin-bottom: 8px;
  }

  .mode-card a {
    color: #4fc1ff;
    font-size: 12px;
    text-decoration: none;
  }

  .hotkey-config {
    background: #252525;
    padding: 40px;
    border-radius: 12px;
  }

  .hotkey-display {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .hotkey-display kbd {
    background: #333;
    padding: 12px 20px;
    border-radius: 8px;
    font-family: monospace;
    font-size: 18px;
  }

  .mode-select {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 24px;
  }

  .mode-select label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .demo blockquote {
    font-size: 20px;
    font-style: italic;
    color: #999;
    margin: 20px 0;
  }

  .demo textarea {
    width: 100%;
    height: 100px;
    padding: 16px;
    background: #252525;
    border: 1px solid #333;
    border-radius: 8px;
    color: #e0e0e0;
    resize: none;
  }

  .actions {
    display: flex;
    gap: 16px;
    margin-top: 40px;
  }

  .btn-primary {
    padding: 14px 32px;
    background: #4fc1ff;
    border: none;
    border-radius: 8px;
    color: #1a1a1a;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-secondary {
    padding: 14px 32px;
    background: transparent;
    border: 1px solid #444;
    border-radius: 8px;
    color: #e0e0e0;
    font-size: 16px;
    cursor: pointer;
  }
</style>
