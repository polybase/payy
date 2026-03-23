import { Heading } from '@chakra-ui/react'
import { Panel } from './Panel'
import { LinkBox } from './LinkBox'

export interface HashLinkPanelProps {
  title: string
  hash: string
  base: 'blocks' | 'elements' | 'transactions'
}

export function HashLinkPanel({ title, hash, base }: HashLinkPanelProps) {
  return (
    <Panel title={title}>
      <LinkBox key={hash} href={`/explorer/${base}/${hash}`}>
        <Heading size="md" fontWeight="normal">
          {hash}
        </Heading>
      </LinkBox>
    </Panel>
  )
}
