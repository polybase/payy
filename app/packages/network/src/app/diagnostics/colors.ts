import { WalletActivityResultStatus } from '../../types'

export type Status = Exclude<WalletActivityResultStatus, null> | 'active'

export const resultColours: Record<Status, string> = {
  success: 'green',
  error: 'red',
  cancelled: 'gray',
  active: 'blue'
}
