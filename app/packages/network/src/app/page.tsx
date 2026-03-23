'use client'

import {
  Center,
  Heading,
  Box,
  Stack,
  HStack,
  Button,
  Text
} from '@chakra-ui/react'
import Image from 'next/image'
import { Layout } from '../components/Layout'
import { Logo } from '../components/Logo'
import { useIsMobile } from '../components/useIsMobile'
import Link from 'next/link'
import { useNetworkData } from '../components/useNetworkData'

export default function Home() {
  const isMobile = useIsMobile()
  const { rollupHeight, avgTime, contractHeight } = useNetworkData()

  return (
    <Layout>
      <Box>
        <Stack spacing={6}>
          <Center mt={10}>
            <Heading
              lineHeight={'82px'}
              textAlign="center"
              letterSpacing="-0.08em"
              fontSize={92}
              maxW={670}
            >
              The end-game for stablecoins
            </Heading>
          </Center>
          <Center mt={6}>
            <Box maxW={800} textAlign="center">
              <Text fontSize={24} color="gray.600" lineHeight={1.4}>
                Payy Network is the first vertically integrated stablecoin
                payments network that enables onchain finance for all of
                humanity.
              </Text>
            </Box>
          </Center>
          <Center mt={8}>
            <Button
              as={Link}
              href="https://docs.payy.network"
              target="_blank"
              rel="noopener noreferrer"
              size="lg"
              bg="primary"
              color="#111"
              _hover={{ color: '#111' }}
              borderRadius={30}
              px={10}
              py={6}
              fontSize={18}
              fontWeight="bold"
            >
              Read the docs
            </Button>
          </Center>
          <Box>
            <Center>
              <HStack spacing={2}>
                <Heading fontSize={24} color="black">
                  Powering
                </Heading>
                <Link href="https://payy.link" target="_blank">
                  <Logo fill="#fff" width={120} />
                </Link>
              </HStack>
            </Center>
          </Box>
          <Center mt={10}>
            {rollupHeight && (
              <Box
                maxW={1000}
                display="flex"
                flexDir={isMobile ? 'column' : 'row'}
              >
                <Stat title="AVG BLOCK TIME" value={`${avgTime}s`} />
                <Stat title="SEQUENCER HEIGHT" value={rollupHeight} />
                <Stat title="ROLLUP HEIGHT" value={contractHeight} />
              </Box>
            )}
          </Center>
          <Box
            position="absolute"
            overflow="hidden"
            width="100%"
            top={400}
            height={400}
            zIndex={-1}
          >
            <Box minW={800}>
              <Image src={require('../img/globe.png')} alt="globe" />
            </Box>
          </Box>
        </Stack>
      </Box>
    </Layout>
  )
}

interface StatProps {
  title: string
  value?: string | number | null
}

function Stat({ title, value }: StatProps) {
  return (
    <Box px={10} py={5}>
      <Heading fontSize={18} color="primary" wordBreak="keep-all">
        {title}
      </Heading>
      <Heading fontSize={60} color="black">
        {value}
      </Heading>
    </Box>
  )
}
