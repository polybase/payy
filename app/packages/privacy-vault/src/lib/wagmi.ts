import { createConfig, http } from 'wagmi'
import { injected } from 'wagmi/connectors'

import { payyTestnet, RPC_URL } from './config'

export const wagmiConfig = createConfig({
  chains: [payyTestnet],
  connectors: [injected()],
  transports: {
    [payyTestnet.id]: http(RPC_URL)
  }
})
