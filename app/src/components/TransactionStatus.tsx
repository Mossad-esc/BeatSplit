'use client';

import { useState, useEffect } from 'react';
import { SorobanRpc } from '@stellar/stellar-sdk';

type Status = 'idle' | 'pending' | 'success' | 'failed';

interface TransactionStatusProps {
  status: Status;
  message?: string;
  onClose?: () => void;
  txHash?: string;
}

const STATUS_CONFIG: Record<Exclude<Status, 'idle'>, { color: string; icon: string; label: string }> = {
  pending: { color: 'var(--color-primary)', icon: '⏳', label: 'Pending' },
  success: { color: 'var(--color-success)', icon: '✅', label: 'Success' },
  failed: { color: 'var(--color-danger)', icon: '❌', label: 'Failed' },
};

export function TransactionStatus({ status, message, onClose, txHash }: TransactionStatusProps) {
  const [dots, setDots] = useState('');

  useEffect(() => {
    if (status !== 'pending') return;
    const interval = setInterval(() => {
      setDots(prev => (prev.length >= 3 ? '' : prev + '.'));
    }, 500);
    return () => clearInterval(interval);
  }, [status]);

  if (status === 'idle') return null;

  const config = STATUS_CONFIG[status];

  return (
    <div
      style={{
        position: 'fixed',
        bottom: '1.5rem',
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 1000,
        animation: 'slideUp 0.3s ease',
      }}
      className="toast"
      role="alert"
      aria-live="polite"
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '0.75rem',
          padding: '1rem 1.5rem',
          borderRadius: 'var(--radius)',
          background: 'var(--color-bg-elevated)',
          border: `1px solid ${config.color}`,
          boxShadow: 'var(--shadow)',
          minWidth: '280px',
          maxWidth: 'calc(100vw - 2rem)',
        }}
      >
        <span style={{ fontSize: '1.5rem' }}>{config.icon}</span>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: '600', color: config.color }}>{config.label}{dots}</div>
          {message && (
            <div style={{ fontSize: '0.875rem', color: 'var(--color-text-muted)', marginTop: '0.25rem' }}>
              {message}
            </div>
          )}
        </div>
        {onClose && (
          <button
            onClick={onClose}
            style={{
              background: 'none',
              border: 'none',
              color: 'var(--color-text-muted)',
              fontSize: '1.25rem',
              cursor: 'pointer',
              padding: '0.25rem',
              lineHeight: 1,
            }}
            aria-label="Dismiss"
          >
            ×
          </button>
        )}
      </div>
      <style jsx>{`
        @keyframes slideUp {
          from { opacity: 0; transform: translateX(-50%) translateY(1rem); }
          to { opacity: 1; transform: translateX(-50%) translateY(0); }
        }
        @media (max-width: 480px) {
          .toast { left: 1rem; right: 1rem; transform: none; }
          @keyframes slideUp {
            from { opacity: 0; transform: translateY(1rem); }
            to { opacity: 1; transform: translateY(0); }
          }
        }
      `}</style>
    </div>
  );
}

// Hook for managing transaction status with contract error decoding
export function useTransactionStatus() {
  const [status, setStatus] = useState<Status>('idle');
  const [message, setMessage] = useState('');

  const reset = () => setStatus('idle');

  const start = (msg?: string) => {
    setStatus('pending');
    setMessage(msg || 'Submitting transaction...');
  };

  const succeed = (msg?: string) => {
    setStatus('success');
    setMessage(msg || 'Transaction successful!');
  };

  const fail = (error: unknown, contractErrors?: Record<number, string>) => {
    setStatus('failed');
    let msg = 'Transaction failed';

    if (error instanceof Error) {
      // Try to extract contract error code from error message
      const match = error.message.match(/Error\(Contract, #(\d+)\)/);
      if (match && contractErrors) {
        const code = parseInt(match[1]);
        msg = contractErrors[code] || `Contract error #${code}`;
      } else {
        msg = error.message;
      }
    } else if (typeof error === 'string') {
      msg = error;
    }

    setMessage(msg);
  };

  return { status, message, start, succeed, fail, reset };
}