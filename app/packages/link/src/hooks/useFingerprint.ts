'use client'

import { useEffect, useState } from 'react'
import { UAParser } from 'ua-parser-js'

// Properties common to, and with values
// the same for - Linky(Safari), Linky(Chrome), App (React Native)
export type MatchingUserAgent = {
  device: {
    type?: string
    model?: string
    vendor?: string
  }
  engine: {
    name?: string
    version?: string
  }
  os: {
    name?: string
    version?: string
  }
}

export type Fingerprint = {
  user_agent: MatchingUserAgent
  language: string
  // timezone: string
  screen: {
    height: number
    width: number
  }
}

export const useFingerprint = (): Fingerprint | null => {
  const [fingerprint, setFingerprint] = useState<Fingerprint | null>(null)

  useEffect(() => {
    if (typeof window === 'undefined') return

    const uaParser = new UAParser()
    uaParser.setUA(navigator.userAgent)
    const res = uaParser.getResult()

    const matchingUserAgent = {
      device: res.device,
      engine: res.engine,
      os: res.os
    }

    const data: Fingerprint = {
      user_agent: matchingUserAgent,
      language: navigator.language,
      // timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      screen: {
        height: window.screen.height,
        width: window.screen.width
      }
    }

    setFingerprint(data)
  }, [])

  return fingerprint
}
