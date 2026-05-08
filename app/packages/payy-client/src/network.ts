import type { Address, PayyNetworkConfig } from './types'

export const defaultPrivacyBridge =
  '0x3100000000000000000000000000000000000000' satisfies Address

export const payyNetworks = {
  dev: {
    chainId: 7297,
    name: 'payy-dev',
    privacyBridge: defaultPrivacyBridge
  },
  testnet: {
    chainId: 7298,
    name: 'payy-testnet',
    privacyBridge: defaultPrivacyBridge
  }
} satisfies Record<string, PayyNetworkConfig>

export type PayyNetworkName = keyof typeof payyNetworks

export function resolveNetwork(
  network: PayyNetworkName | PayyNetworkConfig
): PayyNetworkConfig {
  if (typeof network === 'string') {
    return payyNetworks[network]
  }
  return network
}
