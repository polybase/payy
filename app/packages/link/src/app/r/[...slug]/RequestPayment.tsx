'use client'

import Image from 'next/image'
import styles from '../../page.module.css'
import note from '../../img/note-request.png'
import paymentRequestText from '../../img/payment-request.svg'
import { Button } from '../../../components/buttons/Button'
import { PAYY_BASE_URL } from '../../../config/constants'
import shortcodeToEmoji from 'emoji-shortcode-mapping/shortcode-to-emoji.json'
import { useStoreRedirect } from '@/hooks/useStoreRedirect'

export interface RequestPaymentParams {
  params: {
    slug: string[]
  }
}

// Add emoji handling
const emojiShortcodeRegex = /~[a-z_ ]+~/g

function decodeMemo(memo: string) {
  // First handle URL encoding
  memo = memo.replaceAll('_', '%20')
  memo = memo.replaceAll('%255F', '%5F')
  memo = decodeURIComponent(memo)

  // Handle emoji variant selector
  memo = memo.replaceAll('~ee~', '~e~')
  memo = memo.replaceAll(/~e~/g, '\uFE0F')

  // Convert emoji shortcodes to actual emojis
  let match
  while ((match = emojiShortcodeRegex.exec(memo)) !== null) {
    const emoji = shortcodeToEmoji[match[0] as keyof typeof shortcodeToEmoji]
    if (!emoji) continue
    memo = memo.replace(match[0], emoji)
  }

  return memo
}

export default function RequestPayment({ params }: RequestPaymentParams) {
  const referrerUrlPayyBaseUrl = 'https://payy.link'
  const hash = window?.location.hash
  const referrerUrl = `${referrerUrlPayyBaseUrl}/r/${params.slug?.join('/') || ''}${hash}`

  const handleRedirectToStore = useStoreRedirect({
    baseReferrerUrl: referrerUrl
  })

  return (
    <>
      <div className={styles.main}>
        <div className={styles.note_container}>
          <div className={styles.note}>
            <div className={styles.value}>
              {fromBigIntToCurrency(BigInt(params.slug[1]))}
            </div>
            <Image src={note} alt="Payy note" width={310} height={326} />
          </div>
          <div className={styles.title}>
            <Image
              src={paymentRequestText}
              alt="Payment Request"
              width={316}
              height={100}
            />
          </div>
          <div className={styles.memo_container}>
            {params.slug[2] && (
              <h1
                className={styles.memo}
                style={{ wordBreak: 'break-all', overflowWrap: 'anywhere' }}
              >
                {decodeMemo(params.slug[2])}
              </h1>
            )}
          </div>
          <Button onClick={handleRedirectToStore}>Download App</Button>
          <Button
            outline
            url={`${PAYY_BASE_URL}/r/${params.slug.join('/')}${hash}`}
          >
            Send in App
          </Button>
        </div>
      </div>
    </>
  )
}

function fromBigIntToCurrency(balance: bigint): string {
  const balanceString = balance.toString().padStart(6, '0')
  const whole = balanceString.slice(0, -6)
  const decimal = balanceString.slice(-6).slice(0, 2).padStart(2, '0')
  return `${whole === '0' || whole === '' ? '0' : whole}.${decimal}`
}
