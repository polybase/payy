'use client'

import { initPosthog } from '@/config/posthog'
import { useEffect } from 'react'

export default function PosthogInit() {
  useEffect(() => {
    initPosthog()
  }, [])

  return null
}
