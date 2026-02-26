import { useMemo } from 'react';
import { useStore } from '../store/useStore';

/**
 * Threat model recommendation #4: detect unscrubbed secrets in loaded .ghostline files.
 * Shows a persistent warning banner above the StatusBar when patterns are found.
 */

const SECRET_PATTERNS = [
  /sk-[A-Za-z0-9_-]{20,}/g,           // OpenAI / Anthropic keys
  /Bearer\s+[A-Za-z0-9._~+/=-]{20,}/gi,
  /AKIA[0-9A-Z]{16}/g,                 // AWS access key
  /api[_-]?key["\s:=]+[A-Za-z0-9_-]{16,}/gi,
  /ghp_[A-Za-z0-9]{36,}/g,            // GitHub PAT
  /sk_live_[A-Za-z0-9]{24,}/g,        // Stripe secret
  /re_[A-Za-z0-9_]{16,}/g,            // Resend
];

function scanForSecrets(text: string): boolean {
  return SECRET_PATTERNS.some((p) => {
    p.lastIndex = 0;
    return p.test(text);
  });
}

export function SecretsWarning() {
  const runs = useStore((s) => s.runs);
  const activeRunId = useStore((s) => s.activeRunId);
  const activeRun = runs.find((r) => r.id === activeRunId);

  const hasSecrets = useMemo(() => {
    if (!activeRun) return false;
    return activeRun.frames.some((frame) => {
      const reqStr = JSON.stringify(frame.request ?? '');
      const resStr = JSON.stringify(frame.response ?? '');
      return scanForSecrets(reqStr) || scanForSecrets(resStr);
    });
  }, [activeRun]);

  if (!hasSecrets) return null;

  return (
    <div
      role="alert"
      style={{
        background: 'rgba(239, 68, 68, 0.12)',
        borderTop: '1px solid rgba(239, 68, 68, 0.3)',
        padding: '6px 16px',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        fontSize: 12,
        color: '#F87171',
        fontFamily: 'var(--font-mono)',
        flexShrink: 0,
      }}
    >
      <span style={{ fontSize: 14 }}>⚠️</span>
      <span>
        This recording may contain <strong>unscrubbed API keys or secrets</strong>.
        Avoid sharing this file. Re-record with <code style={{ background: 'rgba(239,68,68,0.15)', padding: '1px 4px', borderRadius: 3 }}>scrub=True</code> (default).
      </span>
    </div>
  );
}
