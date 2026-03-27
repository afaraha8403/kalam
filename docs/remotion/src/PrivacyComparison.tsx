import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from 'remotion';

/**
 * "Smart Privacy" — Kalam Auto mode detects a sensitive app (bank) and switches to local STT.
 *
 * Scene 1: User dictates into a Notepad (normal app) → cloud STT, fast.
 * Scene 2: User switches to "Meridian Bank — Online Banking" → Kalam detects it,
 *          switches to local STT, audio never leaves the device.
 *
 * Polish: lighter window backgrounds, much larger windows filling most of the frame,
 * bigger text for readability, realistic bank UI with account balances and transfer form.
 */

const ACCENT = '#4fc1ff';
const GREEN = '#4ade80';
const AMBER = '#fbbf24';
const PILL_BG = '#0a0a0c';
const TASKBAR_BG = '#0c0c0e';

/** Recognizable app icons for the taskbar — SVG-based for crisp rendering */
const TaskbarIcon: React.FC<{ app: string }> = ({ app }) => {
  const size = 42;
  const r = 10;
  const icons: Record<string, React.ReactNode> = {
    chrome: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#1a1a2a" />
        <circle cx="21" cy="21" r="12" fill="none" stroke="#4285F4" strokeWidth="3" />
        <circle cx="21" cy="21" r="5" fill="#4285F4" />
        <path d="M21 9 L27 19" stroke="#EA4335" strokeWidth="3" strokeLinecap="round" />
        <path d="M14 28 L21 18" stroke="#34A853" strokeWidth="3" strokeLinecap="round" />
        <path d="M28 28 L18 22" stroke="#FBBC05" strokeWidth="3" strokeLinecap="round" />
      </svg>
    ),
    slack: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#4A154B" />
        <rect x="12" y="18" width="5" height="12" rx="2.5" fill="#E01E5A" />
        <rect x="18" y="12" width="12" height="5" rx="2.5" fill="#36C5F0" />
        <rect x="25" y="18" width="5" height="12" rx="2.5" fill="#2EB67D" />
        <rect x="12" y="25" width="12" height="5" rx="2.5" fill="#ECB22E" />
      </svg>
    ),
    vscode: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#1e1e2e" />
        <path d="M28 10 L28 32 L14 26 L28 10Z" fill="#007ACC" opacity="0.8" />
        <path d="M14 16 L22 21 L14 26 L10 23 L14 16Z" fill="#007ACC" />
      </svg>
    ),
    word: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#185ABD" />
        <text x="21" y="27" textAnchor="middle" fill="#fff" fontSize="18" fontWeight="800" fontFamily="Segoe UI, sans-serif">W</text>
      </svg>
    ),
    spotify: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#191414" />
        <circle cx="21" cy="21" r="12" fill="#1DB954" />
        <path d="M15 18 Q21 16 27 19" stroke="#191414" strokeWidth="2.5" fill="none" strokeLinecap="round" />
        <path d="M16 22 Q21 20 26 23" stroke="#191414" strokeWidth="2" fill="none" strokeLinecap="round" />
        <path d="M17 26 Q21 24.5 25 26.5" stroke="#191414" strokeWidth="1.5" fill="none" strokeLinecap="round" />
      </svg>
    ),
    files: (
      <svg width={size} height={size} viewBox="0 0 42 42">
        <rect width={size} height={size} rx={r} fill="#1a1a2a" />
        <rect x="12" y="14" width="18" height="16" rx="2" fill="#F59E0B" opacity="0.85" />
        <rect x="12" y="12" width="10" height="4" rx="1" fill="#F59E0B" opacity="0.6" />
      </svg>
    ),
  };
  return <div style={{ width: size, height: size, borderRadius: r, overflow: 'hidden', flexShrink: 0 }}>{icons[app]}</div>;
};

const WaveformBars: React.FC<{ frame: number; color: string; active: boolean }> = ({ frame, color, active }) => {
  if (!active) return null;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 3 }}>
      {Array.from({ length: 9 }, (_, i) => {
        const h = 4 + Math.abs(Math.sin(frame * 0.22 + i * 1.1)) * 18;
        return (
          <div key={i} style={{ width: 3.5, height: h, borderRadius: 2, backgroundColor: color }} />
        );
      })}
    </div>
  );
};

export const PrivacyComparison: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  // --- Phases ---
  const isNormalPhase = frame >= 40 && frame < 100;
  const isTransitioning = frame >= 100 && frame < 130;
  const isDetecting = frame >= 130 && frame < 160;
  const isSensitivePhase = frame >= 160 && frame < 235;
  const isOutro = frame >= 235;

  const entrance = spring({ frame, fps, config: { damping: 200 } });

  // --- Text content ---
  const notepadText = "Remind me to pick up groceries after work and call the dentist to reschedule Friday's appointment.";
  const bankText = "Transfer five hundred dollars to savings";

  const normalTextProgress = interpolate(frame, [48, 95], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const normalChars = Math.floor(normalTextProgress * notepadText.length);

  const bankTextProgress = interpolate(frame, [168, 225], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const bankChars = Math.floor(bankTextProgress * bankText.length);

  // --- App switch ---
  const switchProgress = spring({ frame: frame - 100, fps, config: { damping: 15, stiffness: 120 } });
  const normalAppX = interpolate(switchProgress, [0, 1], [0, -1920]);
  const sensitiveAppX = interpolate(switchProgress, [0, 1], [1920, 0]);

  // --- Shield ---
  const shieldPulse = spring({ frame: frame - 132, fps, config: { damping: 8, stiffness: 200 } });
  const shieldGlow = interpolate(frame, [132, 145, 155, 160], [0, 1, 1, 0.6], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });

  // --- Badges ---
  const cloudBadgeOpacity = interpolate(frame, [40, 48, 95, 103], [0, 1, 1, 0], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const localBadgeOpacity = interpolate(frame, [135, 145], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });

  // --- Pill ---
  const pillExpand = spring({ frame: frame - 35, fps, config: { damping: 15, stiffness: 120 } });
  const pillWidth = interpolate(pillExpand, [0, 1], [52, 220]);
  const pillHeight = interpolate(pillExpand, [0, 1], [12, 44]);

  // --- Detection banner ---
  const detectionBannerY = spring({ frame: frame - 130, fps, config: { damping: 12, stiffness: 150 } });
  const detectionBannerOpacity = interpolate(frame, [130, 138, 155, 162], [0, 1, 1, 0], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });

  // --- Outro ---
  const outroOpacity = interpolate(frame, [240, 255], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const outroScale = spring({ frame: frame - 240, fps, config: { damping: 200 } });

  /** Window dimensions — large, filling most of the 1920x1080 frame */
  const WIN_W = 1500;
  const WIN_H = 780;

  return (
    <AbsoluteFill style={{
      backgroundColor: '#111318',
      fontFamily: "'Segoe UI', 'SF Pro Display', system-ui, sans-serif",
    }}>
      <div style={{
        opacity: entrance,
        transform: `scale(${interpolate(entrance, [0, 1], [0.96, 1])})`,
        width: '100%', height: '100%', position: 'relative', overflow: 'hidden',
      }}>

        {/* Desktop wallpaper — lighter so windows are visible */}
        <div style={{
          position: 'absolute', inset: 0,
          background: 'linear-gradient(135deg, #1a2540 0%, #1e2d4a 30%, #162238 70%, #1a2744 100%)',
        }} />
        <div style={{
          position: 'absolute', inset: 0, opacity: 0.04,
          backgroundImage: 'linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)',
          backgroundSize: '60px 60px',
        }} />

        {/* ===== TOP: Mode indicator ===== */}
        <div style={{
          position: 'absolute', top: 18, width: '100%',
          display: 'flex', justifyContent: 'center', zIndex: 10,
        }}>
          {/* Cloud badge */}
          <div style={{
            opacity: cloudBadgeOpacity,
            position: 'absolute',
            display: 'flex', alignItems: 'center', gap: 8,
            padding: '8px 20px', borderRadius: 100,
            backgroundColor: 'rgba(79, 193, 255, 0.12)',
            border: `1px solid ${ACCENT}40`,
          }}>
            <span style={{ fontSize: 16 }}>☁️</span>
            <span style={{ color: ACCENT, fontSize: 15, fontWeight: 600 }}>Cloud STT — Fast</span>
          </div>
          {/* Local badge */}
          <div style={{
            opacity: localBadgeOpacity,
            position: 'absolute',
            display: 'flex', alignItems: 'center', gap: 8,
            padding: '8px 20px', borderRadius: 100,
            backgroundColor: 'rgba(74, 222, 128, 0.12)',
            border: `1px solid ${GREEN}40`,
          }}>
            <span style={{ fontSize: 16 }}>🔒</span>
            <span style={{ color: GREEN, fontSize: 15, fontWeight: 600 }}>Local STT — Private</span>
          </div>
        </div>

        {/* ===== APP WINDOW AREA ===== */}
        <div style={{
          position: 'absolute', top: 70, left: 0, width: '100%', height: WIN_H + 40,
          overflow: 'hidden',
        }}>

          {/* --- NOTEPAD WINDOW --- */}
          <div style={{
            position: 'absolute', top: 0, left: '50%',
            transform: `translateX(calc(-50% + ${normalAppX}px))`,
            width: WIN_W, height: WIN_H,
            borderRadius: 16, overflow: 'hidden',
            border: '1px solid rgba(255,255,255,0.12)',
            boxShadow: '0 30px 80px rgba(0,0,0,0.6)',
            display: 'flex', flexDirection: 'column',
          }}>
            {/* Title bar */}
            <div style={{
              height: 52, backgroundColor: '#2c2c3a',
              display: 'flex', alignItems: 'center', padding: '0 20px', gap: 10,
              borderBottom: '1px solid rgba(255,255,255,0.06)',
              flexShrink: 0,
            }}>
              <div style={{ display: 'flex', gap: 8 }}>
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#ff5f57' }} />
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#febc2e' }} />
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#28c840' }} />
              </div>
              <div style={{ flex: 1, textAlign: 'center', color: 'rgba(255,255,255,0.6)', fontSize: 15, fontWeight: 500 }}>
                📝 Notepad — reminders.txt
              </div>
            </div>

            {/* Notepad body — lighter background */}
            <div style={{
              flex: 1, backgroundColor: '#1e1e2e', padding: '40px 48px',
            }}>
              {/* Line numbers gutter */}
              <div style={{ display: 'flex', gap: 24 }}>
                <div style={{ color: 'rgba(255,255,255,0.15)', fontSize: 18, lineHeight: 1.8, fontFamily: "'Consolas', 'Courier New', monospace", userSelect: 'none', textAlign: 'right', width: 28 }}>
                  1{'\n'}2{'\n'}3
                </div>
                <div style={{
                  color: '#e2e8f0', fontSize: 22, lineHeight: 1.8,
                  fontFamily: "'Consolas', 'Courier New', monospace",
                  flex: 1,
                }}>
                  {notepadText.slice(0, normalChars)}
                  {isNormalPhase && (
                    <span style={{
                      display: 'inline-block', width: 2.5, height: 24,
                      backgroundColor: ACCENT, marginLeft: 2,
                      opacity: frame % 30 < 15 ? 1 : 0.2,
                      verticalAlign: 'text-bottom',
                    }} />
                  )}
                  {frame < 45 && (
                    <span style={{ color: 'rgba(255,255,255,0.2)' }}>Start typing or dictate with Kalam...</span>
                  )}
                </div>
              </div>
            </div>

            {/* Status bar */}
            <div style={{
              height: 32, backgroundColor: '#252535',
              borderTop: '1px solid rgba(255,255,255,0.06)',
              display: 'flex', alignItems: 'center', padding: '0 20px',
              gap: 20,
            }}>
              <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 12 }}>UTF-8</span>
              <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 12 }}>Ln 1, Col {normalChars + 1}</span>
              {isNormalPhase && (
                <span style={{ color: ACCENT, fontSize: 12, fontWeight: 600, marginLeft: 'auto' }}>🎙 Kalam dictating...</span>
              )}
            </div>
          </div>

          {/* --- BANK WINDOW --- */}
          <div style={{
            position: 'absolute', top: 0, left: '50%',
            transform: `translateX(calc(-50% + ${sensitiveAppX}px))`,
            width: WIN_W, height: WIN_H,
            borderRadius: 16, overflow: 'hidden',
            border: `1px solid ${(isSensitivePhase || isDetecting) ? `${GREEN}25` : 'rgba(255,255,255,0.12)'}`,
            boxShadow: (isSensitivePhase || isDetecting)
              ? `0 30px 80px rgba(0,0,0,0.6), 0 0 50px ${GREEN}08`
              : '0 30px 80px rgba(0,0,0,0.6)',
            display: 'flex', flexDirection: 'column',
          }}>
            {/* Browser chrome */}
            <div style={{
              height: 52, backgroundColor: '#2a2a3a',
              display: 'flex', alignItems: 'center', padding: '0 20px', gap: 10,
              borderBottom: '1px solid rgba(255,255,255,0.06)',
              flexShrink: 0,
            }}>
              <div style={{ display: 'flex', gap: 8 }}>
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#ff5f57' }} />
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#febc2e' }} />
                <div style={{ width: 14, height: 14, borderRadius: '50%', backgroundColor: '#28c840' }} />
              </div>
              {/* URL bar */}
              <div style={{
                flex: 1, height: 32, borderRadius: 8,
                backgroundColor: '#1a1a28',
                display: 'flex', alignItems: 'center', padding: '0 14px', gap: 8,
              }}>
                <span style={{ color: GREEN, fontSize: 13 }}>🔒</span>
                <span style={{ color: 'rgba(255,255,255,0.5)', fontSize: 14 }}>
                  https://<span style={{ color: 'rgba(255,255,255,0.7)' }}>meridianbank.com</span>/dashboard
                </span>
              </div>
            </div>

            {/* Bank app body */}
            <div style={{
              flex: 1, backgroundColor: '#161625', display: 'flex',
            }}>
              {/* Bank sidebar nav */}
              <div style={{
                width: 240, backgroundColor: '#12121f',
                borderRight: '1px solid rgba(255,255,255,0.05)',
                padding: '28px 0',
              }}>
                <div style={{ padding: '0 20px 20px', borderBottom: '1px solid rgba(255,255,255,0.05)', marginBottom: 16 }}>
                  <div style={{ color: '#fff', fontSize: 22, fontWeight: 800, letterSpacing: '-0.02em' }}>Meridian</div>
                  <div style={{ color: 'rgba(255,255,255,0.35)', fontSize: 13, marginTop: 2 }}>Personal Banking</div>
                </div>
                {['Dashboard', 'Accounts', 'Transfers', 'Payments', 'Settings'].map((item, i) => (
                  <div key={i} style={{
                    padding: '10px 20px',
                    color: i === 2 ? '#fff' : 'rgba(255,255,255,0.45)',
                    fontSize: 15, fontWeight: i === 2 ? 600 : 400,
                    backgroundColor: i === 2 ? 'rgba(74, 222, 128, 0.1)' : 'transparent',
                    borderLeft: i === 2 ? `3px solid ${GREEN}` : '3px solid transparent',
                  }}>
                    {item}
                  </div>
                ))}
              </div>

              {/* Bank main content */}
              <div style={{ flex: 1, padding: '32px 40px' }}>
                <div style={{ color: '#fff', fontSize: 26, fontWeight: 700, marginBottom: 8 }}>Transfer Funds</div>
                <div style={{ color: 'rgba(255,255,255,0.4)', fontSize: 15, marginBottom: 32 }}>Move money between your accounts</div>

                {/* Account cards */}
                <div style={{ display: 'flex', gap: 20, marginBottom: 36 }}>
                  <div style={{
                    flex: 1, backgroundColor: '#1e1e30', borderRadius: 12,
                    padding: '22px 24px', border: '1px solid rgba(255,255,255,0.06)',
                  }}>
                    <div style={{ color: 'rgba(255,255,255,0.4)', fontSize: 13, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>Checking</div>
                    <div style={{ color: '#fff', fontSize: 32, fontWeight: 700, marginTop: 8 }}>$4,231.50</div>
                    <div style={{ color: 'rgba(255,255,255,0.3)', fontSize: 13, marginTop: 4 }}>••••  7842</div>
                  </div>
                  <div style={{
                    flex: 1, backgroundColor: '#1e1e30', borderRadius: 12,
                    padding: '22px 24px', border: '1px solid rgba(255,255,255,0.06)',
                  }}>
                    <div style={{ color: 'rgba(255,255,255,0.4)', fontSize: 13, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>Savings</div>
                    <div style={{ color: '#fff', fontSize: 32, fontWeight: 700, marginTop: 8 }}>$12,847.00</div>
                    <div style={{ color: 'rgba(255,255,255,0.3)', fontSize: 13, marginTop: 4 }}>••••  3156</div>
                  </div>
                </div>

                {/* Voice command input */}
                <div style={{
                  backgroundColor: '#1e1e30', borderRadius: 12,
                  padding: '24px 28px',
                  border: `1px solid ${isSensitivePhase ? `${GREEN}30` : 'rgba(255,255,255,0.06)'}`,
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 14 }}>
                    <span style={{ fontSize: 18 }}>🎙</span>
                    <span style={{ color: 'rgba(255,255,255,0.5)', fontSize: 14, fontWeight: 600 }}>Voice Command</span>
                    {isSensitivePhase && (
                      <span style={{
                        fontSize: 12, fontWeight: 600, color: GREEN,
                        backgroundColor: `${GREEN}15`, padding: '3px 10px', borderRadius: 4,
                        marginLeft: 'auto',
                      }}>
                        🔒 Local STT
                      </span>
                    )}
                  </div>
                  <div style={{
                    color: '#e2e8f0', fontSize: 22, lineHeight: 1.6,
                    minHeight: 36,
                  }}>
                    {bankText.slice(0, bankChars)}
                    {isSensitivePhase && (
                      <span style={{
                        display: 'inline-block', width: 2.5, height: 24,
                        backgroundColor: GREEN, marginLeft: 2,
                        opacity: frame % 30 < 15 ? 1 : 0.2,
                        verticalAlign: 'text-bottom',
                      }} />
                    )}
                    {!isSensitivePhase && bankChars === 0 && (
                      <span style={{ color: 'rgba(255,255,255,0.2)' }}>Speak to enter a command...</span>
                    )}
                  </div>
                </div>

                {/* Privacy indicator */}
                {(isDetecting || isSensitivePhase) && (
                  <div style={{
                    display: 'flex', alignItems: 'center', gap: 10, marginTop: 16,
                    opacity: shieldPulse,
                  }}>
                    <div style={{
                      width: 28, height: 28, borderRadius: '50%',
                      backgroundColor: `${GREEN}20`,
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 14, boxShadow: `0 0 ${shieldGlow * 16}px ${GREEN}40`,
                    }}>🛡️</div>
                    <span style={{ color: GREEN, fontSize: 14, fontWeight: 600 }}>Audio processed on-device — never sent to any server</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* ===== DETECTION BANNER ===== */}
        {frame >= 130 && frame < 165 && (
          <div style={{
            position: 'absolute', top: '50%', left: '50%',
            transform: `translate(-50%, calc(-50% + ${interpolate(detectionBannerY, [0, 1], [30, 0])}px))`,
            opacity: detectionBannerOpacity, zIndex: 20,
          }}>
            <div style={{
              padding: '20px 44px', borderRadius: 16,
              backgroundColor: 'rgba(10, 14, 23, 0.96)',
              border: `2px solid ${AMBER}60`,
              boxShadow: `0 0 50px ${AMBER}20, 0 20px 60px rgba(0,0,0,0.5)`,
              display: 'flex', alignItems: 'center', gap: 16,
            }}>
              <div style={{ fontSize: 32, transform: `scale(${shieldPulse})` }}>⚡</div>
              <div>
                <div style={{ color: AMBER, fontSize: 22, fontWeight: 700 }}>Sensitive app detected</div>
                <div style={{ color: 'rgba(255,255,255,0.5)', fontSize: 15, marginTop: 3 }}>
                  Banking site recognized — switching to local STT
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ===== OS TASKBAR ===== */}
        <div style={{
          position: 'absolute', bottom: 0, left: 0, right: 0,
          height: 64, backgroundColor: TASKBAR_BG,
          borderTop: '1px solid rgba(255,255,255,0.08)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
        }}>
          {['chrome', 'slack', 'vscode', 'word', 'spotify', 'files'].map((app) => (
            <TaskbarIcon key={app} app={app} />
          ))}
        </div>

        {/* ===== KALAM PILL ===== */}
        <div style={{
          position: 'absolute', bottom: 84, left: '50%',
          transform: 'translateX(-50%)', zIndex: 15,
        }}>
          <div style={{
            width: pillWidth, height: pillHeight, borderRadius: 100,
            backgroundColor: PILL_BG,
            border: `1px solid ${(isSensitivePhase || isDetecting) ? `${GREEN}30` : 'rgba(255,255,255,0.1)'}`,
            display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8,
            boxShadow: isSensitivePhase ? `0 4px 20px ${GREEN}20` : '0 4px 20px rgba(0,0,0,0.4)',
          }}>
            {frame >= 35 && frame < 45 && (
              <div style={{
                width: 8, height: 8, borderRadius: '50%',
                backgroundColor: ACCENT, opacity: 0.7 + Math.sin(frame * 0.4) * 0.3,
              }} />
            )}
            <WaveformBars frame={frame} color={ACCENT} active={isNormalPhase} />
            <WaveformBars frame={frame} color={GREEN} active={isSensitivePhase} />
            {isNormalPhase && pillWidth > 120 && (
              <span style={{ color: ACCENT, fontSize: 14, fontWeight: 600, whiteSpace: 'nowrap' }}>Cloud</span>
            )}
            {isSensitivePhase && pillWidth > 120 && (
              <span style={{ color: GREEN, fontSize: 14, fontWeight: 600, whiteSpace: 'nowrap', display: 'flex', alignItems: 'center', gap: 4 }}>
                🔒 Local
              </span>
            )}
            {isTransitioning && (
              <div style={{
                width: 8, height: 8, borderRadius: '50%',
                backgroundColor: AMBER, opacity: 0.7 + Math.sin(frame * 0.5) * 0.3,
              }} />
            )}
            {isDetecting && (
              <span style={{ color: AMBER, fontSize: 13, fontWeight: 600, whiteSpace: 'nowrap' }}>Switching...</span>
            )}
          </div>
        </div>

        {/* ===== OUTRO ===== */}
        {isOutro && (
          <div style={{
            position: 'absolute', inset: 0,
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
            opacity: outroOpacity, zIndex: 30,
            backgroundColor: `rgba(10, 14, 23, ${interpolate(frame, [235, 255], [0, 0.94], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' })})`,
          }}>
            <div style={{
              transform: `scale(${interpolate(outroScale, [0, 1], [0.9, 1])})`,
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18,
            }}>
              <div style={{ fontSize: 60 }}>🛡️</div>
              <div style={{ color: '#fff', fontSize: 56, fontWeight: 800, letterSpacing: '-0.03em' }}>
                Your voice. Your rules.
              </div>
              <div style={{
                color: 'rgba(255,255,255,0.45)', fontSize: 22, fontWeight: 500,
                maxWidth: 650, textAlign: 'center', lineHeight: 1.5,
              }}>
                Kalam's Auto mode detects sensitive apps and keeps your audio on-device — automatically.
              </div>
            </div>
          </div>
        )}

      </div>
    </AbsoluteFill>
  );
};
