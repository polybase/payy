'use client'

import { Heading, Stack, useBreakpoint } from '@chakra-ui/react'
import { Panel } from './Panel'
// import QRCode from 'react-qr-code'
import { QRCodeSVG } from 'qrcode.react'

export interface HashPanelProps {
  hash: string
  base: 'blocks' | 'elements' | 'transactions'
}

export function HashPanel({ hash, base }: HashPanelProps) {
  const breakpoint = useBreakpoint()
  const qrCodeValue = `${window.location.origin ?? ''}/explorer/${base}/${hash}`

  return (
    <Panel title="Hash">
      <Stack spacing={4}>
        <Heading size="md" fontWeight="normal">
          {hash}
        </Heading>
        {breakpoint === 'xs'
        || breakpoint === 'sm'
        || breakpoint === 'base' ? null : (
          <QRCodeSVG
            bgColor="#181818"
            fgColor="#fff"
            width="100%"
            height="auto"
            value={qrCodeValue}
            imageSettings={{
              src: '/icon.svg',
              height: 44,
              width: 40,
              excavate: true
            }}
          />
          // <QRCode
          //   bgColor='#181818'
          //   fgColor='#fff'
          //   value={qrCodeValue}
          // />
        )}
      </Stack>
    </Panel>
  )
}
