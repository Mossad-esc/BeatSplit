'use client';

import { useState, useCallback } from 'react';
import { useWallet } from '@/components/WalletProvider';
import { buildDepositOp, signWithFreighter, getRpcClient, CONFIG } from '@/lib/stellar';
import { SorobanRpc, TransactionBuilder } from '@stellar/stellar-sdk';

interface Recipient {
  address: string;
  bps: number;
  label: string;
}

export default function CreateSplitPage() {
  const { address: walletAddress, signTransaction, connected, networkMismatch } = useWallet();
  const [recipients, setRecipients] = useState<Recipient[]>([
    { address: '', bps: 0, label: '' },
    { address: '', bps: 0, label: '' },
  ]);
  const [metadata, setMetadata] = useState({ title: '', isrc: '', notes: '' });
  const [tokenAddress, setTokenAddress] = useState(CONFIG.usdcContractId);
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [splitId, setSplitId] = useState<string | null>(null);
  const [errors, setErrors] = useState<string[]>([]);

  const totalBps = recipients.reduce((sum, r) => sum + r.bps, 0);
  const isValid = totalBps === 10000 && recipients.length >= 2 && recipients.every(r => r.address && r.bps > 0);

  const validateRecipients = useCallback(() => {
    const errs: string[] = [];
    const seen = new Set<string>();
    for (const r of recipients) {
      if (r.address && seen.has(r.address)) {
        errs.push(`Duplicate address: ${r.address}`);
      }
      if (r.address) seen.add(r.address);
    }
    if (recipients.length < 2) errs.push('At least 2 recipients required');
    if (recipients.length > 20) errs.push('Maximum 20 recipients allowed');
    if (totalBps !== 10000) errs.push(`Shares must sum to 100% (currently ${totalBps / 100}%)`);
    setErrors(errs);
    return errs.length === 0;
  }, [recipients, totalBps]);

  const addRecipient = () => {
    if (recipients.length >= 20) return;
    setRecipients([...recipients, { address: '', bps: 0, label: '' }]);
  };

  const removeRecipient = (index: number) => {
    if (recipients.length <= 2) return;
    setRecipients(recipients.filter((_, i) => i !== index));
  };

  const updateRecipient = (index: number, field: keyof Recipient, value: string | number) => {
    setRecipients(recipients.map((r, i) => i === index ? { ...r, [field]: value } : r));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!connected) {
      setMessage('Please connect your wallet first');
      setStatus('error');
      return;
    }
    if (networkMismatch) {
      setMessage('Wrong network. Please switch to testnet in Freighter.');
      setStatus('error');
      return;
    }
    if (!validateRecipients()) return;

    setStatus('submitting');
    setMessage('Creating split...');

    try {
      // In a real app, you'd call the contract via the RPC
      // For now, simulate the flow
      const metadataStr = JSON.stringify(metadata);
      const encoder = new TextEncoder();
      const data = encoder.encode(metadataStr);
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const metadataHash = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

      // This would be the actual contract call
      // const op = buildCreateSplitOp(walletAddress!, tokenAddress, recipients, metadataHash);
      // const tx = await simulateAndSend(walletAddress!, [op], signTransaction);

      // Simulate success for demo
      await new Promise(r => setTimeout(r, 1500));
      const mockId = Math.floor(Math.random() * 1000000);
      setSplitId(mockId.toString());
      setStatus('success');
      setMessage(`Split created successfully! Split ID: ${mockId}`);
    } catch (err) {
      setStatus('error');
      setMessage(`Failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const copyInviteLink = () => {
    if (splitId) {
      const url = `${window.location.origin}/split/${splitId}`;
      navigator.clipboard.writeText(url);
      setMessage('Invite link copied to clipboard!');
    }
  };

  return (
    <div className="page">
      <header className="header">
        <div className="header-content">
          <div className="logo">
            <span className="logo-icon">♪</span>
            BeatSplit
          </div>
        </div>
      </header>

      <main className="main">
        <div className="container">
          <h1 className="section-title">Create a New Split</h1>
          <p className="section-subtitle">
            Define who gets paid and how much. Everyone must accept before the split goes live.
          </p>

          {message && (
            <div className={status === 'error' ? 'error-message' : status === 'success' ? 'success-message' : ''}>
              {message}
            </div>
          )}

          {errors.length > 0 && (
            <div className="error-message">
              {errors.map((e, i) => <div key={i}>{e}</div>)}
            </div>
          )}

          <form onSubmit={handleSubmit}>
            <section className="section">
              <h2 className="section-title">Collaborators</h2>
              <p className="section-subtitle">
                Add each collaborator by their Stellar address. Shares are in basis points (10,000 = 100%).
              </p>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
                {recipients.map((r, i) => (
                  <div key={i} className="recipient-row" style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', flexWrap: 'wrap' }}>
                    <input
                      type="text"
                      placeholder="Stellar address (G...)"
                      value={r.address}
                      onChange={e => updateRecipient(i, 'address', e.target.value)}
                      aria-label={`Recipient ${i + 1} address`}
                      style={{ flex: '1', minWidth: '200px' }}
                    />
                    <input
                      type="number"
                      min="1"
                      max="9999"
                      placeholder="Share %"
                      value={r.bps}
                      onChange={e => updateRecipient(i, 'bps', parseInt(e.target.value) || 0)}
                      aria-label={`Recipient ${i + 1} share`}
                      style={{ width: '100px' }}
                    />
                    <span className="bps-label">{r.bps ? `${(r.bps / 100).toFixed(2)}%` : '0%'}</span>
                    <input
                      type="text"
                      placeholder="Role (producer, vocalist, etc.)"
                      value={r.label}
                      onChange={e => updateRecipient(i, 'label', e.target.value)}
                      aria-label={`Recipient ${i + 1} role`}
                      style={{ flex: '1', minWidth: '150px' }}
                    />
                    {recipients.length > 2 && (
                      <button
                        type="button"
                        className="btn btn-ghost"
                        onClick={() => removeRecipient(i)}
                        aria-label={`Remove recipient ${i + 1}`}
                      >
                        ×
                      </button>
                    )}
                  </div>
                ))}
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginTop: '1rem' }}>
                <button type="button" className="btn btn-secondary" onClick={addRecipient} disabled={recipients.length >= 20}>
                  + Add Collaborator
                </button>
                <span style={{ color: totalBps === 10000 ? 'var(--color-success)' : 'var(--color-text-muted)' }}>
                  Total: {totalBps / 100}% / 100%
                </span>
              </div>
            </section>

            <section className="section">
              <h2 className="section-title">Track Metadata (Optional)</h2>
              <p className="section-subtitle">This info is hashed and stored on-chain. It helps identify the track later.</p>

              <div className="form-group">
                <label htmlFor="title">Track Title</label>
                <input
                  id="title"
                  type="text"
                  placeholder="e.g., Summer Vibes (feat. Alex)"
                  value={metadata.title}
                  onChange={e => setMetadata({ ...metadata, title: e.target.value })}
                />
              </div>

              <div className="form-group">
                <label htmlFor="isrc">ISRC</label>
                <input
                  id="isrc"
                  type="text"
                  placeholder="e.g., USRC12345678"
                  value={metadata.isrc}
                  onChange={e => setMetadata({ ...metadata, isrc: e.target.value })}
                />
              </div>

              <div className="form-group">
                <label htmlFor="notes">Notes</label>
                <textarea
                  id="notes"
                  rows={3}
                  placeholder="Additional details, version info, etc."
                  value={metadata.notes}
                  onChange={e => setMetadata({ ...metadata, notes: e.target.value })}
                />
              </div>
            </section>

            <section className="section">
              <h2 className="section-title">Token</h2>
              <p className="section-subtitle">The token used for all payments into this split.</p>
              <div className="form-group">
                <label htmlFor="token">Token Contract Address</label>
                <div className="input-with-button">
                  <input
                    id="token"
                    type="text"
                    value={tokenAddress}
                    onChange={e => setTokenAddress(e.target.value)}
                    readOnly
                  />
                  <button type="button" className="btn btn-ghost copy-btn" onClick={() => navigator.clipboard.writeText(tokenAddress)}>
                    Copy
                  </button>
                </div>
                <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)', marginTop: '0.5rem' }}>
                  Default: USDC on Stellar testnet
                </p>
              </div>
            </section>

            <div style={{ display: 'flex', gap: '1rem', marginTop: '2rem' }}>
              <button
                type="submit"
                className="btn btn-primary"
                style={{ flex: 1 }}
                disabled={status === 'submitting' || !isValid}
              >
                {status === 'submitting' ? (
                  <span><span className="loading-spinner"></span> Creating... </span>
                ) : (
                  'Create Split'
                )}
              </button>
            </div>
          </form>

          {status === 'success' && splitId && (
            <section className="section" style={{ marginTop: '2rem', paddingTop: '1.5rem', borderTop: '1px solid var(--color-border)' }}>
              <h2 className="section-title">Split Created!</h2>
              <p className="section-subtitle">Share this link with your collaborators so they can accept their shares.</p>
              <div className="qr-code">
                <div style={{ width: '200px', height: '200px', background: 'var(--color-bg)', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: 'var(--radius)', border: '1px solid var(--color-border)' }}>
                  <span style={{ color: 'var(--color-text-muted)' }}>QR Code: /split/{splitId}</span>
                </div>
                <div className="address-display">
                  <strong>Split ID: </strong>
                  <code>{splitId}</code>
                </div>
                <button className="btn btn-secondary" onClick={copyInviteLink}>
                  Copy Invite Link
                </button>
              </div>
            </section>
          )}
        </div>
      </main>

      <footer className="footer">
        BeatSplit v0.1 — Unaudited. Testnet only.
      </footer>
    </div>
  );
}