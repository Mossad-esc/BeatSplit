/**
 * Stellar RPC + Contract Client Helpers
 * Wraps Stellar SDK for BeatSplit contract interactions
 */

import {
  Contract,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  Operation,
  nativeToScVal,
  xdr,
  Address,
  scValToNative,
  StrKey,
} from '@stellar/stellar-sdk';
import { Buffer } from 'buffer';

// Ensure Buffer is available globally (for stellar-sdk)
if (typeof window !== 'undefined' && !window.Buffer) {
  window.Buffer = Buffer;
}

// ── Types ────────────────────────────────────────────────────────────────

export interface Recipient {
  address: string;
  bps: number; // basis points (10000 = 100%)
}

export interface Split {
  id: number;
  creator: string;
  token: string;
  recipients: Recipient[];
  status: 'pending' | 'active' | 'locked';
  metadataHash: string;
  totalReceived: string; // i128 as string
  version: number;
}

export interface ClaimableBalance {
  address: string;
  amount: string;
}

export interface EarnedBalance {
  address: string;
  amount: string;
}

export interface AmendmentProposal {
  splitId: number;
  proposer: string;
  newRecipients: Recipient[];
  approvals: string[];
  baseVersion: number;
}

// ── Configuration ────────────────────────────────────────────────────────
// Use placeholder values during build (SSG) to avoid "Invalid contract ID" errors
// Real values are injected via NEXT_PUBLIC_* env vars at runtime
const isBuildTime = typeof window === 'undefined' && process.env.NODE_ENV === 'production';

export const CONFIG = {
  contractId: process.env.NEXT_PUBLIC_CONTRACT_ID || (isBuildTime ? 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM' : ''),
  rpcUrl: process.env.NEXT_PUBLIC_RPC_URL || (isBuildTime ? 'https://soroban-testnet.stellar.org' : ''),
  networkPassphrase: process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE || (isBuildTime ? 'Test SDF Network ; September 2015' : ''),
  usdcContractId: process.env.NEXT_PUBLIC_USDC_CONTRACT_ID || (isBuildTime ? 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC' : ''),
} as const;

// Validate config at runtime (client-side only)
if (typeof window !== 'undefined') {
  const missing = Object.entries(CONFIG)
    .filter(([, v]) => !v)
    .map(([k]) => k);
  if (missing.length > 0) {
    console.warn('[BeatSplit] Missing env vars:', missing.join(', '));
  }
}

// ── RPC Client ───────────────────────────────────────────────────────────

let rpcClient: SorobanRpc.Server | null = null;

export function getRpcClient(): SorobanRpc.Server {
  if (!rpcClient) {
    rpcClient = new SorobanRpc.Server(CONFIG.rpcUrl, { allowHttp: true });
  }
  return rpcClient;
}

// ── Contract Client ──────────────────────────────────────────────────────

const contract = new Contract(CONFIG.contractId);

function toScValRecipient(r: Recipient): xdr.ScVal {
  return nativeToScVal({
    addr: r.address,
    bps: r.bps,
  }, { type: 'map' });
}

function toScValRecipients(recipients: Recipient[]): xdr.ScVal {
  return nativeToScVal(recipients.map(toScValRecipient), { type: 'vec' });
}

function parseSplit(result: xdr.ScVal): Split {
  const native = scValToNative(result);
  // native is a Map with Split fields
  const map = native as Map<string, unknown>;
  return {
    id: Number(map.get('id')),
    creator: map.get('creator') as string,
    token: map.get('token') as string,
    recipients: (map.get('recipients') as Array<{ addr: string; bps: number }>).map(r => ({
      address: r.addr,
      bps: r.bps,
    })),
    status: (map.get('status') as string).toLowerCase() as Split['status'],
    metadataHash: map.get('metadata_hash') as string,
    totalReceived: map.get('total_received') as string,
    version: Number(map.get('version')),
  };
}

function parseRecipients(result: xdr.ScVal): Recipient[] {
  const native = scValToNative(result);
  return (native as Array<{ addr: string; bps: number }>).map(r => ({
    address: r.addr,
    bps: r.bps,
  }));
}

function parseClaimable(result: xdr.ScVal): string {
  return scValToNative(result) as string;
}

function parseEarned(result: xdr.ScVal): string {
  return scValToNative(result) as string;
}

// ── Public API ───────────────────────────────────────────────────────────

export async function simulateAndSend(
  sourceAccount: string,
  ops: xdr.Operation[],
  signCallback: (tx: string) => Promise<string>
): Promise<SorobanRpc.Api.SendTransactionResponse> {
  const rpc = getRpcClient();
  const account = await rpc.getAccount(sourceAccount);

  const tx = new TransactionBuilder(account, {
    fee: '1000000', // 0.1 XLM max fee
    networkPassphrase: CONFIG.networkPassphrase,
  });
  for (const op of ops) {
    tx.addOperation(op);
  }
  tx.setTimeout(300);
  const builtTx = tx.build();

  const simulateRes = await rpc.simulateTransaction(builtTx);
  if (SorobanRpc.Api.isSimulationError(simulateRes)) {
    throw new Error(`Simulation failed: ${simulateRes.error}`);
  }

  const prepared = SorobanRpc.assembleTransaction(builtTx, simulateRes).build();
  const signedTxXdr = await signCallback(prepared.toEnvelope().toXDR('base64'));

  const sendRes = await rpc.sendTransaction(signedTxXdr as any);
  return sendRes;
}

export async function createSplit(
  creator: string,
  token: string,
  recipients: Recipient[],
  metadataHash: string
): Promise<number> {
  const op = contract.call(
    'create_split',
    nativeToScVal(creator, { type: 'address' }),
    nativeToScVal(token, { type: 'address' }),
    toScValRecipients(recipients),
    nativeToScVal(Buffer.from(metadataHash, 'hex'), { type: 'bytes' })
  );

  // This is a simulate-only call; actual submission needs wallet signing
  const rpc = getRpcClient();
  const account = await rpc.getAccount(creator);
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

  // Return the simulated result (split ID)
  const result = simulateRes.result!.retval;
  return Number(scValToNative(result));
}

export async function getSplit(splitId: number): Promise<Split | null> {
  const rpc = getRpcClient();
  const op = contract.call('get_split', nativeToScVal(splitId, { type: 'u64' }));

  const tx = new TransactionBuilder(
    { accountId: CONFIG.contractId, sequence: '0' } as any,
    { fee: '100000', networkPassphrase: CONFIG.networkPassphrase }
  )
    .addOperation(op)
    .setTimeout(30)
    .build();

  const simulateRes = await rpc.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simulateRes)) {
    return null;
  }

  const result = simulateRes.result!.retval;
  const native = scValToNative(result);
  if (!native) return null;
  return parseSplit(result);
}

export async function getClaimable(splitId: number, address: string): Promise<string> {
  const rpc = getRpcClient();
  const op = contract.call(
    'get_claimable',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(address, { type: 'address' })
  );

  const tx = new TransactionBuilder(
    { accountId: CONFIG.contractId, sequence: '0' } as any,
    { fee: '100000', networkPassphrase: CONFIG.networkPassphrase }
  )
    .addOperation(op)
    .setTimeout(30)
    .build();

  const simulateRes = await rpc.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simulateRes)) {
    return '0';
  }

  return parseClaimable(simulateRes.result!.retval);
}

export async function getEarned(splitId: number, address: string): Promise<string> {
  const rpc = getRpcClient();
  const op = contract.call(
    'get_earned',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(address, { type: 'address' })
  );

  const tx = new TransactionBuilder(
    { accountId: CONFIG.contractId, sequence: '0' } as any,
    { fee: '100000', networkPassphrase: CONFIG.networkPassphrase }
  )
    .addOperation(op)
    .setTimeout(30)
    .build();

  const simulateRes = await rpc.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simulateRes)) {
    return '0';
  }

  return parseEarned(simulateRes.result!.retval);
}

export function buildAcceptOp(splitId: number, recipient: string): xdr.Operation {
  return contract.call(
    'accept',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(recipient, { type: 'address' })
  );
}

export function buildDepositOp(splitId: number, from: string, amount: string): xdr.Operation {
  return contract.call(
    'deposit',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(from, { type: 'address' }),
    nativeToScVal(BigInt(amount), { type: 'i128' })
  );
}

export function buildClaimOp(splitId: number, recipient: string): xdr.Operation {
  return contract.call(
    'claim',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(recipient, { type: 'address' })
  );
}

export function buildProposeAmendmentOp(
  splitId: number,
  proposer: string,
  newRecipients: Recipient[]
): xdr.Operation {
  return contract.call(
    'propose_amendment',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(proposer, { type: 'address' }),
    toScValRecipients(newRecipients)
  );
}

export function buildApproveAmendmentOp(splitId: number, approver: string): xdr.Operation {
  return contract.call(
    'approve_amendment',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(approver, { type: 'address' })
  );
}

export function buildCancelAmendmentOp(splitId: number, canceller: string): xdr.Operation {
  return contract.call(
    'cancel_amendment',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(canceller, { type: 'address' })
  );
}

export function buildApproveLockOp(splitId: number, approver: string): xdr.Operation {
  return contract.call(
    'approve_lock',
    nativeToScVal(splitId, { type: 'u64' }),
    nativeToScVal(approver, { type: 'address' })
  );
}

export function buildExtendTtlOp(splitId: number): xdr.Operation {
  return contract.call(
    'extend_ttl',
    nativeToScVal(splitId, { type: 'u64' })
  );
}

// ── Utility Functions ────────────────────────────────────────────────────

export function formatAmount(amount: string, decimals = 7): string {
  const num = BigInt(amount);
  const divisor = BigInt(10 ** decimals);
  const whole = num / divisor;
  const fraction = num % divisor;
  if (fraction === 0n) return whole.toString();
  const fracStr = fraction.toString().padStart(decimals, '0').replace(/0+$/, '');
  return `${whole}.${fracStr}`;
}

export function parseAmount(amount: string, decimals = 7): string {
  const [whole, fraction = ''] = amount.split('.');
  const fracPadded = fraction.padEnd(decimals, '0').slice(0, decimals);
  return (BigInt(whole) * BigInt(10 ** decimals) + BigInt(fracPadded)).toString();
}

export function bpsToPercent(bps: number): string {
  return (bps / 100).toFixed(2) + '%';
}

export function percentToBps(percent: string): number {
  const num = parseFloat(percent);
  return Math.round(num * 100);
}

export function validateRecipients(recipients: Recipient[]): { valid: boolean; error?: string } {
  if (recipients.length < 2) return { valid: false, error: 'At least 2 recipients required' };
  if (recipients.length > 20) return { valid: false, error: 'Maximum 20 recipients allowed' };

  const addresses = new Set<string>();
  let totalBps = 0;

  for (const r of recipients) {
    if (!StrKey.isValidEd25519PublicKey(r.address) && !StrKey.isValidContract(r.address)) {
      return { valid: false, error: `Invalid Stellar address: ${r.address}` };
    }
    if (addresses.has(r.address)) {
      return { valid: false, error: `Duplicate address: ${r.address}` };
    }
    if (r.bps <= 0) {
      return { valid: false, error: `Share must be > 0 for ${r.address}` };
    }
    addresses.add(r.address);
    totalBps += r.bps;
  }

  if (totalBps !== 10000) {
    return { valid: false, error: `Shares must sum to 100% (current: ${totalBps / 100}%)` };
  }

  return { valid: true };
}

export function computeShares(amount: string, recipients: Recipient[]): Map<string, string> {
  const amt = BigInt(amount);
  const shares = new Map<string, string>();
  let sumOthers = 0n;

  // All except first get floor(amount * bps / 10000)
  for (let i = 1; i < recipients.length; i++) {
    const r = recipients[i];
    const share = (amt * BigInt(r.bps)) / 10000n;
    shares.set(r.address, share.toString());
    sumOthers += share;
  }

  // First gets remainder (absorbs dust)
  const firstShare = amt - sumOthers;
  shares.set(recipients[0].address, firstShare.toString());

  return shares;
}

export function computeSharePreview(amount: string, recipients: Recipient[]): Array<{ address: string; share: string; percent: string }> {
  const shares = computeShares(amount, recipients);
  return recipients.map(r => ({
    address: r.address,
    share: formatAmount(shares.get(r.address) || '0'),
    percent: bpsToPercent(r.bps),
  }));
}

// ── Error Handling ──────────────────────────────────────────────────────

export const CONTRACT_ERRORS: Record<number, string> = {
  1: 'Invalid recipient count (must be 2-20)',
  2: 'Shares must sum to exactly 100% (10,000 basis points)',
  3: 'A recipient has 0% share',
  4: 'Duplicate recipient address',
  5: 'Amount must be positive',
  6: 'Split not found',
  7: 'Split is not active (must be Active or Locked)',
  8: 'Split is already active or locked',
  9: 'Caller is not a recipient of this split',
  10: 'Recipient has already accepted',
  11: 'Arithmetic overflow',
  12: 'An amendment proposal is already open',
  13: 'No open amendment proposal',
  14: 'Already approved this amendment',
  15: 'Split is locked and cannot be changed',
  16: 'Nothing to claim',
};

export function getErrorMessage(errorCode: number): string {
  return CONTRACT_ERRORS[errorCode] || `Unknown error (code ${errorCode})`;
}

// ── Freighter Wallet Integration ────────────────────────────────────────

declare global {
  interface Window {
    freighter?: {
      isConnected: () => Promise<boolean>;
      connect: () => Promise<{ address: string; network: string; publicKey: string }>;
      disconnect: () => Promise<void>;
      getAddress: () => Promise<string>;
      getNetwork: () => Promise<string>;
      getPublicKey: () => Promise<string>;
      signTransaction: (xdr: string, networkPassphrase: string) => Promise<string>;
      signMessage: (message: string) => Promise<string>;
    };
  }
}

export async function isFreighterInstalled(): Promise<boolean> {
  return typeof window !== 'undefined' && !!window.freighter;
}

export async function isFreighterConnected(): Promise<boolean> {
  if (!await isFreighterInstalled()) return false;
  try {
    return await window.freighter!.isConnected();
  } catch {
    return false;
  }
}

export async function connectFreighter(): Promise<{ address: string; network: string } | null> {
  if (!await isFreighterInstalled()) {
    throw new Error('Freighter wallet not installed. Please install from https://freighter.app');
  }
  try {
    const result = await window.freighter!.connect();
    return { address: result.address, network: result.network };
  } catch (e) {
    console.error('Freighter connect failed:', e);
    return null;
  }
}

export async function disconnectFreighter(): Promise<void> {
  if (await isFreighterInstalled()) {
    await window.freighter!.disconnect();
  }
}

export async function getFreighterAddress(): Promise<string | null> {
  if (!await isFreighterConnected()) return null;
  return window.freighter!.getAddress();
}

export async function getFreighterNetwork(): Promise<string | null> {
  if (!await isFreighterConnected()) return null;
  return window.freighter!.getNetwork();
}

export async function signWithFreighter(txXdr: string): Promise<string> {
  if (!await isFreighterConnected()) {
    throw new Error('Freighter not connected');
  }
  return window.freighter!.signTransaction(txXdr, CONFIG.networkPassphrase);
}

export function checkNetworkMatch(walletNetwork: string): boolean {
  const expected = process.env.NEXT_PUBLIC_NETWORK === 'mainnet' ? 'mainnet' : 'testnet';
  return walletNetwork === expected;
}