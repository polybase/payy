'use client'

import { Panel } from '../../components/Panel'
import { Layout } from '../../components/Layout'
import { SimpleGrid, Center, Stack, Box, Heading } from '@chakra-ui/react'
import { useNetworkData } from '../../components/useNetworkData'
import { BlocksPanel } from '../../components/BlocksPanel'
import { TxnChart } from '../../components/TxnChart'
import { TxnsPanel } from '../../components/TxnsPanel'

export default function Explorer() {
  const { rollupHeight, avgTime, contractHeight } = useNetworkData()

  return (
    <Layout>
      <Box p={8} maxW={1200} margin="0 auto">
        <Stack spacing="40px">
          <SimpleGrid columns={[1, 1, 3]} spacing="40px">
            <Panel title="Avg Block Time">
              <Box height="80px">
                <Center>
                  <Heading fontSize={60} color="black">
                    {avgTime}s
                  </Heading>
                </Center>
              </Box>
            </Panel>
            <Panel title="Sequencer Height">
              <Box height="80px">
                <Center>
                  <Heading fontSize={60} color="black">
                    {rollupHeight?.toLocaleString()}
                  </Heading>
                </Center>
              </Box>
            </Panel>
            <Panel title="Rollup Height">
              <Box height="80px">
                <Center>
                  <Heading fontSize={60} color="black">
                    {contractHeight?.toLocaleString()}
                  </Heading>
                </Center>
              </Box>
            </Panel>
          </SimpleGrid>
          <SimpleGrid columns={[1]} spacing="40px">
            <Panel title="Txns over time">
              <Box height="220px">
                <TxnChart />
              </Box>
            </Panel>
          </SimpleGrid>
          <SimpleGrid columns={[1, 1, 2]} spacing="40px">
            <BlocksPanel />
            <TxnsPanel />
          </SimpleGrid>
        </Stack>
      </Box>
    </Layout>
  )
}
