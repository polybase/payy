import posthog, { PostHog } from 'posthog-js'

declare global {
  interface Window {
    posthogInitialized?: boolean
    posthog: PostHog
  }
}

export const initPosthog = () => {
  if (typeof window === 'undefined') return

  if (!window.posthogInitialized) {
    posthog.init('phc_2WvD3IJabsDhmHALah3v5VA20yvEptB357mxuARI3b5', {
      api_host: 'https://us.i.posthog.com',
      defaults: '2025-05-24'
    })

    window.posthogInitialized = true
    window.posthog = posthog
  }
}

export const getPosthogDistinctId = (): string | undefined => {
  if (typeof window === 'undefined') return
  return posthog.get_distinct_id()
}
