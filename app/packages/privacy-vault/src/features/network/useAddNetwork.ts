import { useCallback, useMemo, useState } from 'react'
import { useAccount, useChainId } from 'wagmi'
import { toHex } from 'viem'

import { payyTestnet } from '../../lib/config'

type NetworkStatus = 'idle' | 'pending' | 'complete' | 'error'

type EthereumProvider = {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>
}

const getEthereum = (): EthereumProvider | undefined =>
  (window as Window & { ethereum?: EthereumProvider }).ethereum

export const useAddNetwork = () => {
  const { isConnected } = useAccount()
  const chainId = useChainId()
  const [isPending, setIsPending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const isAdded = useMemo(
    () => isConnected && chainId === payyTestnet.id,
    [isConnected, chainId]
  )

  const status = useMemo<NetworkStatus>(() => {
    if (isAdded) {
      return 'complete'
    }
    if (isPending) {
      return 'pending'
    }
    if (error) {
      return 'error'
    }
    return 'idle'
  }, [isAdded, isPending, error])

  const addNetwork = useCallback(async () => {
    const ethereum = getEthereum()
    if (!ethereum) {
      setError('No wallet detected. Please install a wallet extension.')
      return
    }

    setError(null)
    setIsPending(true)

    try {
      await ethereum.request({
        method: 'wallet_addEthereumChain',
        params: [
          {
            chainId: toHex(payyTestnet.id),
            chainName: payyTestnet.name,
            nativeCurrency: payyTestnet.nativeCurrency,
            rpcUrls: payyTestnet.rpcUrls.default.http,
            blockExplorerUrls: [payyTestnet.blockExplorers.default.url]
          }
        ]
      })

      setError(null)
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to add Payy Testnet'
      )
    } finally {
      setIsPending(false)
    }
  }, [])

  return {
    status,
    error,
    isAdded,
    addNetwork
  }
}

export type AddNetworkState = ReturnType<typeof useAddNetwork>
