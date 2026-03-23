'use client'

import Image from 'next/image'
import styles from '../../page.module.css'
import logo from '../../img/logo.png'
import payyInviteText from '../../img/payy-invite.svg'
import { Button } from '../../../components/buttons/Button'
import {
  PAYY_BASE_URL,
  PAYY_REFERRER_BASE_URL
} from '../../../config/constants'
import { useStoreRedirect } from '@/hooks/useStoreRedirect'

export default function InvitePageClient({ code }: { code: string }) {
  const baseReferrerUrl = `${PAYY_REFERRER_BASE_URL}/invite/${code}`
  const handleRedirectToStore = useStoreRedirect({ baseReferrerUrl })

  return (
    <>
      <div className={styles.main} style={{ background: '#E0FF32' }}>
        <div>
          <div className={styles.note}>
            <Image src={logo} alt="Payy" width={426 / 2} height={297 / 2} />
          </div>
          <div className={styles.title} style={{ marginTop: 100 }}>
            <Image
              src={payyInviteText}
              alt="Payy Invite"
              width={316}
              height={100}
            />
          </div>
          <Button onClick={handleRedirectToStore}>Download App</Button>
          <Button outline url={`${PAYY_BASE_URL}/invite/${code}`}>
            Open in App
          </Button>
        </div>
      </div>
    </>
  )
}
