# To Do

## Done
- [X] When the toggle is held down or is in press down and no speech or text is being picked up, Grok and possibly local models and possibly OpenAI Whisper return the words in the dictionary.
- [X] on Mac the transparent wrapper for the overlay or blob or pill is white and not transparent.
- [X] When you attempt to type in the email in the onboarding page it crashes the app.
- [X] The website does not properly identify if the macOS is intel or arm.
- [X] Show the model and cloud/local mode in the bottom status bar.
- [X] Language selector: onboarding + settings, default English, multi-language, keybinding to toggle; show notification (blip) on toggle.
- [X] Pill expand direction, offset and initial starting position.
- [X] Version number in settings. (Verify it displays with beta tag, e.g. beta 1, beta 2, beta 3.)

---

## Now (immediate)
- [X] Embed a document DB for syncing later.
- [ ] Local speech-to-text: app must download and run the models (not only download).
- [X] Transcription history.
- [X] Dictionary: add dictionary table; research passing dictionary to Groq/OpenAI Whisper for accuracy; research support for local models.
- [ ] Help menu option: link to docs top page.
- On the blip/overlay, have it display the language that we are currently dictating in. This should be a toggleable setting and only available when more than one language is enabled.
- Overlay (inactive mode): keep blip visible at all times as a reminder to dictate, but optimize so it does not cover OS icons or block interaction with other applications (e.g. popover/tooltip behavior, or draggable repositioning).

---

## Future

### General (non-premium)
- [ ] Code signing: Microsoft trusted publisher (or equivalent) for installer to reduce SmartScreen friction and improve adoption.
- [ ] Syncing service (backend): collect emails and other statistics to provide a better service.
- [ ] Memory usage: investigate and optimize if needed.

### Premium (post-MVP, paid features)
*Notes, Tasks, Reminders: all behind paywall (transcribed or manual). User can see the feature and add 1–2 items to try; rest paywalled. Show "Get your license from [so-and-so]."*
- Syncing: Notes, Tasks, Reminder, Transcription history; sync with Google and Outlook.
- Balance / low-balance warning / reload UX (subscription only; not for bring-your-own-key).
- Auto dictionary learning.
- **Context-aware behavior**: Style (learn user's writing style; guided profiles — e.g. personal, work, email); App-aware dictation (detect target app and apply per-app settings, cleanup, formatting; profiles editable; research how to detect target app).
