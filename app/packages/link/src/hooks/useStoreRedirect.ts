'use client'

import { GUILD_URL } from '@/config/constants'
import { useFingerprint } from '@/hooks/useFingerprint'
import { usePosthogDistinctId } from '@/hooks/usePosthogDistinctId'
import { detectBrowser } from '@/util/detectBrowser'
import { useCallback } from 'react'

export interface StoreRedirectProps {
  baseReferrerUrl: string
}

export function useStoreRedirect({ baseReferrerUrl }: StoreRedirectProps) {
  const fingerprint = useFingerprint()
  const posthogDistinctId = usePosthogDistinctId()
  const isIos = detectBrowser() === 'ios'

  const fullReferrerUrlIos = `${baseReferrerUrl}${baseReferrerUrl.includes('?') ? '&' : '?'}posthog_id=${posthogDistinctId}`
  const encodedReferrerUrl = encodeURIComponent(
    isIos ? fullReferrerUrlIos : baseReferrerUrl
  )
  const storeUrl = `https://payy.link/download?referrer=${encodedReferrerUrl}`

  const redirectToStore = useCallback(async () => {
    if (typeof window === 'undefined') return

    if (!isIos || !fingerprint || !posthogDistinctId) {
      window.location.href = storeUrl
      return
    }

    // only for iOS
    try {
      const udhUrl = `${GUILD_URL}/udh`

      const payload = {
        properties: fingerprint,
        posthog_id: posthogDistinctId,
        referrer_url: fullReferrerUrlIos
      }

      const res = await fetch(udhUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(payload)
      })

      const resData = await res.json()
      const userDeviceHash = resData?.user_device_hash

      if (userDeviceHash) {
        // identify with PostHog
        window.posthog?.identify?.(posthogDistinctId)
        window.posthog?.group?.('device', userDeviceHash)
        // add the user device hash as an attribute
        window.posthog?.setPersonProperties?.({
          user_device_hash: userDeviceHash
        })
      }
    } catch (err: any) {
      console.error(`Failed to contact the /udh endpoint: ${err}`)
    } finally {
      // redirect to the app store
      window.location.href = storeUrl
    }
  }, [fingerprint, fullReferrerUrlIos, isIos, posthogDistinctId, storeUrl])

  return redirectToStore
}
