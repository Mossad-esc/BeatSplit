'use client';

import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import {
  isFreighterInstalled,
  isFreighterConnected,
  connectFreighter,
  disconnectFreighter,
  getFreighterAddress,
  getFreighterNetwork,
  signWithFreighter,
  checkNetworkMatch,
  CONFIG,
} from '@/lib/stellar';

interface WalletState {
  connected: boolean;
  address: string | null;
  network: string | null;
  networkMismatch: boolean;
  connecting: boolean;
  error: string | null;
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  signTransaction: (xdr: string) => Promise<string>;
}

const WalletContext = createContext<WalletState | null>(null);

export function WalletProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<WalletState>({
    connected: false,
    address: null,
    network: null,
    networkMismatch: false,
    connecting: false,
    error: null,
    connect: async () => {},
    disconnect: async () => {},
    signTransaction: async () => '',
  });

  const refreshState = async () => {
    if (!await isFreighterInstalled()) {
      setState(s => ({ ...s, connected: false, error: 'Freighter not installed' }));
      return;
    }
    const connected = await isFreighterConnected();
    if (connected) {
      const [address, network] = await Promise.all([
        getFreighterAddress(),
        getFreighterNetwork(),
      ]);
      setState(s => ({
        ...s,
        connected: true,
        address,
        network,
        networkMismatch: !checkNetworkMatch(network || ''),
        error: null,
      }));
    } else {
      setState(s => ({ ...s, connected: false, address: null, network: null }));
    }
  };

  const connect = async () => {
    setState(s => ({ ...s, connecting: true, error: null }));
    try {
      const result = await connectFreighter();
      if (result) {
        await refreshState();
      } else {
        setState(s => ({ ...s, connecting: false, error: 'Connection cancelled' }));
      }
    } catch (e) {
      setState(s => ({ ...s, connecting: false, error: String(e) }));
    }
  };

  const disconnect = async () => {
    await disconnectFreighter();
    setState(s => ({ ...s, connected: false, address: null, network: null }));
  };

  const signTransaction = async (xdr: string) => {
    return signWithFreighter(xdr);
  };

  useEffect(() => {
    refreshState();
    // Listen for wallet changes
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'freighter_wallet') refreshState();
    };
    window.addEventListener('storage', handleStorageChange);
    return () => window.removeEventListener('storage', handleStorageChange);
  }, []);

  const value = { ...state, connect, disconnect, signTransaction };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const ctx = useContext(WalletContext);
  if (!ctx) throw new Error('useWallet must be used within WalletProvider');
  return ctx;
}