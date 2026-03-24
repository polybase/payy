import { useEffect, useMemo, useState } from 'react'

import { Layout } from './components/Layout'
import { StepCard } from './components/StepCard'
import type { StepStatus } from './components/StatusIndicator'
import { FaucetStep } from './features/faucet/FaucetStep'
import { useFaucet } from './features/faucet/useFaucet'
import { AuthStep } from './features/auth/AuthStep'
import { useWalletAuth } from './features/auth/useWalletAuth'
import { NetworkStep } from './features/network/NetworkStep'
import { useAddNetwork } from './features/network/useAddNetwork'
import { useTheme } from './hooks/useTheme'
import { IS_TESTNET } from './lib/config'

const computeStatus = (
  isPending: boolean,
  isComplete: boolean,
  hasError: boolean
): StepStatus => {
  if (isComplete) {
    return 'complete'
  }
  if (isPending) {
    return 'pending'
  }
  if (hasError) {
    return 'error'
  }
  return 'idle'
}

const App = () => {
  useTheme()

  const auth = useWalletAuth()
  const network = useAddNetwork()
  const faucet = useFaucet()

  const [openStep, setOpenStep] = useState(1)

  const authStatus = auth.status
  const networkStatus = network.status

  const faucetStatus = useMemo(
    () =>
      computeStatus(faucet.isPending, faucet.isSuccess, Boolean(faucet.error)),
    [faucet.error, faucet.isPending, faucet.isSuccess]
  )

  useEffect(() => {
    if (authStatus === 'complete' && networkStatus !== 'complete') {
      setOpenStep(2)
    }
  }, [authStatus, networkStatus])

  useEffect(() => {
    if (
      authStatus === 'complete'
      && networkStatus === 'complete'
      && faucetStatus !== 'complete'
    ) {
      setOpenStep(3)
    }
  }, [authStatus, networkStatus, faucetStatus])

  return (
    <Layout>
      <StepCard
        step={1}
        title="Authenticate wallet"
        description="Connect your wallet and sign an EIP-712 message."
        status={authStatus}
        isOpen={openStep === 1}
        onToggle={() => setOpenStep(1)}
      >
        <AuthStep auth={auth} />
      </StepCard>

      <StepCard
        step={2}
        title="Add Payy Testnet"
        description="One click to add the Payy Testnet RPC to your wallet."
        status={networkStatus}
        isOpen={openStep === 2}
        onToggle={() => setOpenStep(2)}
      >
        <NetworkStep network={network} />
      </StepCard>

      {IS_TESTNET && (
        <StepCard
          step={3}
          title="Mint test tokens"
          description="Use the faucet to fund your Payy Testnet wallet."
          status={faucetStatus}
          isOpen={openStep === 3}
          onToggle={() => setOpenStep(3)}
        >
          <FaucetStep
            faucet={faucet}
            address={auth.address}
            isConnected={auth.isConnected}
          />
        </StepCard>
      )}
    </Layout>
  )
}

export default App
