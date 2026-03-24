import { Button } from '../../components/Button'
import { EXPLORER_URL, payyTestnet, RPC_URL } from '../../lib/config'
import type { AddNetworkState } from './useAddNetwork'

interface NetworkStepProps {
  network: AddNetworkState
}

export const NetworkStep = ({ network }: NetworkStepProps) => {
  const { status, error, isAdded, addNetwork } = network

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h3 className="text-base font-semibold text-text">Add Payy Testnet</h3>
        <p className="text-sm text-text/60">
          Add the Payy Testnet network to your wallet so it can interact with
          testnet assets.
        </p>
      </div>

      <div className="rounded-xl border border-gray-800 bg-[var(--surface-strong)] p-4 text-sm">
        <div className="flex flex-col gap-2">
          <div className="text-text/70">Network</div>
          <div className="text-base font-semibold text-text">
            {payyTestnet.name}
          </div>
          <div className="text-text/60">Chain ID: {payyTestnet.id}</div>
          <div className="text-text/60">RPC: {RPC_URL}</div>
          <div className="text-text/60">Block Explorer: {EXPLORER_URL}</div>
          <div className="text-text/60">
            Currency: {payyTestnet.nativeCurrency.symbol}
          </div>
        </div>
      </div>

      {error && <div className="text-sm text-red-400">{error}</div>}

      <div>
        <Button onClick={addNetwork} disabled={status === 'pending'}>
          {status === 'pending'
            ? 'Adding Network...'
            : isAdded
              ? 'Re-add Network'
              : 'Add Network'}
        </Button>
      </div>
    </div>
  )
}
