# **Architecting an Open-Source, Cross-Platform Voice Dictation Ecosystem**

## **1\. Executive Summary and Industry Context**

The paradigm of human-computer interaction is undergoing a profound transformation, moving away from traditional peripheral input toward voice-first interfaces. Applications operating within this "Voice OS" paradigm, such as Wispr Flow, have demonstrated the immense productivity benefits of universal voice dictation.1 By allowing users to seamlessly transcribe spoken language into any active application via system-wide hotkeys, these tools circumvent the friction of traditional keyboard input, achieving dictation speeds up to four times faster than typing while automatically handling formatting, punctuation, and filler-word removal.1

Despite these advancements, the current landscape of voice dictation utilities is dominated by proprietary solutions that exhibit significant architectural, operational, and ethical limitations. Foremost among these is a near-total reliance on cloud infrastructure.4 Because audio processing occurs on third-party servers managed by entities like OpenAI or Meta, severe data privacy and compliance concerns arise, particularly for enterprise users subject to stringent regulations like HIPAA or GDPR.1 Furthermore, proprietary tools are frequently characterized by exorbitant resource consumption; users regularly report idle Random Access Memory (RAM) usage exceeding 800MB, accompanied by initialization delays of up to ten seconds.4 These performance bottlenecks directly undermine the rapid, frictionless workflows the tools purport to enable. Finally, cloud dependency renders these applications entirely non-functional in offline environments or areas with unreliable internet connectivity.4

This research report provides an exhaustive, expert-level architectural blueprint for constructing an open-source, cross-platform alternative. The proposed application will operate natively on Windows, macOS, and Linux, running unobtrusively in the background via the system tray.5 It leverages the Groq Application Programming Interface (API) for ultra-low-latency cloud transcription, while simultaneously offering users the option to utilize highly efficient, open-source local models—specifically advanced Chinese foundation models like SenseVoice—for privacy-preserving, offline execution.7

The system is designed to trigger transcription via complex global hotkeys (e.g., Control \+ Windows key), requiring sophisticated operating system-level event hooking to prevent default behaviors like the Windows Start Menu from interrupting the user experience.4 To ensure long-term maintainability, community contribution, and broad distribution, the architecture details a highly automated Continuous Integration and Continuous Deployment (CI/CD) pipeline utilizing GitHub Actions.11 This pipeline works in tandem with an Over-The-Air (OTA) update mechanism orchestrated entirely through static file hosting on GitHub Pages, completely eliminating the need for dedicated release servers.13

## **2\. Foundational Technology Stack and Desktop Framework Selection**

To achieve a cross-platform application that runs securely in the background with minimal resource overhead, the selection of the foundational desktop framework is the most critical architectural decision. The application must support Windows, macOS, and Linux natively, maintain a near-zero idle footprint, and possess deep, low-level access to the host operating system to manage global keyboard hooks and audio capture devices.

### **2.1 Evaluation of Cross-Platform Frameworks**

The modern development landscape offers several cross-platform frameworks, each with distinct architectural philosophies and performance profiles.

Electron has historically served as the de facto standard for building cross-platform desktop applications using web technologies.6 Electron bundles a complete instance of the Chromium web browser and the Node.js runtime into every compiled application.16 While this guarantees consistent rendering across all platforms, it inherently results in massive application binaries—often exceeding 100MB even for trivial applications—and severe memory bloat.15 Background idle RAM consumption for Electron applications frequently exceeds 120MB, making it highly unsuitable for a background dictation daemon designed to run continuously.15

Flutter, developed by Google, utilizes the Dart programming language and the Skia (or Impeller) graphics engine to render native-feeling interfaces.6 While highly performant for mobile applications and increasingly viable for desktop, Flutter's ecosystem for low-level desktop system hooks—such as global hotkey interception across X11, Wayland, and Windows APIs—remains less mature than systems-level languages.6

React Native relies on a JavaScript bridge to communicate with native components.6 While Microsoft maintains a robust fork for Windows and macOS, Linux support remains outside the official tree, violating the core requirement for universal cross-platform support.17 Similarly,.NET MAUI (Multi-platform App UI) offers excellent integration with the Windows ecosystem via C\#, but entirely lacks native Linux deployment capabilities, rendering it incompatible with the project's goals.17

### **2.2 The Architectural Superiority of Tauri**

The optimal choice for this architecture is Tauri (specifically the v2 release).15 Tauri allows developers to construct lightweight, highly performant binaries by strictly separating the frontend user interface from the backend logic.15

The backend is written in Rust, a systems-level programming language that provides memory safety without a garbage collector, advanced multi-threading capabilities, and direct, zero-cost abstractions over native operating system APIs.15 The frontend can be constructed using any standard web technology (e.g., React, Solid.js, or vanilla TypeScript), but crucially, it is rendered using the host operating system's native WebView.16 On Windows, this utilizes WebView2 (Edge/Chromium); on macOS, WebKit (Safari); and on Linux, WebKitGTK.15

| Framework Metric | Electron | Tauri (Rust-based) | Architectural Impact |
| :---- | :---- | :---- | :---- |
| **Rendering Engine** | Bundled Chromium | OS Native WebView | Eliminates redundant browser instances, saving disk space.16 |
| **Backend Runtime** | Node.js | Rust | Provides deterministic performance and strict memory safety.16 |
| **Bundle Size** | 85MB \- 150MB | 3MB \- 15MB | Facilitates faster downloads and a negligible disk footprint.15 |
| **Memory (Idle)** | High (120MB+) | Minimal (10MB \- 30MB) | Crucial for an "always-on" system tray application.15 |
| **Security Model** | Node integration risks | Strict IPC, Capability scopes | Prevents arbitrary code execution via compromised frontends.15 |

This architecture yields profound benefits for a dictation utility. Tauri applications feature binary sizes exponentially smaller than Electron, initialize in a fraction of the time, and consume significantly less RAM, directly addressing the core complaints leveled against proprietary tools like Wispr Flow.4

Furthermore, Tauri's security model is inherently superior. Unlike Electron, which historically exposed powerful Node.js APIs directly to the frontend, Tauri relies on a strict Inter-Process Communication (IPC) bridge.16 System capabilities—such as network access, file system reads, or hotkey registration—must be explicitly allow-listed in a default.json capability file, drastically reducing the application's attack surface.15 This is paramount for an application that processes potentially sensitive spoken audio.

## **3\. Low-Level Operating System Integration**

An effective, invisible dictation utility must interface flawlessly with the host operating system. The application requires sophisticated management of background execution, global keyboard interception, and low-latency audio capture. Rust's robust package ecosystem provides the necessary crates to achieve this without resorting to fragile bridging code.

### **3.1 Background Execution and System Tray Orchestration**

The application must operate invisibly until invoked by the user. This requires suppressing the standard application window upon launch and establishing a persistent, interactive system tray icon.5

The system tray serves as the primary user interface for application management. Tauri v2 provides a native tray-icon feature that can be configured directly within the Rust backend.23 By initializing the application with a TrayIconBuilder, the software remains resident in memory.23 The tray icon will host a context menu constructed using MenuItemBuilder, allowing the user to access settings, switch between cloud (Groq) and local AI models, view transcription history, or terminate the daemon.5

A critical architectural consideration is lifecycle management. By default, desktop frameworks terminate the underlying process when all associated windows are closed.25 The Rust backend must explicitly override this behavior. The event loop must be configured to intercept the RunEvent::ExitRequested event; when triggered by a window closure (e.g., closing the settings panel), the backend must invoke api.prevent\_exit(), ensuring the daemon continues listening for hardware events in the background.25

### **3.2 Global Hotkey Implementation and OS Interception**

The core interaction model relies on a global hotkey—specifically requested as a combination involving the Control and Windows keys (e.g., Ctrl \+ Super)—to trigger audio recording globally, regardless of which application currently holds window focus.4

Tauri supports the official @tauri-apps/plugin-global-shortcut plugin, which bridges the JavaScript frontend to Rust's global hotkey listener.26 The key codes mapped for cross-platform usage dynamically translate the Windows key to the Super or Meta modifier depending on the host OS.27 The architecture must register this shortcut during the application's setup phase and cleanly unregister it during shutdown or suspension to prevent OS-level shortcut collisions.26

#### **The Windows Key Interception Challenge**

A highly specific and complex challenge arises exclusively on the Windows operating system when utilizing the Windows key (LWin or RWin) as part of a custom hotkey combination. By default, the Windows operating system registers the release of the Windows key as an instruction to open the Start Menu.10 If a user presses and holds Ctrl \+ Win to dictate, speaks their sentence, and then releases the keys, the transcription will complete, but the Start Menu will simultaneously open.10 This steals window focus from the target application, causing the injected transcription text to be typed into the Start Menu search bar instead of the user's intended document.

Standard JavaScript event prevention (e.g., event.preventDefault()) or Tauri's tauri-plugin-prevent-default are insufficient here, as they only apply to the WebView context and cannot override global operating system behaviors.10 To suppress the Start Menu while preserving the Windows key's utility as a modifier, the Rust backend must implement low-level Windows API hooks.

The most effective architectural approach is to utilize the windows-sys or winapi Rust crates to establish a WH\_KEYBOARD\_LL (Low-Level Keyboard Hook) directly in the OS event queue.10 When the application detects that the Windows key was pressed as part of the dictation hotkey, the Rust hook must inject a "dummy" keystroke into the OS immediately before the Windows key is physically released. The standard dummy key utilized in Win32 programming for this purpose is vkE8 (an unassigned virtual key code).10 By sending the sequence {Blind}{vkE8} via the API, the Windows operating system registers that a key was pressed *in combination* with the Windows key, thereby canceling the default behavior of opening the Start Menu upon release.10 This ensures seamless text injection into the active window.

### **3.3 Audio Capture and Hardware Interfaces**

High-fidelity, low-latency audio capture is a strict prerequisite for accurate speech-to-text processing. The application must interface directly with the hardware abstraction layers of macOS (CoreAudio), Windows (WASAPI), and Linux (ALSA/PulseAudio/JACK).32

The optimal Rust crate for this task is cpal (Cross-Platform Audio Library), a low-level library that provides a unified, safe API across all major operating systems.32

When the user depresses the global hotkey, the Rust backend will initialize an input stream using cpal. To avoid blocking the main application thread or causing buffer overruns, the audio callback must write to a lock-free concurrent data structure, such as a ring buffer (e.g., the ringbuf crate).32

A critical data transformation step is required. Modern microphones capture audio at high sample rates (e.g., 44.1kHz or 48kHz) in stereo or multi-channel formats. However, both Groq's cloud API and local whisper-style models operate exclusively on **16kHz, 16-bit mono PCM** audio.7 Sending raw, uncompressed 48kHz audio to the cloud wastes massive amounts of bandwidth and introduces severe latency. The audio processing pipeline must downsample and downmix the raw f32 PCM data retrieved from the microphone locally. Utilizing a Rust resampling crate like rubato guarantees that the audio payloads transmitted over the network or passed to the local inference engine are as small as mathematically possible.7

## **4\. Cloud Speech-to-Text (STT) Engine: Groq LPU Integration**

The core value proposition of the application is the rapid, accurate conversion of captured audio into text. The architecture supports a dual-engine approach. The primary engine relies on cloud-based inference for ultra-fast, zero-local-overhead transcription, utilizing the Groq API as explicitly requested.7

### **4.1 Groq API Mechanics and Model Selection**

Groq's proprietary Language Processing Unit (LPU) infrastructure provides state-of-the-art inference speeds that rival human interaction, making it ideal for real-time dictation workflows.7 The system will interface with Groq using the OpenAI-compatible /v1/audio/transcriptions endpoint.7

Model selection is configurable based on user priorities. The architecture defaults to the whisper-large-v3-turbo model.7 This model provides the optimal balance of speed, price ($0.04 per hour of audio), and multilingual accuracy.7 If a user requires extreme precision for highly technical jargon or heavy accents, the application allows falling back to the standard whisper-large-v3 model ($0.111 per hour), accepting a slight latency penalty for maximum accuracy.7

Integration with Groq from the Tauri backend can be facilitated through standard asynchronous HTTP requests using the reqwest crate, or via dedicated Rust wrappers like the groq-api-rust library to strictly type the request payloads.36

### **4.2 Pseudo-Streaming and Audio Chunking Mathematics**

A critical architectural constraint in modern STT integrations is that standard REST APIs do not support continuous, bidirectional WebSocket streaming for audio.7 Users dictating long paragraphs cannot reasonably wait until they finish speaking to receive the transcribed text; the application must provide intermediate, real-time results to maintain the illusion of seamless dictation.39

To achieve "pseudo-streaming" over a REST architecture, the Rust backend must implement sophisticated audio chunking and overlapping logic.7

1. **Voice Activity Detection (VAD):** The incoming 16kHz audio stream must be constantly monitored by a lightweight VAD algorithm (e.g., utilizing a Rust port of Silero VAD).40 The VAD analyzes the energy and frequency of the audio to distinguish human speech from background noise or silence.41  
2. **Semantic Chunking:** Instead of arbitrarily slicing audio every 3 seconds—which frequently cuts words in half—the system uses the VAD to slice the audio at natural conversational pauses.7 If a user speaks continuously, a hard fallback chunking limit (e.g., 5 seconds) is enforced to ensure the user receives visual feedback.  
3. **Overlap Implementation:** Even with VAD, slicing continuous speech risks data loss at the boundaries.7 To prevent this, consecutive chunks must share a mathematical overlap, typically configured at 10% to 20% of the chunk size (roughly 0.5 to 1 second).39  
   * *Mathematical Formula:* If Chunk ![][image1] spans from time ![][image2] to ![][image3], Chunk ![][image4] must capture audio from $$ to ![][image5].  
4. **Prompt Chaining for Context:** When submitting Chunk ![][image4] to the Groq API, the transcription result of Chunk ![][image1] must be passed into the API's prompt parameter.7 The Whisper model uses this preceding text to maintain contextual awareness. This ensures consistent spelling of domain-specific terms across chunk boundaries and prevents the model from hallucinating punctuation (like periods or capitalization) in the middle of a continuous sentence.7

As the transcriptions return asynchronously from Groq, the Rust backend utilizes operating system-level simulated keystrokes (via crates such as enigo or mouce) to instantly inject the text into the active user application, completely bypassing the clipboard.4

## **5\. Local, Privacy-Preserving STT: Chinese Foundation Models**

To address the severe privacy, security, and offline-capability limitations of existing cloud-only tools like Wispr Flow 4, the application mandates the inclusion of local, open-source model execution. While Whisper.cpp is a standard choice for local inference 34, recent advancements in open-source Chinese foundation models offer drastically superior performance, particularly regarding latency and Asian language support.45

### **5.1 The Advantage of SenseVoice-Small over Whisper**

The architecture will prioritize **SenseVoice-Small**, developed by Alibaba's Tongyi Lab (FunAudioLLM).8

The fundamental architectural difference between Whisper and SenseVoice lies in their decoding strategies. Whisper is an autoregressive model; it generates text token-by-token, heavily limiting its speed, especially on CPUs without dedicated AI accelerators.8 Conversely, SenseVoice-Small utilizes a non-autoregressive, encoder-only end-to-end architecture.8

This design choice yields extraordinary computational efficiency. SenseVoice-Small requires only 70 milliseconds to process 10 seconds of audio, making it up to 15 times faster than Whisper-Large and 5 times faster than Whisper-Small on identical hardware.8

| Model Feature | OpenAI Whisper-Small | Alibaba SenseVoice-Small | Architectural Impact |
| :---- | :---- | :---- | :---- |
| **Architecture** | Autoregressive Encoder-Decoder | Non-autoregressive Encoder-only | Massive latency reduction.47 |
| **Speed (10s audio)** | \~350ms+ | \~70ms | Enables near-instantaneous offline dictation.9 |
| **Multilingual Focus** | Western-centric | Pan-Asian \+ Western (50+ languages) | Outperforms Whisper in Mandarin, Cantonese, Japanese, Korean.8 |
| **Inherent Modalities** | ASR | ASR, VAD, AED, SER | Natively identifies emotion and audio events without external logic.8 |

### **5.2 Tauri Sidecar Integration via Sherpa-ONNX**

Executing heavy machine learning models directly within the main Rust application binary is architecturally flawed. It leads to unstable thread management, massive binary bloat (defeating Tauri's primary advantage), and cross-compilation nightmares. The sophisticated solution is to utilize the **Tauri Sidecar** architecture.22

A Sidecar is a standalone, isolated executable bundled alongside the Tauri application that can be spawned and managed as a child process.50

1. **Binary Generation:** A lightweight C++ inference server is compiled utilizing the sherpa-onnx runtime.52 Sherpa-ONNX is highly recommended over native PyTorch as it supports INT8 quantized versions of SenseVoice-Small out-of-the-box.52 This quantization reduces the model footprint to roughly 200MB while maintaining accuracy, and operates highly efficiently on standard CPUs or via CUDA on NVIDIA GPUs.52  
2. **Configuration and Bundling:** The inference executables—compiled for different target triples (e.g., x86\_64-pc-windows-msvc, aarch64-apple-darwin)—are placed in the src-tauri/binaries directory.53 The tauri.conf.json is updated to register these sidecars under the bundle \> externalBin array.50 The INT8 .onnx model weights are bundled using Tauri's resources configuration, ensuring they are packaged inside the installer.54  
3. **Process Orchestration:** When the user navigates to the UI and toggles "Local Mode" (disabling the Groq API), the Tauri Rust backend uses the Command::new\_sidecar("sensevoice-server").spawn() instruction to launch the local model into memory.50  
4. **Local Communication:** The Rust backend communicates with this local sidecar process via a local HTTP or WebSocket port (e.g., localhost:10095) or standard input/output pipes.22 The audio chunking logic remains identical to the Groq implementation, simply routing the network traffic internally.  
5. **Security Sandboxing:** The sidecar executable and its expected arguments must be explicitly whitelisted in the src-tauri/capabilities/default.json scope. This security policy prevents the application from arbitrarily executing malicious binaries, preserving system integrity.22

By abstracting the STT engine behind an internal trait interface, the application can effortlessly toggle between Groq and SenseVoice, providing the user with unparalleled flexibility and guaranteed functionality regardless of internet availability.

## **6\. Continuous Integration and Continuous Deployment (CI/CD)**

To manage an open-source project spanning three major operating systems, manual compilation and release management are entirely unfeasible. The architecture dictates a highly automated, zero-touch CI/CD pipeline leveraging **GitHub Actions** to build the application automatically whenever a release tag is pushed.11

### **6.1 Automated Matrix Builds**

The core workflow is configured to trigger upon the creation of a Git tag matching a semantic versioning pattern (e.g., v\*).11

The GitHub Actions YAML configuration defines a matrix strategy to compile the application across multiple ephemeral environments concurrently, drastically reducing overall build times 12:

* windows-latest targeting x86\_64-pc-windows-msvc (producing .exe and .msi)  
* macos-latest targeting x86\_64-apple-darwin (Intel) and aarch64-apple-darwin (Apple Silicon) (producing .dmg and .app)  
* ubuntu-22.04 targeting x86\_64-unknown-linux-gnu (producing .AppImage and .deb)

To support Linux ARM builds (e.g., for Raspberry Pi or ARM-based laptops), the workflow can utilize the recently introduced ubuntu-24.04-arm native runners, or fall back to pguyot/arm-runner-action for QEMU emulation.57

The pipeline relies on the official @tauri-apps/tauri-action to orchestrate the complex Rust compilation process.57 The pipeline executes the following sequence:

1. **Environment Provisioning:** Checks out the repository and installs OS-specific build dependencies (e.g., webkit2gtk, libasound2-dev for Linux).  
2. **Dependency Caching:** Provisions the Node.js frontend cache and the Rust cargo cache utilizing swatinem/rust-cache. This step is vital for open-source projects, as downloading and compiling hundreds of Rust crates from scratch can take over an hour; caching reduces subsequent builds to minutes.57  
3. **Frontend Compilation:** Executes the frontend package managers (e.g., npm install and npm run build) to generate the static web assets.  
4. **Backend Compilation:** Executes tauri build to compile the Rust binary, inject the frontend assets, bundle the SenseVoice sidecars and ONNX weights, and generate the final installer artifacts.11 Crucially, this step also generates the cryptographic .sig signatures required for OTA updates.11

### **6.2 Cryptographic Code Signing and OS Security**

Modern operating systems aggressively block or quarantine unsigned binaries via mechanisms like Windows SmartScreen or macOS Gatekeeper. To ensure end-users can install the application without alarming security warnings, the CI/CD pipeline must integrate automated code signing.11

* **macOS:** Requires an Apple Developer ID certificate. The CI/CD pipeline extracts a base64-encoded .p12 certificate from GitHub Secrets, creates a temporary isolated keychain on the macOS runner, imports the certificate with partition lists (allowing non-GUI access to the key), and executes the codesign utility.11 Furthermore, the resulting .dmg must be submitted to Apple's notary service during the build process to receive a valid notarization ticket.11  
* **Windows:** Requires an Authenticode certificate. This is integrated by passing the certificate details or Azure Key Vault credentials into the GitHub Actions environment, allowing the Windows runner to utilize signtool.exe to sign the .exe and .msi installers.11

## **7\. Serverless Over-The-Air (OTA) Updates via GitHub Pages**

A modern desktop application must possess the capability to update itself seamlessly to deploy bug fixes, API adjustments, and updated AI models. The architectural requirement strictly specifies avoiding external server hosting or maintenance costs. This necessitates a purely GitHub-driven infrastructure for both file hosting and update checks.

### **7.1 Tauri Updater Cryptography and Verification**

Tauri v2 includes a highly secure, built-in tauri-plugin-updater.59 The updater operates on strict cryptographic verification to prevent man-in-the-middle attacks from distributing malicious binaries.

1. **Key Generation:** The repository owner generates an Elliptic Curve Digital Signature Algorithm (ECDSA) keypair using the Tauri CLI (tauri signer generate).13  
2. **Public Key Distribution:** The generated public key is embedded directly into the source code within tauri.conf.json. This key is safe to be public and is compiled into the application binary.59  
3. **Private Key Security:** The private key is stored exclusively as an encrypted GitHub Secret (TAURI\_SIGNING\_PRIVATE\_KEY).13 During the GitHub Actions build step, the tauri-action utilizes this secret to generate a cryptographic .sig file for every compiled binary artifact.11

When the application launches, the frontend invokes checkForAppUpdates() via the plugin.13 The application queries a remote endpoint for a JSON manifest. If the manifest advertises a version higher than the local application, the user receives a native push notification prompting them to upgrade.13 Upon approval, the application downloads the binary, verifies the cryptographic signature against the embedded public key, installs the payload, and orchestrates a process restart.60

### **7.2 Automating the Manifest on GitHub Pages**

The critical component linking the compiled GitHub Releases to the Tauri updater is the latest.json manifest file.61 While traditional setups rely on custom backend servers (e.g., FastAPI) to dynamically construct this JSON 13, a completely serverless approach hosted on **GitHub Pages** fulfills the requirements optimally.14

#### **Table: Tauri Updater latest.json Schema**

| Field | Data Type | Description and Content |
| :---- | :---- | :---- |
| version | String | Semantic version identifier matching the GitHub tag (e.g., "v1.2.0").61 |
| notes | String | Markdown-formatted changelog or release notes.61 |
| pub\_date | Date String | ISO 8601 formatted publication timestamp.61 |
| platforms | Object Map | Nested objects defining the architecture targets (e.g., windows-x86\_64, darwin-aarch64).61 |
| signature | String | The raw contents of the .sig file generated by the CI pipeline for that specific platform.61 |
| url | URL String | The direct browser\_download\_url pointing to the binary artifact on GitHub Releases.61 |

#### **The GitHub Actions Automation Workflow**

To fully automate this process without human intervention, a secondary job is appended to the GitHub Actions release workflow. Once the tauri-action successfully completes the build and uploads the artifacts to a GitHub Draft Release 11, a custom shell script executes:

1. **Metadata Parsing:** The script utilizes the GitHub API (via curl and jq) to extract the direct asset download URLs (browser\_download\_url) and the raw string contents of the uploaded .sig files from the release.63  
2. **JSON Construction:** It dynamically constructs the latest.json file, populating the schema detailed above with the specific URLs and signatures for the macOS, Windows, and Linux binaries.61  
3. **Deployment to GitHub Pages:** The action then commits this newly generated latest.json file. Modern GitHub Actions workflows utilize the actions/upload-pages-artifact and actions/deploy-pages actions to deploy the JSON directly to the GitHub Pages environment.68 This is highly advantageous over committing the JSON to a gh-pages branch, as it prevents polluting the git commit history with automated bot commits.68

The tauri.conf.json updater endpoint is statically configured to point to the raw GitHub Pages URL (e.g., https://\[username\].github.io/\[repo\]/latest.json).62 This creates an infinitely scalable, completely free, and highly secure OTA update pipeline.

## **8\. Open-Source Ecosystem and Community Management Strategy**

To successfully transition this architecture into a thriving open-source project that invites public contribution, the repository must be structured to reduce friction for new developers.

The architecture inherently supports this by strictly separating concerns. Developers proficient in React/TypeScript can contribute to the frontend user interface and settings panels without needing to understand the Rust backend. Similarly, systems engineers can optimize the audio capture, VAD logic, or Sidecar execution in Rust without interacting with the web technologies.

To facilitate contribution, the repository must establish stringent documentation:

* **Architecture Decision Records (ADRs):** Documenting why Tauri was chosen over Electron, or why SenseVoice-Small is the default local model over Whisper.  
* **Developer Environment Setup:** Utilizing tools like GitHub Codespaces or DevContainers to allow contributors to spin up the required Rust and Node.js toolchains instantly.  
* **Issue Templating:** Ensuring bug reports provide necessary context regarding the host operating system, audio hardware, and whether the issue occurred during Groq cloud transcription or local SenseVoice execution.

## **9\. Conclusion**

The creation of a universal, cross-platform voice dictation application designed to rival proprietary solutions requires a meticulously orchestrated technology stack. By rejecting the heavily bloated Electron framework in favor of the Rust-powered **Tauri v2**, the application guarantees minimal resource consumption and discrete, always-ready system tray operation. This fundamentally addresses the primary architectural flaws of current market offerings.

The dual-engine transcription approach uniquely positions this project for massive adoption. Integrating **Groq's LPU API** guarantees ultra-fast transcription via sophisticated mathematical chunking and overlapping strategies, ensuring fluid user experiences. Concurrently, the integration of **SenseVoice-Small** via ONNX-powered **Tauri Sidecars** delivers unprecedented privacy, true offline capability, and superior accuracy for complex Asian and Western languages without compromising system stability.

Finally, the reliance on **GitHub Actions** for cross-platform matrix compilation, code signing, and artifact generation, combined with a completely serverless OTA update mechanism hosted on **GitHub Pages**, establishes an enterprise-grade CI/CD pipeline at zero operational cost. This architectural blueprint not only fulfills the complex technical requirements of global hotkey interception and continuous audio processing but also ensures the project remains highly accessible, easily maintainable, and highly attractive to the global open-source community.

#### **Works cited**

1. Wispr Flow | Effortless Voice Dictation, accessed March 3, 2026, [https://wisprflow.ai/](https://wisprflow.ai/)  
2. Wispr Flow: The Architecture of Voice-First Mobile Productivity, accessed March 3, 2026, [https://www.youtube.com/watch?v=dXrVWSybU4w](https://www.youtube.com/watch?v=dXrVWSybU4w)  
3. Features \- Wispr Flow, accessed March 3, 2026, [https://wisprflow.ai/features](https://wisprflow.ai/features)  
4. Wispr Flow Review: AI Voice Dictation Tool January 2026, accessed March 3, 2026, [https://willowvoice.com/blog/wispr-flow-review-voice-dictation](https://willowvoice.com/blog/wispr-flow-review-voice-dictation)  
5. Understanding the System Tray: From Concept to Tauri v2 ... \- Medium, accessed March 3, 2026, [https://medium.com/@sjobeiri/understanding-the-system-tray-from-concept-to-tauri-v2-implementation-252f278bb57c](https://medium.com/@sjobeiri/understanding-the-system-tray-from-concept-to-tauri-v2-implementation-252f278bb57c)  
6. Best Cross-Platform App Development Frameworks \- GetStream.io, accessed March 3, 2026, [https://getstream.io/blog/cross-platform-development-frameworks/](https://getstream.io/blog/cross-platform-development-frameworks/)  
7. Speech to Text \- GroqDocs \- Groq Console, accessed March 3, 2026, [https://console.groq.com/docs/speech-to-text](https://console.groq.com/docs/speech-to-text)  
8. FunAudioLLM/SenseVoice: Multilingual Voice Understanding Model, accessed March 3, 2026, [https://github.com/FunAudioLLM/SenseVoice](https://github.com/FunAudioLLM/SenseVoice)  
9. FunAudioLLM/SenseVoiceSmall \- Hugging Face, accessed March 3, 2026, [https://huggingface.co/FunAudioLLM/SenseVoiceSmall](https://huggingface.co/FunAudioLLM/SenseVoiceSmall)  
10. Disable Windows Start menu without disabling the Windows key in, accessed March 3, 2026, [https://superuser.com/questions/1767403/disable-windows-start-menu-without-disabling-the-windows-key-in-windows-10](https://superuser.com/questions/1767403/disable-windows-start-menu-without-disabling-the-windows-key-in-windows-10)  
11. Ship Your Tauri v2 App Like a Pro: GitHub Actions and Release, accessed March 3, 2026, [https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7)  
12. Tauri with GitHub Actions \- CrabNebula Docs, accessed March 3, 2026, [https://docs.crabnebula.dev/cloud/guides/publish-cloud-github/](https://docs.crabnebula.dev/cloud/guides/publish-cloud-github/)  
13. Tauri v2 updater \- Ratul's Blog, accessed March 3, 2026, [https://ratulmaharaj.com/posts/tauri-automatic-updates/](https://ratulmaharaj.com/posts/tauri-automatic-updates/)  
14. Linking to releases \- GitHub Docs, accessed March 3, 2026, [https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)  
15. Tauri VS. Electron \- Real world application, accessed March 3, 2026, [https://www.levminer.com/blog/tauri-vs-electron](https://www.levminer.com/blog/tauri-vs-electron)  
16. Tauri vs. Electron: The Ultimate Desktop Framework Comparison, accessed March 3, 2026, [https://peerlist.io/jagss/articles/tauri-vs-electron-a-deep-technical-comparison](https://peerlist.io/jagss/articles/tauri-vs-electron-a-deep-technical-comparison)  
17. 5 Best Cross Platform Frameworks for App Dev in 2026, accessed March 3, 2026, [https://platform.uno/articles/best-cross-platform-frameworks-2026/](https://platform.uno/articles/best-cross-platform-frameworks-2026/)  
18. Looking for Global-Hotkeys cross platform .net library (Windows, accessed March 3, 2026, [https://www.reddit.com/r/dotnet/comments/zu8hj1/looking\_for\_globalhotkeys\_cross\_platform\_net/](https://www.reddit.com/r/dotnet/comments/zu8hj1/looking_for_globalhotkeys_cross_platform_net/)  
19. 6 Cross-Platform Mobile App Development Frameworks Compared, accessed March 3, 2026, [https://leancode.co/blog/cross-platform-mobile-app-development-frameworks](https://leancode.co/blog/cross-platform-mobile-app-development-frameworks)  
20. Why I chose Tauri instead of Electron \- Aptabase, accessed March 3, 2026, [https://aptabase.com/blog/why-chose-to-build-on-tauri-instead-electron](https://aptabase.com/blog/why-chose-to-build-on-tauri-instead-electron)  
21. Electron vs. Tauri: Building desktop apps with web technologies, accessed March 3, 2026, [https://www.codecentric.de/en/knowledge-hub/blog/electron-tauri-building-desktop-apps-web-technologies](https://www.codecentric.de/en/knowledge-hub/blog/electron-tauri-building-desktop-apps-web-technologies)  
22. A Technical Blueprint for Local-First AI with Rust and Tauri \- Medium, accessed March 3, 2026, [https://medium.com/@Musbell008/a-technical-blueprint-for-local-first-ai-with-rust-and-tauri-b9211352bc0e](https://medium.com/@Musbell008/a-technical-blueprint-for-local-first-ai-with-rust-and-tauri-b9211352bc0e)  
23. System Tray | Tauri, accessed March 3, 2026, [https://v2.tauri.app/learn/system-tray/](https://v2.tauri.app/learn/system-tray/)  
24. Tauri（5）—— Tray Icon Implementation and Event Handling, accessed March 3, 2026, [https://dev.to/rain9/tauri5-tray-icon-implementation-and-event-handling-5d1e](https://dev.to/rain9/tauri5-tray-icon-implementation-and-event-handling-5d1e)  
25. Tauri GlobalShortcut · tauri-apps · Discussion \#9550 \- GitHub, accessed March 3, 2026, [https://github.com/tauri-apps/tauri/discussions/9550](https://github.com/tauri-apps/tauri/discussions/9550)  
26. Global Shortcut \- Tauri, accessed March 3, 2026, [https://v2.tauri.app/plugin/global-shortcut/](https://v2.tauri.app/plugin/global-shortcut/)  
27. tauri-plugin-context-menu \- crates.io: Rust Package Registry, accessed March 3, 2026, [https://crates.io/crates/tauri-plugin-context-menu/0.6.0](https://crates.io/crates/tauri-plugin-context-menu/0.6.0)  
28. global-hotkey | Skills Marketplace \- LobeHub, accessed March 3, 2026, [https://lobehub.com/fr/skills/johnlindquist-script-kit-next-global-hotkey](https://lobehub.com/fr/skills/johnlindquist-script-kit-next-global-hotkey)  
29. The ultimate guide to keyboard shortcuts in Windows 11 \- XDA, accessed March 3, 2026, [https://www.xda-developers.com/windows-11-keyboard-shortcuts/](https://www.xda-developers.com/windows-11-keyboard-shortcuts/)  
30. tauri\_plugin\_prevent\_default \- Rust \- Docs.rs, accessed March 3, 2026, [https://docs.rs/tauri-plugin-prevent-default](https://docs.rs/tauri-plugin-prevent-default)  
31. \[bug\] Global shortcuts not working in some games \#7318 \- GitHub, accessed March 3, 2026, [https://github.com/tauri-apps/tauri/issues/7318](https://github.com/tauri-apps/tauri/issues/7318)  
32. RustAudio/cpal: Cross-platform audio I/O library in pure Rust \- GitHub, accessed March 3, 2026, [https://github.com/RustAudio/cpal](https://github.com/RustAudio/cpal)  
33. Audio \- Lib.rs, accessed March 3, 2026, [https://lib.rs/multimedia/audio](https://lib.rs/multimedia/audio)  
34. Whisper-rs speech-to-text example \- GitHub, accessed March 3, 2026, [https://github.com/lmammino/whisper-rs-example](https://github.com/lmammino/whisper-rs-example)  
35. Groq Automatic Speech Recognition (ASR) API, accessed March 3, 2026, [https://groq.humain.ai/GroqDocs/Groq%20ASR%20Model%20Guide.pdf](https://groq.humain.ai/GroqDocs/Groq%20ASR%20Model%20Guide.pdf)  
36. groq-api-rust \- Crates.io, accessed March 3, 2026, [https://crates.io/crates/groq-api-rust](https://crates.io/crates/groq-api-rust)  
37. Groq API Rust Client Library \- Crates.io, accessed March 3, 2026, [https://crates.io/crates/groq-api-rust/0.1.0](https://crates.io/crates/groq-api-rust/0.1.0)  
38. Chunking Longer Audio Files for Whisper Models on Groq, accessed March 3, 2026, [https://community.groq.com/t/chunking-longer-audio-files-for-whisper-models-on-groq/162](https://community.groq.com/t/chunking-longer-audio-files-for-whisper-models-on-groq/162)  
39. Chunking in AI: From Documents to Audio — The Hidden Key to, accessed March 3, 2026, [https://saicharankummetha.medium.com/chunking-in-ai-from-documents-to-audio-the-hidden-key-to-accuracy-44385e12f7b3](https://saicharankummetha.medium.com/chunking-in-ai-from-documents-to-audio-the-hidden-key-to-accuracy-44385e12f7b3)  
40. modelscope/FunASR: A Fundamental End-to-End Speech ... \- GitHub, accessed March 3, 2026, [https://github.com/modelscope/FunASR](https://github.com/modelscope/FunASR)  
41. Groq (Whisper) \- Pipecat, accessed March 3, 2026, [https://docs.pipecat.ai/server/services/stt/groq](https://docs.pipecat.ai/server/services/stt/groq)  
42. What is Chunk Size and Chunk Overlap \- DEV Community, accessed March 3, 2026, [https://dev.to/tak089/what-is-chunk-size-and-chunk-overlap-1hlj](https://dev.to/tak089/what-is-chunk-size-and-chunk-overlap-1hlj)  
43. How to Create Overlap Strategies \- OneUptime, accessed March 3, 2026, [https://oneuptime.com/blog/post/2026-01-30-rag-overlap-strategies/view](https://oneuptime.com/blog/post/2026-01-30-rag-overlap-strategies/view)  
44. How to process audio with Whisper in Rust \- tauri \- Stack Overflow, accessed March 3, 2026, [https://stackoverflow.com/questions/78530532/how-to-process-audio-with-whisper-in-rust](https://stackoverflow.com/questions/78530532/how-to-process-audio-with-whisper-in-rust)  
45. FireRedTeam/FireRedASR: Open-source industrial-grade ASR, accessed March 3, 2026, [https://github.com/FireRedTeam/FireRedASR](https://github.com/FireRedTeam/FireRedASR)  
46. accessed December 31, 1969, [https://github.com/modelscope/FunASR/blob/main/docs/model\_zoo/model\_introduction.md](https://github.com/modelscope/FunASR/blob/main/docs/model_zoo/model_introduction.md)  
47. Voice Understanding and Generation Foundation Models for Natural, accessed March 3, 2026, [https://fun-audio-llm.github.io/pdf/FunAudioLLM.pdf](https://fun-audio-llm.github.io/pdf/FunAudioLLM.pdf)  
48. The Latest in Open Source AI from Alibaba's Tongyi Lab \- Dev.to, accessed March 3, 2026, [https://dev.to/xidaisme/the-latest-in-open-source-ai-from-alibabas-tongyi-lab-funaudiollm-3ebd](https://dev.to/xidaisme/the-latest-in-open-source-ai-from-alibabas-tongyi-lab-funaudiollm-3ebd)  
49. Making desktop apps with revved-up potential: Rust \+ Tauri \+ sidecar, accessed March 3, 2026, [https://evilmartians.com/chronicles/making-desktop-apps-with-revved-up-potential-rust-tauri-sidecar](https://evilmartians.com/chronicles/making-desktop-apps-with-revved-up-potential-rust-tauri-sidecar)  
50. Sidecar \- The Tauri Documentation WIP, accessed March 3, 2026, [https://jonaskruckenberg.github.io/tauri-docs-wip/examples/sidecar.html](https://jonaskruckenberg.github.io/tauri-docs-wip/examples/sidecar.html)  
51. Building Local LM Desktop Applications with Tauri | by Dillon de Silva, accessed March 3, 2026, [https://medium.com/@dillon.desilva/building-local-lm-desktop-applications-with-tauri-f54c628b13d9](https://medium.com/@dillon.desilva/building-local-lm-desktop-applications-with-tauri-f54c628b13d9)  
52. Speech Recognition (ASR) | Open LLM Vtuber, accessed March 3, 2026, [http://docs.llmvtuber.com/en/docs/user-guide/backend/asr/](http://docs.llmvtuber.com/en/docs/user-guide/backend/asr/)  
53. Node.js as a sidecar \- Tauri, accessed March 3, 2026, [https://v2.tauri.app/learn/sidecar-nodejs/](https://v2.tauri.app/learn/sidecar-nodejs/)  
54. Embedding Additional Files \- Tauri, accessed March 3, 2026, [https://v2.tauri.app/develop/resources/](https://v2.tauri.app/develop/resources/)  
55. How to run a sidecar with args from rust? \#5379 \- tauri-apps \- GitHub, accessed March 3, 2026, [https://github.com/orgs/tauri-apps/discussions/5379](https://github.com/orgs/tauri-apps/discussions/5379)  
56. FunASR/runtime/docs/SDK\_tutorial.md at main \- GitHub, accessed March 3, 2026, [https://github.com/alibaba-damo-academy/FunASR/blob/main/runtime/docs/SDK\_tutorial.md](https://github.com/alibaba-damo-academy/FunASR/blob/main/runtime/docs/SDK_tutorial.md)  
57. GitHub \- Tauri, accessed March 3, 2026, [https://v2.tauri.app/distribute/pipelines/github/](https://v2.tauri.app/distribute/pipelines/github/)  
58. GitHub \- tauri-apps/tauri-action: Build your Web application as a, accessed March 3, 2026, [https://github.com/tauri-apps/tauri-action](https://github.com/tauri-apps/tauri-action)  
59. Updater \- Tauri, accessed March 3, 2026, [https://v2.tauri.app/plugin/updater/](https://v2.tauri.app/plugin/updater/)  
60. Tauri v2 with Auto-Updater \- CrabNebula Docs, accessed March 3, 2026, [https://docs.crabnebula.dev/cloud/guides/auto-updates-tauri/](https://docs.crabnebula.dev/cloud/guides/auto-updates-tauri/)  
61. How to make automatic updates work with Tauri v2 and GitHub, accessed March 3, 2026, [https://thatgurjot.com/til/tauri-auto-updater/](https://thatgurjot.com/til/tauri-auto-updater/)  
62. Tauri Action: Updater latest.json file \#6385 \- GitHub, accessed March 3, 2026, [https://github.com/tauri-apps/tauri/discussions/6385](https://github.com/tauri-apps/tauri/discussions/6385)  
63. Determine latest release asset download URL from site ... \- GitHub Gist, accessed March 3, 2026, [https://gist.github.com/dleidert/99a8e6ee3a879a7ed1f160c5dd07c13d](https://gist.github.com/dleidert/99a8e6ee3a879a7ed1f160c5dd07c13d)  
64. Auto-updating pages with GitHub Actions \- Jonathan Soma, accessed March 3, 2026, [https://jonathansoma.com/fancy-github/github-actions-dynamic-page/](https://jonathansoma.com/fancy-github/github-actions-dynamic-page/)  
65. How do I auto generate/update a json file of a github repository files, accessed March 3, 2026, [https://stackoverflow.com/questions/75192570/how-do-i-auto-generate-update-a-json-file-of-a-github-repository-files-during-co](https://stackoverflow.com/questions/75192570/how-do-i-auto-generate-update-a-json-file-of-a-github-repository-files-during-co)  
66. Is there a link to GitHub for downloading a file in the latest release of, accessed March 3, 2026, [https://stackoverflow.com/questions/24987542/is-there-a-link-to-github-for-downloading-a-file-in-the-latest-release-of-a-repo](https://stackoverflow.com/questions/24987542/is-there-a-link-to-github-for-downloading-a-file-in-the-latest-release-of-a-repo)  
67. Automating App Version Update With Github Actions and Slack, accessed March 3, 2026, [https://dev.to/zach0811/automating-app-version-update-with-github-actions-and-slack-5h49](https://dev.to/zach0811/automating-app-version-update-with-github-actions-and-slack-5h49)  
68. GitHub Action to publish artifacts to GitHub Pages for deployments, accessed March 3, 2026, [https://github.com/actions/deploy-pages](https://github.com/actions/deploy-pages)

[image1]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABIAAAAYCAYAAAD3Va0xAAABm0lEQVR4AeySyytFURTGzyXJIwPJqygyN2NoamRgIAMz/gGRgamJMkdRBgaUjGREMWVASsmjUCiPgXfe1+/DPmfv7dSt2x3cwb19v/uttfY5a5/9yAsy9Mv+RvmstPSPElw5FipBVARyLJK/tCaGxmAVHmEE7GbF5P2wCbswDa2Q8BvtU9SDS/gcdEMtGD0RjMI49EIfbEDSb0QtKOPvBCZBy+zE7a8qIK8GTYr9Kq5RFUNnsAPHoFnLcSM1vyS5g1BxjSoZPYcH0DLq8BYw0kSnJEkI5TdS3sjoDXzBGmxBFxSCpGVdKLDRi3auJejTtamq3/I3D+3QDHq+Hr8CRxqwCw0kB/AORuYq9FDQsmpwXQ0skt1IsfZiLxr+ibRfy0QdoC87xN/AkV42Bd1Y7YO//lcemAVdxiFcp+lsNLXAblRBQbO/4L62KSzCEegZzJUa6RQGKc/AMAyANh0L9UG0ACvwDP+kRtdUp0A3uA1XrNMidLRONgGxUqNPRnRL9bK4J9cdwhypZp+mM6hGTiHdJNco9c5lbI++AQAA//+mPFAsAAAABklEQVQDAItSSjG4Sgc/AAAAAElFTkSuQmCC>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAA8AAAAZCAYAAADuWXTMAAABg0lEQVR4AdyTOy9EQRiGz7rfEoJOJCJEQcQl/oBINEoKpUThB6j0CjqJSq1WiUpJRUEiCIVKgrhEIsTd8wy7OdnYPRvb2bzPvPPNfN+ZM3NmS6Iifv+kuJQjqIGCFN9ziophWII6SFS8uJzsMXiGV0hUvLiW7EHYhhdIlMVVZDVDB7jnU7wBnMNyy4QupqdhDtzrKD4BiQdn8T6Ji3AMmzAPK/AAeWWxCfU0fbAD2TLHLfUwUQkZOWHQRjMAro5lVEZvFjzMa3wGPCMsiiyWXqIrOAflwaXo9IN+gF/80I0HWVhBbwi24B5cbRz3FX1Vx96J1SeNC2FRZuV2okN4Ax90iXtZWvEUxNWYDlz5iWAZJmEVmsCLgkXVNNnFDH3LYl9lg3AEpmAd0tfzjP4HxOViIbY4dGgeITNB34da7MVJ5/kmJ8wFpQdD8Euzy5h77MRbwD/PHh6UVHxH1gKY50Vao38DQQ6GTp7GT3XEvF/jFnc7WBQ+Vej8pSlk5ZzPLar4CwAA///lSDreAAAABklEQVQDAGNjOTNkNKpWAAAAAElFTkSuQmCC>

[image3]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAA8AAAAZCAYAAADuWXTMAAABS0lEQVR4AdySTytEURyGr/9CUSwsJWUnEaXYSNlYUiyVT2GvZKesWCkfQVaWrKxsUKQsKHZK5F88T83cube5d87ULKZmep957znn987c8zunOarh0yDhFlrQBVUpuecmEvOwCz0QVDLcRvUSfMI3BJUMd1M9CefwBUEZ7qRqAEbAPd/ifeAali8LRlnegE1wr4v4CgQbZ/iSwh24gVPYgn14g4oybEEvX+NwAVlqZXIQUiqGh5idAP8di2UTZxkdwCqkZFjGmH2BR1A2TvfIbOAxA5+xkgy2M5yCM3gFX3EZ7wCP7LngWFqGZZjpK/gBf8iAl4Vhvgx+sLwHa3AE/eBFwSrL8B8lJ7AA65C5P+bLZLg4+c6Db4FVp2Q4K2HTZliYLjCHe3WxKAqFbeADlYewDfcQv10o/EvxE9zBNXgP4lMIhanPV/3C/wAAAP//iGddeAAAAAZJREFUAwDRWTQzTx0yTAAAAABJRU5ErkJggg==>

[image4]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAADMAAAAZCAYAAACclhZ6AAACl0lEQVR4AeyWuYsUQRSH2/sWFUUURVTwwhszI41EQQP/AEE0MfUIxMTIyMBUUBCPQAxEFMQjEBEEUTwwUREUvN1d2F32vr5vlh5qa5fZmWV6p1lm+H39qqqrq+rV8WomJxPoV3cmr4sZr4z5WQw2ZQrpWDMomAS5k4MPB7WLzFv4Bm1wHsKBzyd/BX5DM3yBIxC3Q1FmcjyLaX0/TIOi4kG84s0OuAjP4AAshVQ6cJTMGTgF1r2O7YNyNZWKQwZBvhzNo9IJuARO6HGsbWEGFTvTT7EV/mIvwCrYB2G9XvIz4QG0QCWOUD1Zx2MvVKouPngM5+A1DJuQcJC8L2gRzz/gB26jY6QXQiob6SGjw5hxUyc9fQZ3B2a4RnJmOdX+QRNchU2wGVLNJdEI3ZArxc54uFYywp/gdnqI/Q+HIK3rGXJ7UZQvpQNMR2XYXUHGVcEkRrVbJIwcrhjJZC0PncXkS7EzyxheK3SAMiDcJTEbDsMc2Aa/YDR5trZQyYgXsoEyJ2QnNizfTt77DTM2xc7YufdMGKE+0bQH7zR2I+hgOdtMZ6y/lW9CjGarKbOvsNxzWTVn7HwNnXyFUO1kLoMR7STWu8hoRrKkvHRvU+NaxD3yTyAuv0GZgQUzNoUr4xZye8UNuhJPaf49uMU+YHMpnRFnfQ8jNFKZtoxsUQ2kvHXfYQ3bmJpoOr16Dy7AekUswRq0CuP14cF7SaFR6yz2ObjXMUW5Oo/I3Yda3S8GoTv070Xu35rdpJ3cH1jPXqIzb8isBw+fXurIR/KxvlNwE2olz+BBOndl/DvlGXcX+afToFVwhvfjKgPMiyx6dGWyaLdUmwYZ77JSdSp+5we1cMZ+M6HuTCbTWoVG6ytThUnMpIkBAAAA//9OHlHoAAAABklEQVQDAO5NbDOOGAZLAAAAAElFTkSuQmCC>

[image5]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAA8AAAAZCAYAAADuWXTMAAABkElEQVR4AdyTyytFURTG9/HOuxgZISUpeaSUiVAmhiRlolCGpuZSZsoAExPlH5CRkUhRSgqFIWEgJO/i963bvZ1769x9687u6fvtb6991jr7cc7Jc1lcOVKczxGUQkYK7zmgoh+WoRy8ChcXkj0MX/ADXoWLy8juggP4Bq9UXEJWLTSB9nyFV4PuYdFSQjO3p2AetNchfBS8B6fiUxKX4BJ2YQHW4Q3SSsVKqKJph2NIVREDrdAC6mMxxYvrCTtBs2MJVdKbA61iAN8AHSzmnIpFG9Ej3IKkgwvo9MIf3MMaPEEfmFSopXQT7cMLFMAIFMMdxB+kd/9BXAcmFYtGonP4BT3oAf8EHeYirn4FriUf4iYV6mkrROOwCTWgDwVLSLNPEG2DJsGc7Vl72iEahElQgpZI1xTQ9sAJ6FXqW6DrrNg6NO+gVWBJUuEzI0egV9qAm7Rs60Q0HYyvgk56D98C/UBY8sw2kNLcEI/BNMzALJyByTfzK1n6cC5wcY3rl8X8M1tSVOObOarOxrMq/gcAAP//SylQzQAAAAZJREFUAwDkwD8zmd5gawAAAABJRU5ErkJggg==>