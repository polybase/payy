import type { Metadata } from 'next'
import RequestPayment from './RequestPayment'

export const metadata: Metadata = {
  title: 'Payment Request - Payy',
  description:
    'Request for payment in USD. Payments will be sent privately using Payy.'
}

export default function Page({ params }: { params: { slug: string[] } }) {
  return <RequestPayment params={params} />
}
