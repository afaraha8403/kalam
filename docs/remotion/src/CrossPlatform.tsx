import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from 'remotion';

/**
 * "Every Platform" — 3D perspective showcase of Kalam working across different apps and OSes.
 *
 * Flow:
 * 1. App windows (Slack, Word, PowerPoint, Notepad) flow onto screen one-by-one with 3D depth,
 *    each at a different height/rotation to create a layered, dynamic composition.
 * 2. Each window shows dictated text appearing live with the Kalam pill active.
 * 3. OS logos (Windows, macOS, Linux) appear at the bottom with animated checkmarks.
 * 4. Outro: "Works everywhere" tagline.
 *
 * Color scheme matches the other two animations (dark desktop, accent blues/greens).
 */

const ACCENT = '#4fc1ff';

/** Simplified app window representations — each mimics a real app's look */
const appWindows = [
  {
    id: 'slack',
    name: 'Slack',
    titleBarColor: '#1a1030',
    bodyColor: '#1e1e2e',
    accentColor: '#4A154B',
    icon: '💬',
    text: 'Hey team, let\'s sync up on the roadmap this afternoon.',
  },
  {
    id: 'word',
    name: 'Microsoft Word',
    titleBarColor: '#1a2540',
    bodyColor: '#1e1e2e',
    accentColor: '#185ABD',
    icon: '📄',
    text: 'The quarterly report shows a 23% increase in user engagement across all platforms.',
  },
  {
    id: 'powerpoint',
    name: 'PowerPoint',
    titleBarColor: '#2a1a1a',
    bodyColor: '#1e1e2e',
    accentColor: '#D24726',
    icon: '📊',
    text: 'Next slide: Revenue projections for Q3 and Q4.',
  },
  {
    id: 'notepad',
    name: 'Notepad',
    titleBarColor: '#2c2c3a',
    bodyColor: '#1e1e2e',
    accentColor: '#F59E0B',
    icon: '📝',
    text: 'Pick up groceries, call dentist, review PR #847.',
  },
];

const osLogos = [
  { name: 'Windows', color: '#3b82f6' },
  { name: 'macOS', color: '#a78bfa' },
  { name: 'Linux', color: '#f59e0b' },
];

/** SVG-based OS logo representations */
const OsLogo: React.FC<{ os: string; color: string; size: number }> = ({ os, color, size }) => {
  if (os === 'Windows') {
    return (
      <svg width={size} height={size} viewBox="0 0 40 40">
        <rect x="2" y="2" width="16" height="16" rx="2" fill={color} opacity="0.9" />
        <rect x="22" y="2" width="16" height="16" rx="2" fill={color} opacity="0.7" />
        <rect x="2" y="22" width="16" height="16" rx="2" fill={color} opacity="0.7" />
        <rect x="22" y="22" width="16" height="16" rx="2" fill={color} opacity="0.5" />
      </svg>
    );
  }
  if (os === 'macOS') {
    return (
      <svg width={size} height={size} viewBox="0 0 40 40">
        <path d="M28 30c-1.5 2-3 3-5 3-2 0-2.5-1.2-5-1.2s-3.2 1.2-5 1.2c-2 0-3.8-1.5-5.2-3.5C5.5 26 4 21 6 17c1.2-2.5 3.5-4 6-4 2.2 0 3.8 1.3 5 1.3 1.2 0 3-1.3 5.5-1.3 2 0 4.2 1 5.5 3-3 1.8-2.5 6.5.5 8.5C27.5 27 27 28.5 28 30z M24 7c-3 .3-5.5 3.5-5 6.5 3 .2 5.5-3 5-6.5z"
          fill={color} />
      </svg>
    );
  }
  // Linux (Tux-inspired)
  return (
    <svg width={size} height={size} viewBox="0 0 40 40">
      <ellipse cx="20" cy="22" rx="10" ry="14" fill={color} opacity="0.8" />
      <ellipse cx="20" cy="14" rx="8" ry="9" fill={color} />
      <circle cx="16" cy="12" r="2" fill="#fff" />
      <circle cx="24" cy="12" r="2" fill="#fff" />
      <circle cx="16" cy="12" r="1" fill="#111" />
      <circle cx="24" cy="12" r="1" fill="#111" />
      <ellipse cx="20" cy="16" rx="3" ry="1.5" fill="#F59E0B" />
      <ellipse cx="14" cy="34" rx="4" ry="2.5" fill={color} opacity="0.6" />
      <ellipse cx="26" cy="34" rx="4" ry="2.5" fill={color} opacity="0.6" />
    </svg>
  );
};

export const CrossPlatform: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  /**
   * Timeline:
   * 0-20:   Title fades in
   * 15-120: App windows flow in one-by-one with staggered 3D entrance
   * 60-200: Text types into the currently "active" window (cycles through them)
   * 140-200: OS logos appear with checkmarks
   * 220+:   Outro
   */

  const titleOpacity = interpolate(frame, [0, 20], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const titleY = interpolate(frame, [0, 20], [20, 0], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });

  /** Which app window is "active" (receiving dictation) — cycles every ~40 frames */
  const activeWindowIndex = Math.min(
    Math.floor(interpolate(frame, [60, 200], [0, 4], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' })),
    3
  );

  /** Text typing progress per window */
  const getTextProgress = (windowIndex: number) => {
    const startFrame = 60 + windowIndex * 35;
    const endFrame = startFrame + 32;
    return interpolate(frame, [startFrame, endFrame], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  };

  /** OS checkmarks */
  const osStartFrame = 150;

  const outroOpacity = interpolate(frame, [230, 250], [0, 1], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' });
  const outroScale = spring({ frame: frame - 230, fps, config: { damping: 200 } });

  return (
    <AbsoluteFill style={{
      backgroundColor: '#111318',
      fontFamily: "'Segoe UI', 'SF Pro Display', system-ui, sans-serif",
    }}>
      <div style={{ width: '100%', height: '100%', position: 'relative', overflow: 'hidden' }}>

        {/* Desktop wallpaper — matches other animations */}
        <div style={{
          position: 'absolute', inset: 0,
          background: 'linear-gradient(135deg, #1a2540 0%, #1e2d4a 30%, #162238 70%, #1a2744 100%)',
        }} />
        <div style={{
          position: 'absolute', inset: 0, opacity: 0.04,
          backgroundImage: 'linear-gradient(rgba(255,255,255,0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.1) 1px, transparent 1px)',
          backgroundSize: '60px 60px',
        }} />

        {/* ===== TITLE ===== */}
        <div style={{
          position: 'absolute', top: 30, width: '100%', textAlign: 'center', zIndex: 10,
          opacity: titleOpacity, transform: `translateY(${titleY}px)`,
        }}>
          <div style={{
            color: 'rgba(255,255,255,0.4)', fontSize: 15, fontWeight: 600,
            letterSpacing: '0.15em', textTransform: 'uppercase', marginBottom: 6,
          }}>
            One voice. Every platform.
          </div>
          <div style={{ color: '#fff', fontSize: 40, fontWeight: 800, letterSpacing: '-0.03em' }}>
            Dictate into <span style={{ color: ACCENT }}>any app</span>, on <span style={{ color: ACCENT }}>any OS</span>
          </div>
        </div>

        {/* ===== 3D APP WINDOWS — flowing in with perspective ===== */}
        <div style={{
          position: 'absolute', top: 120, left: 0, right: 0, bottom: 140,
          perspective: '1800px',
          perspectiveOrigin: '50% 40%',
        }}>
          {appWindows.map((app, i) => {
            const staggerDelay = i * 18;
            const cardEntrance = spring({
              frame: frame - 15 - staggerDelay,
              fps,
              config: { damping: 16, stiffness: 80 },
            });

            /** Each card has a unique 3D position — stacked with offset, different depths */
            const positions = [
              { x: -320, y: 0, z: 0, rotY: 12, rotX: -3 },
              { x: 120, y: 30, z: -80, rotY: -5, rotX: -2 },
              { x: -180, y: 280, z: -40, rotY: 8, rotX: -1 },
              { x: 260, y: 260, z: -120, rotY: -10, rotX: -2 },
            ];
            const pos = positions[i];

            const isActive = activeWindowIndex === i && frame >= 60;
            const textProgress = getTextProgress(i);
            const charsToShow = Math.floor(textProgress * app.text.length);

            /** Subtle float animation when active */
            const floatY = isActive ? Math.sin(frame * 0.08) * 3 : 0;

            /** Glow when active */
            const glowIntensity = isActive ? 0.15 : 0;

            const cardW = 580;
            const cardH = 340;

            return (
              <div
                key={app.id}
                style={{
                  position: 'absolute',
                  left: '50%',
                  top: 0,
                  width: cardW,
                  height: cardH,
                  transform: `
                    translateX(calc(-50% + ${pos.x}px))
                    translateY(${pos.y + interpolate(cardEntrance, [0, 1], [120, 0]) + floatY}px)
                    translateZ(${pos.z}px)
                    rotateY(${pos.rotY}deg)
                    rotateX(${pos.rotX}deg)
                    scale(${interpolate(cardEntrance, [0, 1], [0.85, 1])})
                  `,
                  opacity: cardEntrance,
                  transformStyle: 'preserve-3d',
                  zIndex: isActive ? 10 : 4 - i,
                }}
              >
                <div style={{
                  width: '100%', height: '100%',
                  borderRadius: 16, overflow: 'hidden',
                  border: `1px solid ${isActive ? `${ACCENT}40` : 'rgba(255,255,255,0.1)'}`,
                  boxShadow: isActive
                    ? `0 25px 60px rgba(0,0,0,0.5), 0 0 40px rgba(79, 193, 255, ${glowIntensity})`
                    : '0 20px 50px rgba(0,0,0,0.4)',
                  display: 'flex', flexDirection: 'column',
                }}>
                  {/* Title bar */}
                  <div style={{
                    height: 42, backgroundColor: app.titleBarColor,
                    display: 'flex', alignItems: 'center', padding: '0 16px', gap: 10,
                    borderBottom: '1px solid rgba(255,255,255,0.06)',
                    flexShrink: 0,
                  }}>
                    <div style={{ display: 'flex', gap: 6 }}>
                      <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#ff5f57' }} />
                      <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#febc2e' }} />
                      <div style={{ width: 12, height: 12, borderRadius: '50%', backgroundColor: '#28c840' }} />
                    </div>
                    <div style={{
                      flex: 1, textAlign: 'center',
                      color: 'rgba(255,255,255,0.5)', fontSize: 13, fontWeight: 500,
                      display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
                    }}>
                      <span>{app.icon}</span>
                      <span>{app.name}</span>
                    </div>
                    {/* App accent stripe */}
                    <div style={{
                      width: 4, height: 20, borderRadius: 2,
                      backgroundColor: app.accentColor, opacity: 0.6,
                    }} />
                  </div>

                  {/* Body */}
                  <div style={{
                    flex: 1, backgroundColor: app.bodyColor,
                    padding: '24px 28px',
                    display: 'flex', flexDirection: 'column', justifyContent: 'center',
                  }}>
                    {/* App-specific UI hint */}
                    {app.id === 'slack' && (
                      <div style={{ display: 'flex', gap: 10, marginBottom: 16 }}>
                        <div style={{
                          width: 32, height: 32, borderRadius: 6,
                          backgroundColor: '#4A154B', display: 'flex', alignItems: 'center', justifyContent: 'center',
                          color: '#fff', fontSize: 14, fontWeight: 700,
                        }}>Y</div>
                        <div>
                          <div style={{ color: '#fff', fontSize: 13, fontWeight: 700 }}>You <span style={{ color: 'rgba(255,255,255,0.3)', fontWeight: 400 }}>in #product</span></div>
                        </div>
                      </div>
                    )}
                    {app.id === 'word' && (
                      <div style={{ marginBottom: 12 }}>
                        <div style={{
                          display: 'inline-flex', gap: 12, padding: '4px 10px',
                          borderRadius: 4, backgroundColor: 'rgba(24, 90, 189, 0.15)',
                          marginBottom: 12,
                        }}>
                          {['B', 'I', 'U'].map((b) => (
                            <span key={b} style={{ color: 'rgba(255,255,255,0.4)', fontSize: 12, fontWeight: b === 'B' ? 800 : 400, fontStyle: b === 'I' ? 'italic' : 'normal', textDecoration: b === 'U' ? 'underline' : 'none' }}>{b}</span>
                          ))}
                        </div>
                      </div>
                    )}
                    {app.id === 'powerpoint' && (
                      <div style={{
                        width: '100%', height: 6, borderRadius: 3,
                        backgroundColor: 'rgba(210, 71, 38, 0.2)', marginBottom: 16,
                        position: 'relative', overflow: 'hidden',
                      }}>
                        <div style={{
                          width: '60%', height: '100%', borderRadius: 3,
                          backgroundColor: '#D24726', opacity: 0.7,
                        }} />
                      </div>
                    )}

                    {/* Dictated text */}
                    <div style={{
                      color: '#e2e8f0', fontSize: 19, lineHeight: 1.65,
                      fontFamily: app.id === 'notepad' ? "'Consolas', 'Courier New', monospace" : 'inherit',
                    }}>
                      {app.text.slice(0, charsToShow)}
                      {isActive && textProgress < 1 && (
                        <span style={{
                          display: 'inline-block', width: 2, height: 20,
                          backgroundColor: ACCENT, marginLeft: 2,
                          opacity: frame % 30 < 15 ? 1 : 0.2,
                          verticalAlign: 'text-bottom',
                        }} />
                      )}
                      {charsToShow === 0 && !isActive && (
                        <span style={{ color: 'rgba(255,255,255,0.15)' }}>Start dictating...</span>
                      )}
                      {textProgress >= 1 && (
                        <span style={{ color: '#4ade80', marginLeft: 8, fontSize: 16 }}>✓</span>
                      )}
                    </div>

                    {/* Kalam pill indicator at bottom of active window */}
                    {isActive && (
                      <div style={{
                        marginTop: 'auto', paddingTop: 16,
                        display: 'flex', alignItems: 'center', gap: 8,
                      }}>
                        <div style={{
                          display: 'flex', alignItems: 'center', gap: 3,
                          padding: '6px 14px', borderRadius: 100,
                          backgroundColor: '#0a0a0c',
                          border: `1px solid ${ACCENT}30`,
                        }}>
                          {Array.from({ length: 6 }, (_, j) => {
                            const h = 3 + Math.abs(Math.sin(frame * 0.2 + j * 1.1)) * 10;
                            return <div key={j} style={{ width: 2.5, height: h, borderRadius: 2, backgroundColor: ACCENT, opacity: 0.85 }} />;
                          })}
                          <span style={{ color: ACCENT, fontSize: 11, fontWeight: 600, marginLeft: 6 }}>Kalam</span>
                        </div>
                        <span style={{ color: 'rgba(255,255,255,0.3)', fontSize: 12 }}>dictating...</span>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* ===== OS LOGOS WITH CHECKMARKS ===== */}
        <div style={{
          position: 'absolute', bottom: 50, left: 0, right: 0,
          display: 'flex', justifyContent: 'center', gap: 60, zIndex: 20,
        }}>
          {osLogos.map((os, i) => {
            const osEntrance = spring({
              frame: frame - osStartFrame - (i * 12),
              fps,
              config: { damping: 14, stiffness: 120 },
            });
            const checkEntrance = spring({
              frame: frame - osStartFrame - (i * 12) - 10,
              fps,
              config: { damping: 10, stiffness: 200 },
            });

            return (
              <div
                key={os.name}
                style={{
                  display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10,
                  opacity: osEntrance,
                  transform: `translateY(${interpolate(osEntrance, [0, 1], [30, 0])}px)`,
                }}
              >
                <div style={{
                  width: 64, height: 64, borderRadius: 16,
                  backgroundColor: 'rgba(15, 21, 34, 0.8)',
                  border: `1px solid ${os.color}30`,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  position: 'relative',
                }}>
                  <OsLogo os={os.name} color={os.color} size={36} />

                  {/* Checkmark badge */}
                  <div style={{
                    position: 'absolute', top: -6, right: -6,
                    width: 22, height: 22, borderRadius: '50%',
                    backgroundColor: '#4ade80',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    transform: `scale(${checkEntrance})`,
                    boxShadow: '0 2px 8px rgba(74, 222, 128, 0.3)',
                  }}>
                    <span style={{ color: '#000', fontSize: 12, fontWeight: 800 }}>✓</span>
                  </div>
                </div>
                <span style={{ color: os.color, fontSize: 14, fontWeight: 600 }}>{os.name}</span>
              </div>
            );
          })}
        </div>

        {/* ===== OUTRO OVERLAY ===== */}
        {frame >= 230 && (
          <div style={{
            position: 'absolute', inset: 0,
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
            opacity: outroOpacity, zIndex: 30,
            backgroundColor: `rgba(10, 14, 23, ${interpolate(frame, [230, 250], [0, 0.92], { extrapolateRight: 'clamp', extrapolateLeft: 'clamp' })})`,
          }}>
            <div style={{
              transform: `scale(${interpolate(outroScale, [0, 1], [0.9, 1])})`,
              display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18,
            }}>
              <div style={{
                display: 'flex', gap: 20, marginBottom: 8,
              }}>
                {osLogos.map((os) => (
                  <OsLogo key={os.name} os={os.name} color={os.color} size={48} />
                ))}
              </div>
              <div style={{ color: '#fff', fontSize: 56, fontWeight: 800, letterSpacing: '-0.03em' }}>
                Works everywhere.
              </div>
              <div style={{
                color: 'rgba(255,255,255,0.45)', fontSize: 22, fontWeight: 500,
                maxWidth: 650, textAlign: 'center', lineHeight: 1.5,
              }}>
                Windows, macOS, Linux — Kalam speaks your language on every platform.
              </div>
            </div>
          </div>
        )}

      </div>
    </AbsoluteFill>
  );
};
