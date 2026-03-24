import { WalletState } from '@/types'
import { applyChangeset } from 'json-diff-ts'
import { cloneDeep } from 'lodash'

/**
 * Reconstruct the wallet state from the currently available latest backup and a wallet backup as a diff.
 *
 * @param latestBackup
 * @param diffs
 */
export const reconstructWalletsFromDiffs = (
  latestBackup: any,
  diffs: any[]
): WalletState => {
  if (!diffs.length) return latestBackup

  // clone to avoid mutation to the original state
  const currBackup = cloneDeep(latestBackup)

  try {
    return applyChangeset(currBackup, diffs)
  } catch (err) {
    console.error('Error while reconstructing state from backup: ', err)
    throw err
  }
}
