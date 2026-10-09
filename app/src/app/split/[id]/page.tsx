'use client';

import { useState, useEffect } from 'react';
import { useParams } from 'next/navigation';
import { useWallet } from '@/components/WalletProvider';
import { getSplit, getClaimable, getEarned, buildAcceptOp, signWithFreighter, getRpcClient, CONFIG, formatAmount, bpsToPercent, computeSharePreview } from '@/lib/stellar';
import { SorobanRpc, TransactionBuilder } from '@stellar/stellar-sdk';

interface SplitData {
  id: number;
  creator: string;
  token: string;
  recipients: Array<{ address: string; bps: number }>;
  status: 'pending' | 'active' | 'locked';
  metadataHash: string;
  totalReceived: string;
  version: number;
}

export default function SplitPage() {
  const params = useParams();
  const splitId = params.id as string;
  const { address: walletAddress, connected, networkMismatch, signTransaction } = useWallet();

  const [split, setSplit] = useState<SplitData | null>(null);
  const [claimables, setClaimables] = useState<Map<string, string>>(new Map());
  const [earnings, setEarnings] = useState<Map<string, string>>(new Map());
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle');
  const [message, setMessage] = useState('');

  useEffect(() => {
    loadSplit();
  }, [splitId]);

  const loadSplit = async () => {
    setLoading(true);
    try {
      const data = await getSplit(parseInt(splitId));
      if (data) {
        setSplit(data);
        // Load claimable and earned for each recipient
        for (const r of data.recipients) {
          const [claimable, earned] = await Promise.all([
            getClaimable(data.id, r.address),
            getEarned(data.id, r.address),
          ]);
          setClaimables(prev => new Map(prev).set(r.address, claimable));
          setEarnings(prev => new Map(prev).set(r.address, earned));
        }
      }
    } catch (err) {
      console.error('Failed to load split:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleAccept = async () => {
    if (!connected || !walletAddress) {
      setMessage('Please connect your wallet first');
      setStatus('error');
      return;
    }
    if (networkMismatch) {
      setMessage('Wrong network. Please switch to testnet in Freighter.');
      setStatus('error');
      return;
    }
    if (!split) return;

    setStatus('submitting');
    setMessage('Accepting share...');

    try {
      const op = buildAcceptOp(split.id, walletAddress);
      const rpc = getRpcClient();
      const account = await rpc.getAccount(walletAddress);
      const tx = new TransactionBuilder(account, {
        fee: '1000000',
        networkPassphrase: CONFIG.networkPassphrase,
      })
        .addOperation(op)
        .setTimeout(300)
        .build();

      const simulateRes = await rpc.simulateTransaction(tx);
      if (SorobanRpc.Api.isSimulationError(simulateRes)) {
        throw new Error(`Simulation failed: ${simulateRes.error}`);
      }

      const prepared = SorobanRpc.assembleTransaction(tx, simulateRes).build();
      const signedTxXdr = await signTransaction(prepared.toEnvelope().toXDR('base64'));

      const sendRes = await rpc.sendTransaction(signedTxXdr as any);
      if (sendRes.status === 'ERROR') {
        throw new Error(String(sendRes.errorResult) || 'Transaction failed');
      }

      setStatus('success');
      setMessage('Share accepted! The split is now active.');
      loadSplit();
    } catch (err) {
      setStatus('error');
      setMessage(`Failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const copyInviteLink = () => {
    const url = window.location.href;
    navigator.clipboard.writeText(url);
    setMessage('Invite link copied!');
    setStatus('success');
  };

  if (loading) {
    return (
      <div className="page">
        <header className="header">
          <div className="header-content">
            <div className="logo"><span className="logo-icon">♪</span> BeatSplit</div>
          </div>
        </header>
        <main className="main">
          <div className="container" style={{ textAlign: 'center', paddingTop: '4rem' }}>
            <span className="loading-spinner" style={{ margin: '0 auto' }}></span>
            <p style={{ marginTop: '1rem', color: 'var(--color-text-muted)' }}>Loading split...</p>
          </div>
        </main>
      </div>
    );
  }

  if (!split) {
    return (
      <div className="page">
        <header className="header">
          <div className="header-content">
            <div className="logo"><span className="logo-icon">♪</span> BeatSplit</div>
          </div>
        </header>
        <main className="main">
          <div className="container" style={{ textAlign: 'center', paddingTop: '4rem' }}>
            <h1>Split Not Found</h1>
            <p style={{ color: 'var(--color-text-muted)', marginTop: '1rem' }}>
              Split ID "{splitId}" does not exist.
            </p>
          </div>
        </main>
      </div>
    );
  }

  const isRecipient = walletAddress && split.recipients.some(r => r.address === walletAddress);
  const hasAccepted = walletAddress ? (claimables.get(walletAddress) !== undefined || earnings.get(walletAddress) !== undefined) : false;
  const isActive = split.status === 'active';
  const isLocked = split.status === 'locked';
  const isPending = split.status === 'pending';

  const sharePreviews = computeSharePreview(split.totalReceived || '0', split.recipients);

  return (
    <div className="page">
      <header className="header">
        <div className="header-content">
          <div className="logo"><span className="logo-icon">♪</span> BeatSplit</div>
        </div>
      </header>

      <main className="main">
        <div className="container">
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1rem' }}>
            <h1 className="section-title" style={{ marginBottom: 0 }}>Split #{split.id}</h1>
            <span className={`badge badge-${split.status}`}>{split.status}</span>
          </div>

          {message && (
            <div className={status === 'error' ? 'error-message' : 'success-message'}>
              {message}
            </div>
          )}

          {/* Split Summary */}
          <section className="section">
            <h2 className="section-title">Overview</h2>
            <div style={{ display: 'grid', gap: '1rem', gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))' }}>
              <div className="card" style={{ padding: '1rem' }}>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Total Received</div>
                <div style={{ fontSize: '1.5rem', fontWeight: '600' }}>{formatAmount(split.totalReceived || '0')} USDC</div>
              </div>
              <div className="card" style={{ padding: '1rem' }}>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Recipients</div>
                <div style={{ fontSize: '1.5rem', fontWeight: '600' }}>{split.recipients.length}</div>
              </div>
              <div className="card" style={{ padding: '1rem' }}>
                <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Version</div>
                <div style={{ fontSize: '1.5rem', fontWeight: '600' }}>{split.version}</div>
              </div>
            </div>
          </section>

          {/* Who receives what */}
          <section className="section">
            <h2 className="section-title">Who Receives What</h2>
            <p className="section-subtitle">
              Based on the last deposit of {formatAmount(split.totalReceived || '0')} USDC
            </p>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
              {split.recipients.map((r, i) => {
                const preview = sharePreviews.find(p => p.address === r.address);
                const claimable = claimables.get(r.address) || '0';
                const earned = earnings.get(r.address) || '0';
                const isCurrentUser = walletAddress === r.address;

                return (
                  <div key={i} className="card" style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
                    <div style={{ flex: 1, minWidth: '150px' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <span className="address-display" style={{ padding: '0.25rem 0.5rem', fontSize: '0.8125rem' }}>
                          {r.address.slice(0, 6)}...{r.address.slice(-4)}
                        </span>
                        {isCurrentUser && <span className="badge badge-active" style={{ fontSize: '0.625rem' }}>You</span>}
                      </div>
                      <div style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)', marginTop: '0.25rem' }}>
                        {preview?.percent} ({r.bps} bps)
                      </div>
                    </div>
                    <div style={{ textAlign: 'right', minWidth: '140px' }}>
                      <div style={{ fontWeight: '600', fontSize: '1.125rem' }}>{preview?.share || '0'} USDC</div>
                      <div style={{ fontSize: '0.75rem', color: 'var(--color-text-muted)' }}>
                        Earned: {formatAmount(earned)} USDC
                      </div>
                      {claimable !== '0' && (
                        <div style={{ fontSize: '0.75rem', color: 'var(--color-warning)' }}>
                          Claimable: {formatAmount(claimable)} USDC
                        </div>
                      )}
                    </div>
                    {isCurrentUser && !isActive && isPending && (
                      <button className="btn btn-primary" onClick={handleAccept} disabled={status === 'submitting'}>
                        {status === 'submitting' ? <span className="loading-spinner"></span> : 'Accept Share'}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          </section>

          {/* Acceptance Status */}
          {isPending && (
            <section className="section">
              <h2 className="section-title">Acceptance Status</h2>
              <p className="section-subtitle">
                All recipients must accept before the split activates and receives payments.
              </p>
              <ul style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                {split.recipients.map((r, i) => (
                  <li key={i} style={{ display: 'flex', alignItems: 'center', gap: '1rem', padding: '0.75rem', background: 'var(--color-bg)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--color-border)' }}>
                    <span className="address-display" style={{ flex: 1 }}>
                      {r.address.slice(0, 6)}...{r.address.slice(-4)}
                    </span>
                    <span className={`badge ${isActive ? 'badge-active' : 'badge-pending'}`}>
                      {isActive ? 'Accepted' : 'Pending'}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {/* Not a recipient / Wallet not connected */}
          {!isRecipient && walletAddress && (
            <section className="section">
              <div className="card" style={{ textAlign: 'center', padding: '2rem' }}>
                <h3>You're not a recipient of this split</h3>
                <p style={{ color: 'var(--color-text-muted)', marginTop: '0.5rem' }}>
                  Your address: <code style={{ background: 'var(--color-bg)', padding: '0.25rem 0.5rem', borderRadius: '4px' }}>{walletAddress.slice(0, 6)}...{walletAddress.slice(-4)}</code>
                </p>
                <p style={{ marginTop: '1rem', fontSize: '0.9375rem' }}>
                  Ask the split creator to add you as a collaborator, or use this link to pay into the split.
                </p>
              </div>
            </section>
          )}

          {!connected && (
            <section className="section">
              <div className="card" style={{ textAlign: 'center', padding: '2rem' }}>
                <h3>Connect your wallet to interact</h3>
                <p style={{ color: 'var(--color-text-muted)', marginTop: '0.5rem' }}>
                  You need a Stellar wallet (Freighter) to accept shares or claim payments.
                </p>
              </div>
            </section>
          )}

          {/* Invite Link */}
          <section className="section">
            <h2 className="section-title">Share This Split</h2>
            <div className="card">
              <div className="address-display" style={{ justifyContent: 'space-between' }}>
                <code>{window.location.href}</code>
                <button className="btn btn-ghost copy-btn" onClick={copyInviteLink}>
                  Copy
                </button>
              </div>
              <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)', marginTop: '0.75rem' }}>
                Send this link to collaborators. They'll connect their wallet and accept their share.
              </p>
            </div>
          </section>
        </div>
      </main>

      <footer className="footer">
        BeatSplit v0.1 — Unaudited. Testnet only.
      </footer>
    </div>
  );
}