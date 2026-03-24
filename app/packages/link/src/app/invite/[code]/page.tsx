import type { Metadata } from 'next'
import InvitePageClient from './InvitePageClient'

export const metadata: Metadata = {
  title: "You've been invited to Payy",
  description: 'Payy is crypto venmo. Fast, private, global payments.'
}

export interface InvitePageParams {
  params: {
    code: string
  }
}

export default function InvitePage({ params }: InvitePageParams) {
  return <InvitePageClient code={params.code} />
}
