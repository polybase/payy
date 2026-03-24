import * as React from 'react'
import type { Metadata, Viewport } from 'next'
import { Providers } from './providers'
import { steradian } from './steradian'

export const metadata: Metadata = {
  metadataBase: new URL(
    process.env.NODE_ENV === 'production'
      ? 'https://payy.network'
      : 'http://localhost:3000'
  ),
  title: 'Payy Network',
  description: 'Payy Network powers Payy, a new way to pay for things.'
}

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 1,
  userScalable: false
}

export default function RootLayout({
  children
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body className={steradian.className}>
        <Providers>{children}</Providers>
      </body>
    </html>
  )
}
