import { Button } from '../../components/Button'
import type { WalletAuthState } from './useWalletAuth'

const shortenAddress = (address: string) =>
  `${address.slice(0, 6)}...${address.slice(address.length - 4)}`

interface AuthStepProps {
  auth: WalletAuthState
}

export const AuthStep = ({ auth }: AuthStepProps) => {
  const {
    address,
    isConnected,
    isConnecting,
    isSigning,
    isAuthenticated,
    error,
    connectWallet,
    authenticate,
    disconnect,
    resetAuth
  } = auth

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h3 className="text-base font-semibold text-text">
          Wallet authentication
        </h3>
        <p className="text-sm text-text/60">
          We use an EIP-712 typed signature to confirm ownership of your wallet
          before enabling the faucet.
        </p>
      </div>

      <div className="rounded-xl border border-gray-800 bg-[var(--surface-strong)] p-4 text-sm">
        {isConnected ? (
          <div className="flex flex-col gap-2">
            <div className="text-text/70">Connected wallet</div>
            <div className="text-base font-semibold text-text">
              {address ? shortenAddress(address) : 'Unknown address'}
            </div>
          </div>
        ) : (
          <div className="text-text/70">No wallet connected yet.</div>
        )}
      </div>

      {error && <div className="text-sm text-red-400">{error}</div>}

      <div className="flex flex-wrap gap-3">
        {!isConnected && (
          <Button onClick={connectWallet} disabled={isConnecting}>
            {isConnecting ? 'Connecting...' : 'Connect Wallet'}
          </Button>
        )}

        {isConnected && !isAuthenticated && (
          <Button onClick={authenticate} disabled={isSigning}>
            {isSigning ? 'Awaiting signature...' : 'Sign Authentication'}
          </Button>
        )}

        {isAuthenticated && (
          <>
            <Button variant="secondary" onClick={resetAuth}>
              Re-authenticate
            </Button>
            <Button variant="ghost" onClick={() => disconnect()}>
              Disconnect
            </Button>
          </>
        )}
      </div>
    </div>
  )
}
