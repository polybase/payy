import { FAUCET_URL } from './config'

export interface FaucetRequest {
  address: string
  token?: string
}

export interface FaucetResponse {
  address: string
  tx_hashes: string[]
}

export interface FaucetErrorData {
  address?: string
  retry_after_secs?: number
  token?: string
}

export class FaucetApiError extends Error {
  reason?: string
  data?: FaucetErrorData
  status?: number

  constructor(
    message: string,
    options?: { reason?: string; data?: FaucetErrorData; status?: number }
  ) {
    super(message)
    this.name = 'FaucetApiError'
    this.reason = options?.reason
    this.data = options?.data
    this.status = options?.status
  }
}

interface ErrorResponse {
  error?: {
    reason?: string
    message?: string
    data?: FaucetErrorData
  }
}

const parseErrorResponse = async (
  response: Response
): Promise<ErrorResponse | null> => {
  try {
    return await response.json()
  } catch {
    return null
  }
}

export const claimFaucet = async (
  payload: FaucetRequest
): Promise<FaucetResponse> => {
  const response = await fetch(`${FAUCET_URL}/faucet/claim`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(payload)
  })

  if (!response.ok) {
    const errorPayload = await parseErrorResponse(response)
    const reason = errorPayload?.error?.reason
    const message = errorPayload?.error?.message || 'Faucet request failed'
    throw new FaucetApiError(message, {
      reason,
      data: errorPayload?.error?.data,
      status: response.status
    })
  }

  return response.json()
}
