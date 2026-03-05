# Distribution Strategy Document

**Kalam Voice - Open Source Voice Dictation**  
**Version:** 1.0  
**Date:** March 4, 2026  
**Status:** Planning Phase

---

## Executive Summary

This document outlines the complete distribution strategy for Kalam Voice, an open-source, cross-platform voice dictation application. The strategy emphasizes zero-cost infrastructure, multi-channel distribution, and seamless user experience from discovery through updates.

---

## 1. Discovery Channels

### 1.1 Primary Discovery Channels

| Channel | Strategy | Timeline | Expected Impact |
|---------|----------|----------|-----------------|
| **GitHub** | Primary distribution hub, README optimization, topics/tags | Launch | Core discovery |
| **Official Website** | Landing page, feature showcase, download portal | Launch | Brand credibility |
| **Product Hunt** | Launch campaign, community engagement | Month 1 | Tech early adopters |
| **Hacker News** | Show HN post, technical deep-dives | Month 1 | Developer community |
| **Reddit** | r/productivity, r/rust, r/opensource, r/accessibility | Month 1-2 | Targeted communities |
| **YouTube** | Demo videos, tutorials, comparison content | Month 2 | Visual learners |
| **Blog/Content** | Technical blog posts, productivity guides | Ongoing | SEO & thought leadership |

### 1.2 Community & Ecosystem Discovery

- **Open Source Directories:**
  - Awesome lists (awesome-rust, awesome-productivity)
  - AlternativeTo.net listing
  - Productivity tool aggregators

- **Developer Communities:**
  - Rust community forums
  - Tauri ecosystem showcase
  - Voice AI/ML communities

- **Accessibility Networks:**
  - Assistive technology forums
  - Disability advocacy groups
  - Occupational therapy resources

### 1.3 Marketing Messaging

**Primary Value Propositions:**
1. **"4x faster than typing"** - Speed advantage
2. **"Your voice, your data, 100% open source"** - Privacy & transparency
3. **"30MB RAM vs 800MB competitors"** - Resource efficiency
4. **"Works everywhere - Windows, Mac, Linux"** - Cross-platform
5. **"Free forever, BYOK or support us"** - Business model transparency

---

## 2. Installation Methods

### 2.1 Platform-Specific Installers

| Platform | Format | Size | Installation Experience |
|----------|--------|------|------------------------|
| **Windows** | `.msi` (primary), `.exe` (portable) | ~15MB | Standard installer, UAC prompt |
| **macOS** | `.dmg` (primary), `.app` (direct) | ~15MB | Drag-to-Applications |
| **Linux** | `.AppImage` (universal), `.deb` (Ubuntu/Debian) | ~15MB | Package manager or direct download |

### 2.2 Package Managers (P1 - Month 2)

| Platform | Package Manager | Installation Command |
|----------|-----------------|---------------------|
| **macOS** | Homebrew | `brew install kalam-voice` |
| **Windows** | Chocolatey | `choco install kalam-voice` |
| **Windows** | winget | `winget install KalamVoice` |
| **Linux** | AUR (Arch) | `yay -S kalam-voice` |
| **Linux** | Flatpak | `flatpak install flathub com.kalam.voice` |

### 2.3 App Stores (P2-P3)

| Store | Priority | Timeline | Notes |
|-------|----------|----------|-------|
| **Microsoft Store** | P2 | Month 4 | Discovery for Windows users |
| **Flathub** | P2 | Month 3 | Primary Linux store |
| **Mac App Store** | P3 | Month 6 | If technically feasible with sandboxing |

### 2.4 Installation Flow

```
User discovers Kalam Voice
         ↓
Downloads from GitHub Releases or website
         ↓
Runs installer (platform-specific)
         ↓
Application launches to system tray
         ↓
First-run onboarding:
  - Microphone permission request
  - Hotkey configuration (Ctrl+Win default)
  - Cloud vs Local mode selection
  - API key setup (if BYOK selected)
         ↓
Ready to use - global hotkey active
```

### 2.5 First-Run Experience

**Initial Setup Wizard:**
1. **Welcome Screen** - Value proposition, privacy commitment
2. **Permission Setup** - Microphone access, accessibility (macOS)
3. **Mode Selection:**
   - Cloud (Groq API) - Requires API key
   - Local (SenseVoice) - Download 200MB model
   - Hybrid - Automatic switching
4. **Hotkey Configuration** - Visual key capture
5. **Quick Tutorial** - 30-second demo
6. **Optional Account** - GitHub Sponsors for hosted tier

---

## 3. Update Mechanism

### 3.1 Serverless OTA Updates

**Architecture:** Tauri Updater + GitHub Pages (zero server costs)

```
Application checks GitHub Pages for updates
         ↓
Compares version against embedded public key
         ↓
Download update in background
         ↓
Verify cryptographic signature (ECDSA)
         ↓
Notify user of ready update
         ↓
Install on user confirmation
         ↓
Automatic restart with saved session state
```

### 3.2 Update Flow Details

| Step | Implementation | User Experience |
|------|---------------|-----------------|
| **Check** | Daily background check to GitHub Pages | Invisible to user |
| **Download** | Silent download with progress in tray | Optional: progress notification |
| **Notify** | OS toast notification | "Update ready - Click to install" |
| **Defer** | Max 7 days for non-critical updates | "Remind me later" option |
| **Install** | Cryptographic verification + install | Automatic with restart |
| **Rollback** | On failure, revert to previous version | Seamless fallback |

### 3.3 Windows UAC Handling

- **Install Mode:** "passive" - Progress bar only, no interaction
- **Elevation:** Automatic UAC prompt
- **Before-Exit Hook:** Save user work before required restart

### 3.4 Critical vs Non-Critical Updates

| Type | Definition | User Prompt |
|------|-----------|-------------|
| **Critical** | Security fix, crash fix, API change | Immediate notification, max 24hr defer |
| **Feature** | New functionality | Standard notification, 7-day defer |
| **Patch** | Bug fix, performance | Weekly digest option |

---

## 4. Distribution Channels Timeline

### 4.1 Launch Phase (Month 1)

| Channel | Status | Assets |
|---------|--------|--------|
| GitHub Releases | ✅ Primary | All platforms |
| Official Website | ✅ Primary | Landing page, docs |
| Direct Download | ✅ Primary | .msi, .dmg, .AppImage, .deb |
| Product Hunt | 🎯 Target | Launch campaign |
| Hacker News | 🎯 Target | Show HN post |

### 4.2 Expansion Phase (Months 2-3)

| Channel | Status | Priority |
|---------|--------|----------|
| Homebrew (macOS) | 🚧 Planned | P1 |
| Chocolatey (Windows) | 🚧 Planned | P1 |
| AUR (Linux) | 🚧 Planned | P1 |
| Flathub (Linux) | 🚧 Planned | P2 |

### 4.3 Mature Phase (Months 4-6)

| Channel | Status | Priority |
|---------|--------|----------|
| Microsoft Store | 📋 Planned | P2 |
| Mac App Store | 📋 Evaluating | P3 |
| Enterprise Distribution | 📋 Planned | P2 |

---

## 5. Monetization & Hosted Tiers

### 5.1 Pricing Tiers

| Tier | Cost | Features | Target User |
|------|------|----------|-------------|
| **Free** | $0 | BYOK only, unlimited local usage | Individual users, developers |
| **Supporter** | $5/month | Kalam-hosted key (10 hrs cloud) | Casual users, convenience |
| **Pro** | $15/month | Kalam-hosted key (50 hrs cloud) | Power users, professionals |
| **Enterprise** | Custom | Self-hosted, unlimited, SLA | Organizations, compliance |

### 5.2 Payment Infrastructure

**Primary:** GitHub Sponsors
- Zero transaction fees
- Integrated with GitHub ecosystem
- Automatic tier management via API

**Alternative (if needed):** Stripe
- For enterprise invoicing
- Custom billing cycles

### 5.3 API Key Management

**BYOK (Default):**
- User provides own Groq API key
- Zero operational cost for Kalam
- Full control over usage/costs

**Hosted (Optional):**
- Kalam manages API keys
- Rate-limited by tier
- Auto-switch to local on limit

### 5.4 Self-Sustainability Target

**Annual Costs:**
- Apple Developer Account: $99
- Windows Code Signing: $0-200
- Domain/hosting: $0 (GitHub Pages)
- CI/CD: $0 (GitHub Actions)

**Total:** $99-299/year

**Break-even:** ~$5,000/month in GitHub Sponsors

---

## 6. Enterprise Distribution

### 6.1 Enterprise Features

| Feature | Implementation |
|---------|---------------|
| **Self-Hosted** | On-premise deployment option |
| **SSO Integration** | SAML, OIDC support |
| **Admin Dashboard** | Usage analytics, user management |
| **Policy Enforcement** | Force local mode, disable cloud |
| **Audit Logs** | Compliance reporting |
| **Custom Models** | Bring your own STT models |

### 6.2 Deployment Options

1. **SaaS (Hosted by Kalam)** - $15/user/month
2. **Self-Hosted (Enterprise License)** - Custom pricing
3. **Air-Gapped** - Fully offline deployment

### 6.3 Sales Process

- **Inbound:** Website contact form, GitHub Discussions
- **Outbound:** Target legal, healthcare, finance verticals
- **Trials:** 30-day enterprise trial with dedicated support

---

## 7. User Journey Maps

### 7.1 Casual User Journey

```
Discovers on Product Hunt
         ↓
Reads about 4x speed improvement
         ↓
Downloads from GitHub (Free)
         ↓
Installs, configures Groq API key (BYOK)
         ↓
Uses daily, happy with performance
         ↓
Decides to support project
         ↓
Becomes GitHub Sponsor ($5/month)
```

### 7.2 Developer Journey

```
Finds on Hacker News / r/rust
         ↓
Interested in tech stack (Tauri + Rust)
         ↓
Clones repository, builds from source
         ↓
Contributes PR or reports issue
         ↓
Uses extensively for coding
         ↓
Evangelizes to team/company
         ↓
Company adopts → Enterprise inquiry
```

### 7.3 Accessibility User Journey

```
Recommended by occupational therapist
         ↓
Downloads accessible version
         ↓
Relies on local mode for privacy
         ↓
Customizes voice commands extensively
         ↓
Shares success story
         ↓
Becomes community advocate
```

---

## 8. Metrics & KPIs

### 8.1 Distribution Metrics

| Metric | Target (6mo) | Target (12mo) |
|--------|-------------|---------------|
| Total Downloads | 50,000 | 200,000 |
| Active Users (monthly) | 15,000 | 60,000 |
| GitHub Stars | 5,000 | 15,000 |
| Package Manager Installs | 20% of total | 40% of total |
| Update Adoption Rate | 80% within 2 weeks | 85% within 1 week |

### 8.2 Business Metrics

| Metric | Target |
|--------|--------|
| GitHub Sponsors Revenue | $5,000/month (Year 1) |
| Enterprise Inquiries | 50+ (Year 1) |
| Enterprise Conversions | 10+ (Year 1) |
| Self-Sustaining | Month 12 |

### 8.3 Success Indicators

- **Organic Growth:** >50% of downloads from direct/organic
- **Retention:** >60% monthly active rate
- **Community:** 100+ contributors (Year 1)
- **Satisfaction:** NPS >50

---

## 9. CI/CD Pipeline for Distribution

### 9.1 GitHub Actions Workflow

```yaml
Trigger: Git tag push (v*.*.*)

Jobs:
1. Build Matrix:
   - Windows (x64, ARM64) → .msi, .exe
   - macOS (Intel, Apple Silicon) → .dmg, .app
   - Linux (x64, ARM64) → .AppImage, .deb

2. Code Signing:
   - Windows: Authenticode via Azure Key Vault
   - macOS: Apple Developer ID + Notarization

3. Artifact Generation:
   - Generate .sig files for updates
   - Create latest.json manifest
   - Upload to GitHub Release (draft)

4. Update Deployment:
   - Deploy latest.json to GitHub Pages
   - Publish GitHub Release
   - Post to changelog
```

### 9.2 Release Artifacts

| Platform | Artifacts | Signature |
|----------|-----------|-----------|
| Windows x64 | `Kalam-Voice-1.0.0-x64.msi` | `.sig` |
| Windows ARM64 | `Kalam-Voice-1.0.0-arm64.msi` | `.sig` |
| macOS Intel | `Kalam-Voice-1.0.0-x64.dmg` | `.sig` |
| macOS Apple Silicon | `Kalam-Voice-1.0.0-arm64.dmg` | `.sig` |
| Linux x64 | `Kalam-Voice-1.0.0-x64.AppImage` | `.sig` |
| Linux x64 | `kalam-voice_1.0.0_amd64.deb` | `.sig` |

### 9.3 Update Manifest (latest.json)

```json
{
  "version": "v1.2.0",
  "notes": "Release notes...",
  "pub_date": "2026-03-04T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "...",
      "url": "https://github.com/.../Kalam-Voice-1.2.0-x64.msi"
    },
    "darwin-aarch64": {
      "signature": "...",
      "url": "https://github.com/.../Kalam-Voice-1.2.0-arm64.dmg"
    },
    "linux-x86_64": {
      "signature": "...",
      "url": "https://github.com/.../Kalam-Voice-1.2.0-x64.AppImage"
    }
  }
}
```

---

## 10. Security & Trust

### 10.1 Code Signing

| Platform | Certificate | User Benefit |
|----------|-------------|--------------|
| **Windows** | Authenticode | No SmartScreen warnings |
| **macOS** | Apple Developer ID | No Gatekeeper warnings |
| **Linux** | GPG signing | Package manager verification |

### 10.2 Update Security

- **ECDSA Signatures:** All updates cryptographically signed
- **Public Key Embedded:** Compiled into binary
- **Private Key:** Stored in GitHub Secrets only
- **Man-in-the-Middle Protection:** Automatic verification

### 10.3 Trust Signals

- **Open Source:** Full source transparency
- **Privacy First:** No telemetry without opt-in
- **Security Audits:** Annual third-party review (Post-v1.0)
- **Bug Bounty:** HackerOne program (Post-v1.0)

---

## 11. Risks & Mitigations

### 11.1 Distribution Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Microsoft Store rejection | Medium | Low | Maintain direct download as primary |
| macOS notarization issues | Medium | Medium | Automated testing, early submission |
| Package manager delays | Medium | Low | Direct download always available |
| Low app store ranking | Low | Medium | Focus on organic/community growth |

### 11.2 Business Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Low GitHub Sponsors | Medium | High | Clear value proposition, supporter benefits |
| Enterprise slow adoption | Low | Medium | Self-sustaining on individual users |
| Competitor response | Medium | Low | Differentiation: open source, offline |

---

## 12. Documentation & Support

### 12.1 User Documentation

| Document | Location | Purpose |
|----------|----------|---------|
| **README** | GitHub | Quick start, features |
| **Installation Guide** | Website/DOCS.md | Platform-specific install |
| **User Manual** | Website/docs | Comprehensive usage |
| **API Documentation** | Website/docs | Integration guides |
| **FAQ** | Website | Common questions |

### 12.2 Support Channels

| Channel | Response Time | Purpose |
|---------|---------------|---------|
| **GitHub Issues** | 48 hours | Bug reports, features |
| **GitHub Discussions** | 72 hours | Q&A, community |
| **Discord** | Community | Real-time chat |
| **Email** | 24 hours | Enterprise, private |

### 12.3 Community Building

- **Contributing Guide:** Clear contribution process
- **Code of Conduct:** Inclusive community standards
- **Developer Docs:** Architecture, setup, APIs
- **Blog:** Updates, tutorials, case studies

---

## 13. Success Criteria

### 13.1 Launch Success (Month 1)

- [ ] 1,000+ GitHub stars
- [ ] 5,000+ downloads
- [ ] 100+ active users
- [ ] Featured on Product Hunt
- [ ] 10+ GitHub Sponsors

### 13.2 Growth Success (Month 6)

- [ ] 5,000+ GitHub stars
- [ ] 50,000+ downloads
- [ ] 15,000+ monthly active users
- [ ] 25+ contributors
- [ ] $2,500/month GitHub Sponsors
- [ ] Available on 3+ package managers

### 13.3 Sustainability Success (Month 12)

- [ ] 15,000+ GitHub stars
- [ ] 200,000+ downloads
- [ ] 60,000+ monthly active users
- [ ] 75+ contributors
- [ ] $5,000/month GitHub Sponsors
- [ ] 10+ enterprise customers
- [ ] Self-sustaining operations

---

## 14. Next Steps

### Pre-Launch (Before Release)

1. [ ] Create website landing page
2. [ ] Set up GitHub Sponsors tiers
3. [ ] Prepare Product Hunt campaign assets
4. [ ] Write launch blog post
5. [ ] Record demo video
6. [ ] Set up Discord community
7. [ ] Create social media accounts
8. [ ] Draft press kit

### Launch Day Checklist

1. [ ] Tag v1.0.0 release
2. [ ] Publish GitHub Release
3. [ ] Deploy latest.json to GitHub Pages
4. [ ] Post on Product Hunt
5. [ ] Submit to Hacker News
6. [ ] Announce on social media
7. [ ] Send to mailing list
8. [ ] Monitor for issues

---

## Appendix

### A. GitHub Repository Structure

```
kalam/
├── .github/
│   ├── workflows/
│   │   ├── build.yml          # CI/CD pipeline
│   │   └── release.yml        # Release automation
│   ├── FUNDING.yml            # GitHub Sponsors config
│   └── ISSUE_TEMPLATE/
├── .doc/
│   ├── research.md            # Technical research
│   ├── prd-proposal.md        # Product requirements
│   └── DISTRIBUTION.md        # This document
├── docs/
│   ├── INSTALL.md             # Installation guide
│   ├── USAGE.md               # User manual
│   └── API.md                 # API documentation
├── src/                       # Source code
├── website/                   # Landing page
└── README.md
```

### B. Key URLs

- **Repository:** https://github.com/[org]/kalam
- **Website:** https://kalamvoice.app (TBD)
- **Documentation:** https://docs.kalamvoice.app (TBD)
- **Releases:** https://github.com/[org]/kalam/releases
- **GitHub Pages:** https://[org].github.io/kalam/latest.json

### C. Contact Information

- **Maintainers:** [Team emails]
- **Security:** security@kalamvoice.app
- **Enterprise:** enterprise@kalamvoice.app
- **General:** hello@kalamvoice.app

---

**Document Version History**

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-03-04 | Distribution Team | Initial distribution strategy |

---

*This document is a living document and will be updated as the distribution strategy evolves.*
