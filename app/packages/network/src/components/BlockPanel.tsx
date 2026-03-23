import { Heading } from '@chakra-ui/react'
import { Panel } from './Panel'

export interface BlockPanelProps {
  height?: number
}

export function BlockPanel({ height }: BlockPanelProps) {
  return (
    <Panel title="Height">
      <Heading size="md" fontWeight="normal">
        {height ?? '-'}
      </Heading>
    </Panel>
  )
}
