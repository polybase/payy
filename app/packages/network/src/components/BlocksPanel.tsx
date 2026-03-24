'use client'

import { useQuery } from '@tanstack/react-query'
import { getBlocks } from '../api'
import { Stack, Text, Divider, HStack } from '@chakra-ui/react'
import { Panel } from './Panel'
import { timeSince } from './date'
import { LinkBox } from './LinkBox'

export function BlocksPanel() {
  const query = useQuery({
    queryKey: ['blocks'],
    queryFn: getBlocks,
    refetchInterval: 1000
  })

  return (
    <Panel title="Blocks">
      <Stack divider={<Divider opacity={0.05} />}>
        {query.data?.blocks.map((block) => {
          return (
            <LinkBox key={block.hash} href={`/explorer/blocks/${block.hash}`}>
              <Text noOfLines={1}>{block.hash}</Text>
              <HStack>
                <Text color="gray.600" fontSize="small">
                  {block.block.content.header.height}
                </Text>
                <Text color="gray.600" fontSize="small">
                  {timeSince(block.time * 1000, true)}
                </Text>
              </HStack>
            </LinkBox>
          )
        })}
      </Stack>
    </Panel>
  )
}
