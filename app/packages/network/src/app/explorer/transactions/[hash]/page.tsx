'use client'

import { use } from 'react'
import { Heading, SimpleGrid, Stack } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { getBlock, getTxn } from '../../../../api'
import { BlockPanel } from '../../../../components/BlockPanel'
import { HashLinkPanel } from '../../../../components/HashLinkPanel'
import { HashPanel } from '../../../../components/HashPanel'
import { PageDetail } from '../../../../components/PageDetail'
import { Panel } from '../../../..//components/Panel'
import { StatusPanel } from '../../../../components/StatusPanel'
import { TimePanel } from '../../../../components/TimePanel'

const NULL_LEAF =
  '0000000000000000000000000000000000000000000000000000000000000000'

type TxnPageProps = { params: Promise<{ hash: string }> }

export default function Txn({ params }: TxnPageProps) {
  const { hash } = use(params)

  const query = useQuery({
    queryKey: ['txns', hash],
    queryFn: async () => {
      const data = await getTxn(hash)
      return data
    }
  })

  const height = query.data?.txn.block_height

  const blockQuery = useQuery({
    queryKey: ['block', query.data?.txn.block_height],
    queryFn: async () => {
      if (!height) return
      const data = await getBlock(`${height}`)
      return data
    }
  })

  return (
    <PageDetail
      title="Transaction"
      loading={query.isLoading}
      notFound={(query.error as any)?.response?.status === 404}
    >
      <Stack spacing="40px">
        <SimpleGrid columns={[1, 1, 3]} spacing="40px">
          <HashPanel hash={hash} base="blocks" />
          <Stack spacing={4}>
            <StatusPanel
              height={query.data?.txn.block_height}
              type="Transaction"
            />
            <TimePanel timestamp={query.data?.txn.time} />
          </Stack>
          <Stack spacing={4}>
            {blockQuery.data?.hash ? (
              <HashLinkPanel
                title="Block"
                hash={blockQuery.data?.hash}
                base="blocks"
              />
            ) : null}
            <BlockPanel height={query.data?.txn.block_height} />
          </Stack>
        </SimpleGrid>
      </Stack>
      <Stack>
        <Heading size="lg">Inputs/Outputs</Heading>
        <SimpleGrid columns={[1, 1, 3]} spacing="40px">
          <Stack spacing={4}>
            {query.data?.txn.proof.public_inputs.input_commitments
              ?.filter((leaf) => leaf !== NULL_LEAF)
              .map((input) => (
                <HashLinkPanel
                  key={input}
                  title="Input Nullifier"
                  hash={input}
                  base="elements"
                />
              ))}
          </Stack>
          <Stack spacing={4}>
            {query.data?.txn.proof.public_inputs.output_commitments
              ?.filter((leaf) => leaf !== NULL_LEAF)
              .map((input) => (
                <HashLinkPanel
                  key={input}
                  title="Output Commitment"
                  hash={input}
                  base="elements"
                />
              ))}
          </Stack>
          <Stack spacing={4}></Stack>
        </SimpleGrid>
      </Stack>
      <Stack>
        <Heading size="lg">Proof</Heading>
        <Panel
          title="Barretenberg Proof"
          titleLink="https://github.com/AztecProtocol/barretenberg"
        >
          <Heading size="md" fontWeight="normal">
            {query.data?.txn.proof.proof}
          </Heading>
        </Panel>
      </Stack>
    </PageDetail>
  )
}
