import { getPosthogDistinctId } from '@/config/posthog'
import { useEffect, useState } from 'react'

export const usePosthogDistinctId = () => {
  const [distinctId, setDistinctId] = useState<string | undefined>(undefined)

  useEffect(() => {
    let attempts = 0
    const maxAttempts = 10

    const interval = setInterval(() => {
      const id = getPosthogDistinctId()

      if (id) {
        setDistinctId(id)
        clearInterval(interval)
      } else if (++attempts >= maxAttempts) {
        console.warn('Posthog Distinct Id not available after retries')
        clearInterval(interval)
      }
    }, 100)

    return () => clearInterval(interval)
  }, [])

  return distinctId
}
