'use client'

import { use } from 'react'
import { SimpleGrid, Stack, Heading, Text } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { getBlock } from '../../../../api'
import { BlockPanel } from '../../../../components/BlockPanel'
import { HashLinkPanel } from '../../../../components/HashLinkPanel'
import { HashPanel } from '../../../../components/HashPanel'
import { PageDetail } from '../../../../components/PageDetail'
import { StatusPanel } from '../../../../components/StatusPanel'
import { TimePanel } from '../../../../components/TimePanel'

type BlockPageProps = { params: Promise<{ hash: string }> }

export default function Block({ params }: BlockPageProps) {
  const { hash } = use(params)

  const query = useQuery({
    queryKey: ['blocks', hash],
    queryFn: async () => {
      const data = await getBlock(hash)
      return data
    }
  })

  const txns = query.data?.block.content.state.txns

  return (
    <PageDetail
      title="Block"
      loading={query.isLoading}
      notFound={(query.error as any)?.response?.status === 404}
    >
      <Stack spacing="40px">
        <SimpleGrid columns={[1, 1, 3]} spacing="40px">
          <Stack>
            <HashPanel hash={hash} base="blocks" />
          </Stack>
          <Stack spacing={4}>
            <StatusPanel height={query.data?.block.content.header.height} />
            <TimePanel timestamp={query.data?.time} />
          </Stack>
          <Stack>
            <BlockPanel height={query.data?.block.content.header.height} />
          </Stack>
        </SimpleGrid>
      </Stack>
      <Stack>
        <Heading size="lg">
          Transactions ({query.data?.block.content.state.txns.length ?? 0})
        </Heading>
        <SimpleGrid columns={[1, 1, 3]} spacing="40px">
          {txns?.length ? (
            txns.map((txn, i) => {
              return (
                <Stack key={txn.hash}>
                  <HashLinkPanel
                    title={`#${i + 1}`}
                    hash={txn.hash}
                    base="transactions"
                  />
                </Stack>
              )
            })
          ) : (
            <Text opacity={0.6}>No transactions in this block</Text>
          )}
        </SimpleGrid>
      </Stack>
    </PageDetail>
  )
}
