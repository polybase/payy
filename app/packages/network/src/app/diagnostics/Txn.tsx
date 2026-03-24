'use client'

import * as React from 'react'
import {
  Box,
  Heading,
  Text,
  Stack,
  Spacer,
  HStack,
  Tag
} from '@chakra-ui/react'
import { fromBigIntToCurrency } from './util'
import { map } from 'lodash'
import { StoredNote, WalletActivityTxn } from '../../types'
import { Note } from './Note'
import { resultColours } from './colors'

export interface TxnProps {
  id: string
  txn: WalletActivityTxn
}

export function Txn({ txn }: TxnProps) {
  const ownAddress = txn?.data?.inputs?.[0]?.note?.address
  return (
    <Stack
      border="1px solid"
      borderColor="gray.800"
      p={3}
      borderRadius={5}
      spacing={4}
    >
      <HStack>
        <Heading fontSize="lg">{txn?.kind}</Heading>
        <Box>
          <Tag
            size="sm"
            textAlign="center"
            variant="solid"
            colorScheme={resultColours[txn?.result ?? 'active']}
          >
            {txn?.result ?? 'active'}
          </Tag>
        </Box>
        <Text opacity={0.7}>{new Date(txn?.timestamp).toLocaleString()}</Text>
        {txn?.error && (
          <Text opacity={0.7} color="red">
            {txn?.error} ({txn?.errorCycles ?? 0})
          </Text>
        )}
        <Spacer />
        <Text fontSize="lg">
          $
          {fromBigIntToCurrency(
            BigInt(`0x${txn?.data?.outputs?.[0]?.note?.value}`)
          )}
        </Text>
      </HStack>
      {txn?.data?.error && (
        <Text opacity={0.7} color="red">
          {txn?.data?.error.substring(0, 100)}
        </Text>
      )}
      <Stack spacing={4}>
        <Stack spacing={1}>
          <Heading fontSize="md">Root</Heading>
          <Text>{txn?.data?.root}</Text>
        </Stack>
        <HStack alignItems="flex-start">
          <Box width="50%">
            <Stack alignItems="flex-start">
              <Heading fontSize="md">Inputs</Heading>
              <Stack>
                {map(txn?.data?.inputs, (input: StoredNote) => {
                  return (
                    <Note
                      note={input}
                      key={input.commitment}
                      checkCommitment
                      ownAddress={ownAddress}
                    />
                  )
                })}
              </Stack>
            </Stack>
          </Box>
          <Stack width="50%">
            <Heading fontSize="md">Outputs</Heading>
            <Stack>
              {map(txn?.data?.outputs, (output: StoredNote) => {
                return (
                  <Note
                    note={output}
                    key={output.commitment}
                    checkCommitment
                    ownAddress={ownAddress}
                  />
                )
              })}
            </Stack>
          </Stack>
        </HStack>
      </Stack>
    </Stack>
  )
}
