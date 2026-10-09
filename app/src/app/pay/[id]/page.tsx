'use client';

import { useState, useEffect } from 'react';
import { useParams } from 'next/navigation';
import { useWallet } from '@/components/WalletProvider';
import { getSplit, buildDepositOp, signWithFreighter, getRpcClient, CONFIG, formatAmount, parseAmount, computeSharePreview, bpsToPercent } from '@/lib/stellar';
import { SorobanRpc, TransactionBuilder } from '@stellar/stellar-sdk';
import { QRCodeSVG } from 'qrcode.react';

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

export default function PaySplitPage() {
  const params = useParams();
  const splitId = params.id as string;
  const { address: walletAddress, connected, networkMismatch, signTransaction } = useWallet();

  const [split, setSplit] = useState<SplitData | null>(null);
  const [loading, setLoading] = useState(true);
  const [status, setStatus] = useState<'idle' | 'submitting' | 'success' | 'error'>('idle');
  const [message, setMessage] = useState('');
  const [amount, setAmount] = useState('');
  const [sharePreviews, setSharePreviews] = useState<Array<{ address: string; share: string; percent: string }>>([]);

  useEffect(() => {
    loadSplit();
  }, [splitId]);

  const loadSplit = async () => {
    setLoading(true);
    try {
      const data = await getSplit(parseInt(splitId));
      if (data) {
        setSplit(data);
        if (data.totalReceived && data.totalReceived !== '0') {
          setSharePreviews(computeSharePreview(data.totalReceived, data.recipients));
        }
      }
    } catch (err) {
      console.error('Failed to load split:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (split && amount) {
      setSharePreviews(computeSharePreview(amount, split.recipients));
    }
  }, [amount, split]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
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

    const amt = parseAmount(amount);
    if (BigInt(amt) <= 0) {
      setMessage('Amount must be greater than 0');
      setStatus('error');
      return;
    }

    setStatus('submitting');
    setMessage('Processing payment...');

    try {
      const op = buildDepositOp(split.id, walletAddress, amt);
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
      setMessage(`Payment sent! ${formatAmount(amt)} USDC distributed to ${split.recipients.length} recipients.`);
      setAmount('');
      loadSplit();
    } catch (err) {
      setStatus('error');
      setMessage(`Failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
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

  const isPending = split.status === 'pending';
  const isLocked = split.status === 'locked';
  const canPay = split.status === 'active' || split.status === 'locked';

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
            <h1 className="section-title" style={{ marginBottom: 0 }}>Pay Split #{split.id}</h1>
            <span className={`badge badge-${split.status}`}>{split.status}</span>
          </div>

          {message && (
            <div className={status === 'error' ? 'error-message' : 'success-message'}>
              {message}
            </div>
          )}

          {/* Split Summary */}
          <section className="section">
            <h2 className="section-title">Split Summary</h2>
            <div className="card">
              <div style={{ display: 'grid', gap: '1rem', gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))' }}>
                <div>
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Recipients</div>
                  <div style={{ fontWeight: '600' }}>{split.recipients.length} collaborators</div>
                </div>
                <div>
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Total Previously Received</div>
                  <div style={{ fontWeight: '600' }}>{formatAmount(split.totalReceived || '0')} USDC</div>
                </div>
                <div>
                  <div style={{ color: 'var(--color-text-muted)', fontSize: '0.875rem' }}>Version</div>
                  <div style={{ fontWeight: '600' }}>{split.version}</div>
                </div>
              </div>
            </div>
          </section>

          {/* Who receives what preview */}
          <section className="section">
            <h2 className="section-title">Who Receives What</h2>
            <p className="section-subtitle">
              {amount ? `Preview for ${formatAmount(parseAmount(amount))} USDC` : 'Enter an amount to see the breakdown'}
            </p>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              {split.recipients.map((r, i) => {
                const preview = sharePreviews.find(p => p.address === r.address);
                return (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0.75rem', background: 'var(--color-bg)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--color-border)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                      <span className="address-display" style={{ padding: '0.25rem 0.5rem', fontSize: '0.8125rem' }}>
                        {r.address.slice(0, 6)}...{r.address.slice(-4)}
                      </span>
                      <span style={{ fontSize: '0.875rem', color: 'var(--color-text-muted)' }}>
                        {bpsToPercent(r.bps)}
                      </span>
                    </div>
                    <div style={{ fontWeight: '600', fontSize: '1rem', minWidth: '100px', textAlign: 'right' }}>
                      {preview?.share ? formatAmount(preview.share) + ' USDC' : '—'}
                    </div>
                  </div>
                );
              })}
            </div>
          </section>

          {/* Payment Form */}
          <section className="section">
            <h2 className="section-title">Make a Payment</h2>
            {isPending && (
              <div className="error-message">
                ⚠️ This split is still pending acceptance. Payments will fail until all recipients accept.
              </div>
            )}
            {!canPay && !isPending && (
              <div className="error-message">
                This split cannot accept payments.
              </div>
            )}

            {canPay && (
              <form onSubmit={handleSubmit}>
                <div className="form-group">
                  <label htmlFor="amount">Amount (USDC)</label>
                  <div className="input-with-button">
                    <input
                      id="amount"
                      type="text"
                      placeholder="e.g., 10.00"
                      value={amount}
                      onChange={e => setAmount(e.target.value)}
                      inputMode="decimal"
                      disabled={status === 'submitting'}
                    />
                  </div>
                  <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)', marginTop: '0.5rem' }}>
                    7 decimals supported (0.0000001 USDC minimum)
                  </p>
                </div>

                <div style={{ display: 'flex', gap: '1rem', marginTop: '1.5rem' }}>
                  <button
                    type="submit"
                    className="btn btn-primary"
                    style={{ flex: 1 }}
                    disabled={status === 'submitting' || !amount}
                  >
                    {status === 'submitting' ? (
                      <span><span className="loading-spinner"></span> Paying... </span>
                    ) : (
                      'Pay Now'
                    )}
                  </button>
                </div>

                <div className="card" style={{ marginTop: '1.5rem', background: 'rgba(255, 165, 2, 0.1)', borderColor: 'var(--color-warning)' }}>
                  <h4 style={{ color: 'var(--color-warning)', marginBottom: '0.5rem' }}>⚠️ Important</h4>
                  <ul style={{ fontSize: '0.875rem', color: 'var(--color-text-muted)', paddingLeft: '1.25rem' }}>
                    <li>Payments are final and cannot be reversed.</li>
                    <li>Funds are distributed immediately to all recipients.</li>
                    <li>If a recipient's wallet has issues, their share is held for later claim.</li>
                    <li>Only pay into splits you trust. Verify the split ID and recipients.</li>
                  </ul>
                </div>
              </form>
            )}
          </section>

          {/* QR Code for easy payment */}
          <section className="section">
            <h2 className="section-title">Scan to Pay</h2>
            <p className="section-subtitle">Share this QR code for easy mobile payments</p>
            <div className="qr-code">
              <div style={{ width: '200px', height: '200px', background: 'white', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: 'var(--radius)', padding: '1rem' }}>
                <QRCodeSVG
                  value={window.location.href}
                  size={180}
                  level="M"
                  includeMargin={true}
                />
              </div>
              <div className="address-display">
                <code>{window.location.href}</code>
              </div>
              <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)' }}>
                Scan with any Stellar wallet app to open this payment page
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