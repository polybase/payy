import { ReactNode } from 'react'
import type { Metadata } from 'next'
import './globals.css'
import { steradian } from './steradian'
import PosthogInit from '@/components/PosthogInit'

export const metadata: Metadata = {
  metadataBase: new URL(
    process.env.NODE_ENV === 'production'
      ? 'https://payy.link'
      : 'http://localhost:3000'
  ),
  title: 'USD Payment - Payy',
  description: "You've been sent a payment in USD. Redeem it now for free."
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <head>
        <meta
          name="apple-itunes-app"
          content="app-id=6474760518, app-clip-bundle-id=com.polybaselabs.wallet.Clip, app-clip-display=card"
        />
        <meta name="viewport" content="initial-scale=1.0, width=device-width" />
      </head>
      <body className={steradian.className}>
        <PosthogInit />
        {children}
      </body>
    </html>
  )
}
