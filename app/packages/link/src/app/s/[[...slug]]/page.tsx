'use client'

import Image from 'next/image'
import styles from '../../page.module.css'
import note from '../../img/note-send.png'
import send from '../../img/payy-note-received.svg'
import { Button } from '../../../components/buttons/Button'
import { ClaimButton } from '@/components/buttons/claim-button'
import { useStoreRedirect } from '@/hooks/useStoreRedirect'

export interface SendPaymentParams {
  params: {
    slug: string[]
  }
}

export default function SendPayment({ params }: SendPaymentParams) {
  const referrerUrlPayyBaseUrl = 'https://payy.link'
  const hash = typeof window !== 'undefined' ? window.location.hash : ''
  const referrerUrl = `${referrerUrlPayyBaseUrl}/s/${params.slug?.join('/') || ''}${hash}`

  const handleRedirectToStore = useStoreRedirect({
    baseReferrerUrl: referrerUrl
  })

  return (
    <>
      <div className={styles.main}>
        <div>
          <div className={styles.note}>
            <Image src={note} alt="Payy note" width={310} height={326} />
          </div>
          <div className={styles.title}>
            <Image
              src={send}
              alt="Payy note received"
              width={316}
              height={100}
            />
          </div>
          <Button onClick={handleRedirectToStore}>Download App</Button>
          <ClaimButton slug={params.slug} />
        </div>
      </div>
    </>
  )
}
