import { useCallback, useEffect, useMemo, useState } from 'react'
import { useAccount, useConnect, useDisconnect, useSignTypedData } from 'wagmi'

type AuthState = {
  signature: string
  issuedAt: string
  nonce: string
  address: `0x${string}`
}

const storageKey = (address: string) =>
  `privacy-vault-auth:${address.toLowerCase()}`

const createNonce = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID()
  }

  return Math.random().toString(36).slice(2)
}

export const useWalletAuth = () => {
  const { address, isConnected } = useAccount()
  const { connectAsync, connectors, isPending: isConnecting } = useConnect()
  const { disconnect } = useDisconnect()
  const { signTypedDataAsync, isPending: isSigning } = useSignTypedData()
  const [authState, setAuthState] = useState<AuthState | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!address) {
      setAuthState(null)
      return
    }

    const stored = localStorage.getItem(storageKey(address))
    if (!stored) {
      setAuthState(null)
      return
    }

    try {
      const parsed = JSON.parse(stored) as AuthState
      if (parsed.address?.toLowerCase() === address.toLowerCase()) {
        setAuthState(parsed)
      } else {
        setAuthState(null)
      }
    } catch {
      setAuthState(null)
    }
  }, [address])

  const connectWallet = useCallback(async () => {
    setError(null)
    const connector = connectors.find((item) => item.ready) ?? connectors[0]

    if (!connector) {
      setError('No wallet connector available.')
      return
    }

    try {
      await connectAsync({ connector })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect wallet')
    }
  }, [connectAsync, connectors])

  const authenticate = useCallback(async () => {
    if (!address) {
      setError('Connect a wallet to continue.')
      return
    }

    setError(null)

    const issuedAt = new Date().toISOString()
    const nonce = createNonce()
    const message = {
      address,
      statement: 'Sign in to Payy Privacy Vault',
      nonce,
      issuedAt
    }

    try {
      const signature = await signTypedDataAsync({
        domain: {
          name: 'Payy Privacy Vault',
          version: '1'
        },
        types: {
          Auth: [
            { name: 'address', type: 'address' },
            { name: 'statement', type: 'string' },
            { name: 'nonce', type: 'string' },
            { name: 'issuedAt', type: 'string' }
          ]
        },
        primaryType: 'Auth',
        message
      })

      const payload: AuthState = {
        signature,
        issuedAt,
        nonce,
        address
      }

      localStorage.setItem(storageKey(address), JSON.stringify(payload))
      setAuthState(payload)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Signature rejected')
    }
  }, [address, signTypedDataAsync])

  const resetAuth = useCallback(() => {
    if (address) {
      localStorage.removeItem(storageKey(address))
    }
    setAuthState(null)
  }, [address])

  const status = useMemo(() => {
    if (authState) {
      return 'complete'
    }
    if (isSigning || isConnecting) {
      return 'pending'
    }
    if (error) {
      return 'error'
    }
    return 'idle'
  }, [authState, error, isConnecting, isSigning])

  return {
    address,
    isConnected,
    isConnecting,
    isSigning,
    isAuthenticated: Boolean(authState),
    status,
    error,
    connectWallet,
    authenticate,
    disconnect,
    resetAuth
  }
}

export type WalletAuthState = ReturnType<typeof useWalletAuth>
