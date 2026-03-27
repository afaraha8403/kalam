import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from 'remotion';

/**
 * "Dictation in Action" — Slack-style chat UI on a desktop.
 * The Kalam pill floats above the taskbar; dictated text appears as a chat message
 * in a Slack-like messaging app with realistic channel sidebar and message bubbles.
 *
 * Key polish: lighter window backgrounds for readability, much larger window,
 * bigger text, visible even in light-mode website embeds.
 */

const ACCENT = '#4fc1ff';
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

const WaveformBars: React.FC<{ frame: number; barCount: number; color: string }> = ({ frame, barCount, color }) => {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 3, height: 32 }}>
      {Array.from({ length: barCount }, (_, i) => {
        const phase = (frame * 0.15) + (i * 1.2);
        const height = 4 + Math.abs(Math.sin(phase)) * 20 + Math.abs(Math.cos(phase * 0.7)) * 8;
        return (
          <div key={i} style={{
            width: 4, height, borderRadius: 2,
            backgroundColor: color, opacity: 0.9,
          }} />
        );
      })}
    </div>
  );
};

const ListenDot: React.FC<{ frame: number }> = ({ frame }) => {
  const opacity = 0.4 + Math.sin(frame * 0.15) * 0.6;
  const scale = 0.9 + Math.sin(frame * 0.15) * 0.15;
  return (
    <div style={{
      width: 10, height: 10, borderRadius: '50%',
      backgroundColor: ACCENT, opacity,
      transform: `scale(${scale})`,
      boxShadow: `0 0 8px ${ACCENT}`,
    }} />
  );
};

export const DictationDemo: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const dictatedText = "Hey team, the new release is looking great. Let's ship it tomorrow morning and announce on Twitter.";

  const desktopOpacity = interpolate(frame, [0, 25], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });

  const pillExpand = spring({ frame: frame - 30, fps, config: { damping: 15, stiffness: 120 } });
  const pillCollapse = spring({ frame: frame - 200, fps, config: { damping: 20, stiffness: 80 } });
  const pillWidth = interpolate(frame < 200 ? pillExpand : 1 - pillCollapse, [0, 1], [52, 260], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const pillHeight = interpolate(frame < 200 ? pillExpand : 1 - pillCollapse, [0, 1], [12, 48], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const pillOpacityVal = frame < 200 ? interpolate(pillExpand, [0, 1], [0.7, 1]) : interpolate(pillCollapse, [0, 1], [1, 0.7]);

  const isListening = frame >= 35 && frame < 60;
  const isRecording = frame >= 60 && frame < 180;
  const isSuccess = frame >= 180 && frame < 220;

  const textProgress = interpolate(frame, [65, 175], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const charsToShow = Math.floor(textProgress * dictatedText.length);
  const visibleText = dictatedText.slice(0, charsToShow);

  const windowSlide = spring({ frame: frame - 10, fps, config: { damping: 18, stiffness: 100 } });
  const windowY = interpolate(windowSlide, [0, 1], [40, 0]);
  const windowOpacity = interpolate(windowSlide, [0, 1], [0, 1]);

  const checkScale = spring({ frame: frame - 182, fps, config: { damping: 10, stiffness: 200 } });

  /** Slack-style channel list items */
  const channels = ['# general', '# product', '# design', '# engineering', '# random'];
  const dms = ['Sarah Chen', 'Mike R.', 'Alex Kim'];

  return (
    <AbsoluteFill style={{ backgroundColor: '#111318', fontFamily: "'Segoe UI', 'SF Pro Display', system-ui, sans-serif" }}>
      <div style={{ opacity: desktopOpacity, width: '100%', height: '100%', position: 'relative' }}>

        {/* Desktop wallpaper — lighter gradient so the window pops */}
        <div style={{
          position: 'absolute', inset: 0,
          background: 'linear-gradient(135deg, #1a2540 0%, #1e2d4a 30%, #162238 70%, #1a2744 100%)',
        }} />

        {/* Subtle grid */}
        <div style={{
          position: 'absolute', inset: 0, opacity: 0.04,
          backgroundImage: 'linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)',
          backgroundSize: '60px 60px',
        }} />

        {/* ===== SLACK WINDOW — takes up most of the screen ===== */}
        <div style={{
          position: 'absolute',
          top: 50,
          left: '50%',
          transform: `translateX(-50%) translateY(${windowY}px)`,
          opacity: windowOpacity,
          width: 1500,
          height: 820,
          borderRadius: 16,
          overflow: 'hidden',
          border: '1px solid rgba(255,255,255,0.12)',
          boxShadow: '0 30px 80px rgba(0,0,0,0.6)',
          display: 'flex',
        }}>

          {/* --- Sidebar --- */}
          <div style={{
            width: 280,
            backgroundColor: '#1a1030',
            borderRight: '1px solid rgba(255,255,255,0.06)',
            padding: '20px 0',
            flexShrink: 0,
          }}>
            {/* Workspace header */}
            <div style={{
              padding: '0 20px 18px',
              borderBottom: '1px solid rgba(255,255,255,0.06)',
              marginBottom: 16,
            }}>
              <div style={{ color: '#fff', fontSize: 20, fontWeight: 700 }}>Kalam HQ</div>
              <div style={{ color: 'rgba(255,255,255,0.4)', fontSize: 13, marginTop: 2 }}>42 members</div>
            </div>

            {/* Channels */}
            <div style={{ padding: '0 12px' }}>
              <div style={{ color: 'rgba(255,255,255,0.35)', fontSize: 12, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', padding: '0 8px', marginBottom: 6 }}>
                Channels
              </div>
              {channels.map((ch, i) => (
                <div key={i} style={{
                  padding: '7px 12px',
                  borderRadius: 6,
                  color: i === 1 ? '#fff' : 'rgba(255,255,255,0.55)',
                  fontSize: 15,
                  fontWeight: i === 1 ? 600 : 400,
                  backgroundColor: i === 1 ? 'rgba(79, 193, 255, 0.15)' : 'transparent',
                }}>
                  {ch}
                </div>
              ))}

              {/* DMs */}
              <div style={{ color: 'rgba(255,255,255,0.35)', fontSize: 12, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', padding: '0 8px', marginTop: 20, marginBottom: 6 }}>
                Direct Messages
              </div>
              {dms.map((dm, i) => (
                <div key={i} style={{
                  padding: '7px 12px',
                  borderRadius: 6,
                  color: 'rgba(255,255,255,0.55)',
                  fontSize: 15,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                }}>
                  <div style={{
                    width: 10, height: 10, borderRadius: '50%',
                    backgroundColor: i === 0 ? '#4ade80' : 'rgba(255,255,255,0.2)',
                  }} />
                  {dm}
                </div>
              ))}
            </div>
          </div>

          {/* --- Main chat area --- */}
          <div style={{
            flex: 1,
            backgroundColor: '#1e1e2e',
            display: 'flex',
            flexDirection: 'column',
          }}>
            {/* Channel header */}
            <div style={{
              height: 60,
              borderBottom: '1px solid rgba(255,255,255,0.06)',
              display: 'flex',
              alignItems: 'center',
              padding: '0 28px',
              gap: 12,
            }}>
              <div style={{ color: '#fff', fontSize: 20, fontWeight: 700 }}># product</div>
              <div style={{ color: 'rgba(255,255,255,0.3)', fontSize: 14 }}>Release planning and updates</div>
            </div>

            {/* Messages area */}
            <div style={{
              flex: 1,
              padding: '28px 28px 20px',
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'flex-end',
              gap: 24,
            }}>
              {/* Existing message 1 */}
              <div style={{ display: 'flex', gap: 14 }}>
                <div style={{
                  width: 44, height: 44, borderRadius: 8,
                  backgroundColor: '#7c3aed', flexShrink: 0,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  color: '#fff', fontSize: 18, fontWeight: 700,
                }}>S</div>
                <div>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 4 }}>
                    <span style={{ color: '#fff', fontSize: 16, fontWeight: 700 }}>Sarah Chen</span>
                    <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 13 }}>10:42 AM</span>
                  </div>
                  <div style={{ color: 'rgba(255,255,255,0.75)', fontSize: 17, lineHeight: 1.55 }}>
                    QA signed off on v2.4 — all critical bugs resolved. We're green across the board.
                  </div>
                </div>
              </div>

              {/* Existing message 2 */}
              <div style={{ display: 'flex', gap: 14 }}>
                <div style={{
                  width: 44, height: 44, borderRadius: 8,
                  backgroundColor: '#0891b2', flexShrink: 0,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  color: '#fff', fontSize: 18, fontWeight: 700,
                }}>M</div>
                <div>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 4 }}>
                    <span style={{ color: '#fff', fontSize: 16, fontWeight: 700 }}>Mike R.</span>
                    <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 13 }}>10:45 AM</span>
                  </div>
                  <div style={{ color: 'rgba(255,255,255,0.75)', fontSize: 17, lineHeight: 1.55 }}>
                    Nice! CI pipeline is stable too. Ready when you are.
                  </div>
                </div>
              </div>

              {/* User's dictated message */}
              {frame >= 60 && (
                <div style={{
                  display: 'flex', gap: 14,
                  opacity: interpolate(frame, [60, 68], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' }),
                }}>
                  <div style={{
                    width: 44, height: 44, borderRadius: 8,
                    backgroundColor: '#4fc1ff', flexShrink: 0,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    color: '#000', fontSize: 18, fontWeight: 700,
                  }}>Y</div>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 4 }}>
                      <span style={{ color: '#fff', fontSize: 16, fontWeight: 700 }}>You</span>
                      <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 13 }}>10:47 AM</span>
                      {isRecording && (
                        <span style={{
                          fontSize: 12, fontWeight: 600, color: ACCENT,
                          backgroundColor: 'rgba(79, 193, 255, 0.12)',
                          padding: '2px 8px', borderRadius: 4,
                        }}>
                          🎙 dictating...
                        </span>
                      )}
                    </div>
                    <div style={{ color: 'rgba(255,255,255,0.85)', fontSize: 17, lineHeight: 1.55, maxWidth: 800 }}>
                      {visibleText}
                      {isRecording && (
                        <span style={{
                          display: 'inline-block', width: 2, height: 20,
                          backgroundColor: ACCENT, marginLeft: 2,
                          opacity: frame % 30 < 15 ? 1 : 0.2,
                          verticalAlign: 'text-bottom',
                        }} />
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Message input bar */}
            <div style={{
              padding: '16px 28px',
              borderTop: '1px solid rgba(255,255,255,0.06)',
            }}>
              <div style={{
                backgroundColor: '#2a2a3e',
                borderRadius: 10,
                padding: '14px 20px',
                color: 'rgba(255,255,255,0.3)',
                fontSize: 16,
                border: '1px solid rgba(255,255,255,0.06)',
              }}>
                Message #product
              </div>
            </div>
          </div>
        </div>

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
          position: 'absolute', bottom: 90, left: '50%',
          transform: 'translateX(-50%)',
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
        }}>
          {(isListening || isRecording) && (
            <div style={{
              color: 'rgba(255,255,255,0.5)', fontSize: 14, fontWeight: 500, letterSpacing: '0.05em',
              opacity: interpolate(frame, [35, 42], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' }),
            }}>
              {isListening ? 'LISTENING' : 'RECORDING'}
            </div>
          )}
          <div style={{
            width: pillWidth, height: pillHeight, borderRadius: 100,
            backgroundColor: PILL_BG,
            border: `1px solid ${frame < 30 ? 'rgba(255,255,255,0.6)' : 'rgba(255,255,255,0.1)'}`,
            opacity: pillOpacityVal,
            display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10,
            overflow: 'hidden',
            boxShadow: isRecording
              ? `0 0 20px rgba(79, 193, 255, 0.15), 0 4px 12px rgba(0,0,0,0.4)`
              : '0 2px 8px rgba(0,0,0,0.3)',
          }}>
            {isListening && <ListenDot frame={frame} />}
            {isRecording && <WaveformBars frame={frame} barCount={14} color={ACCENT} />}
            {isSuccess && (
              <div style={{ transform: `scale(${checkScale})`, color: '#4ade80', fontSize: 24, fontWeight: 'bold' }}>✓</div>
            )}
          </div>
        </div>

      </div>
    </AbsoluteFill>
  );
};
