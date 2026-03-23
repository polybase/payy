'use client'

import { useQuery } from '@tanstack/react-query'
import { getTxns } from '../api'
import { Stack, Text, Divider, HStack } from '@chakra-ui/react'
import { Panel } from './Panel'
import { timeSince } from './date'
import { LinkBox } from './LinkBox'

export function TxnsPanel() {
  const query = useQuery({
    queryKey: ['txns'],
    queryFn: getTxns,
    refetchInterval: 1000
  })

  return (
    <Panel title="Txns">
      <Stack divider={<Divider opacity={0.05} />}>
        {query.data?.txns?.map((txn) => {
          return (
            <LinkBox key={txn.hash} href={`/explorer/transactions/${txn.hash}`}>
              <Text noOfLines={1}>{txn.hash}</Text>
              <HStack>
                <Text color="gray.600" fontSize="small">
                  {txn.block_height}
                </Text>
                <Text color="gray.600" fontSize="small">
                  {timeSince(txn.time * 1000)}
                </Text>
              </HStack>
            </LinkBox>
          )
        })}
      </Stack>
    </Panel>
  )
}
