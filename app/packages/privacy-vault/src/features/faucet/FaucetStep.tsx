import { useMemo } from 'react'
import type { UseMutationResult } from '@tanstack/react-query'

import { Button } from '../../components/Button'
import { FaucetApiError, type FaucetResponse } from '../../lib/api'
import { buildExplorerTxUrl } from '../../lib/config'

interface FaucetStepProps {
  faucet: UseMutationResult<FaucetResponse, Error, string>
  address?: `0x${string}`
  isConnected: boolean
}

export const FaucetStep = ({
  faucet,
  address,
  isConnected
}: FaucetStepProps) => {
  const { mutateAsync, data, error, isPending, isSuccess, reset } = faucet

  const apiError = error instanceof FaucetApiError ? error : null
  const retryAfter =
    apiError?.reason === 'rate-limited' ? apiError.data?.retry_after_secs : null

  const statusMessage = useMemo(() => {
    if (isPending) {
      return 'Requesting faucet transfer...'
    }
    if (isSuccess) {
      return 'Faucet request completed. Transactions are broadcasting.'
    }
    return null
  }, [isPending, isSuccess])

  const onMint = async () => {
    if (!address) {
      return
    }
    reset()
    await mutateAsync(address)
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h3 className="text-base font-semibold text-text">
          Mint testnet funds
        </h3>
        <p className="text-sm text-text/60">
          Request PUSD on Payy Testnet to explore the Privacy Vault without real
          funds.
        </p>
      </div>

      <div className="rounded-xl border border-gray-800 bg-[var(--surface-strong)] p-4 text-sm">
        {isConnected ? (
          <div className="flex flex-col gap-2">
            <div className="text-text/70">Destination</div>
            <div className="text-base font-semibold text-text">{address}</div>
          </div>
        ) : (
          <div className="text-text/70">
            Connect your wallet first to request faucet funds.
          </div>
        )}
      </div>

      {statusMessage && (
        <div className="text-sm text-text/70">{statusMessage}</div>
      )}

      {error && (
        <div className="rounded-lg border border-red-500/40 bg-red-500/10 p-3 text-sm text-red-200">
          {retryAfter
            ? `Rate limit reached. Try again in ${retryAfter} seconds.`
            : apiError?.message || 'Faucet request failed. Please try again.'}
        </div>
      )}

      {data && (
        <div className="rounded-xl border border-gray-800 bg-[var(--surface-strong)] p-4 text-sm">
          <div className="text-text/70">Transaction hashes</div>
          <ul className="mt-2 flex flex-col gap-2 text-sm text-primary">
            {data.tx_hashes.map((hash) => (
              <li key={hash}>
                <a
                  href={buildExplorerTxUrl(hash)}
                  target="_blank"
                  rel="noreferrer"
                >
                  {hash}
                </a>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div>
        <Button onClick={onMint} disabled={!isConnected || isPending}>
          {isPending ? 'Requesting...' : 'Mint Test Tokens'}
        </Button>
      </div>
    </div>
  )
}
