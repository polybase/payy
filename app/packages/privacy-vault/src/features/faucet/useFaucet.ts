import { useMutation } from '@tanstack/react-query'

import { claimFaucet, type FaucetResponse } from '../../lib/api'

export const useFaucet = () => {
  return useMutation<FaucetResponse, Error, string>({
    mutationFn: async (address: string) => claimFaucet({ address })
  })
}
