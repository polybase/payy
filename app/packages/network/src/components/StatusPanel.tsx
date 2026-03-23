import { HStack, Heading, Stack, Text, Box } from '@chakra-ui/react'
import { Panel } from './Panel'
import { useNetworkData } from './useNetworkData'
import TickIcon from '../img/tick.svg'

export interface StatusPanelProps {
  height?: number
  type?: 'Element' | 'Block' | 'Transaction'
}

export function StatusPanel({ height, type }: StatusPanelProps) {
  const { contractHeight } = useNetworkData()
  const isHard = contractHeight && height && contractHeight >= height

  const STATUSES = {
    soft: {
      title: 'Soft confirmation',
      desc: `${type ?? 'Block'} has been included in a sequenced block, but has not been rolled up to the L1 chain.`,
      color: 'green'
    },
    hard: {
      title: 'Hard confirmation',
      desc: `${type ?? 'Block'} has been rolled up to base layer (Polygon) and now has the full security of the base layer`,
      color: 'green'
    }
  }

  const status = isHard ? STATUSES.hard : STATUSES.soft

  return (
    <Panel title="Status">
      <Stack>
        <HStack>
          <Box filter="grayscale(100%);">
            <TickIcon width={27} height={27} />
          </Box>
          <Heading size="md" fontWeight="normal">
            {status.title}
          </Heading>
        </HStack>
        <Text opacity={0.6}>{status.desc}</Text>
      </Stack>
    </Panel>
  )
}
