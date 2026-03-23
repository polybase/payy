import { Heading, Stack, Text } from '@chakra-ui/react'
import { Panel } from './Panel'
import { timeSince } from './date'

export interface TimePanelProps {
  timestamp?: number
}

export function TimePanel({ timestamp }: TimePanelProps) {
  return (
    <Panel title="Timestamp">
      <Stack>
        <Heading size="md" fontWeight="normal">
          {timestamp ? new Date(timestamp * 1000).toISOString() : 'Loading...'}
        </Heading>
        <Text opacity={0.6}>
          {timestamp ? timeSince(timestamp * 1000) : '-'}
        </Text>
      </Stack>
    </Panel>
  )
}
