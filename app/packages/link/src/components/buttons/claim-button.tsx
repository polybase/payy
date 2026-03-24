'use client'

import { PAYY_BASE_URL } from '../../config/constants'
import { Button } from './Button'

export function ClaimButton(params: { slug?: string[] }) {
  return (
    <Button
      outline
      url={`${PAYY_BASE_URL}/s/${params.slug?.join('/') || ''}${window?.location.hash ?? ''}`}
    >
      Claim in App
    </Button>
  )
}
