'use client'

import * as React from 'react'
import { Heading, Text, Stack, Spacer, HStack, Spinner } from '@chakra-ui/react'
import { CheckCircleIcon, WarningIcon } from '@chakra-ui/icons'
import { fromBigIntToCurrency } from './util'
import { StoredNote, Element } from '../../types'
import { useAsync } from 'react-async-hook'
import axios from 'axios'

export interface NoteProps {
  note: StoredNote
  checkCommitment?: boolean
  ownAddress?: Element
}

export function Note({ note, ownAddress, checkCommitment }: NoteProps) {
  const commitment = useAsync(async () => {
    if (!checkCommitment) return
    try {
      const res = await axios.get(
        `${process.env.NEXT_PUBLIC_ROLLUP_URL}/elements/${note.commitment}`
      )
      return res.data
    } catch (e: any) {
      if (e?.response?.status === 404) {
        throw new Error('Commitment not in tree')
      }
      throw e
    }
  }, [])

  return (
    <Stack border="1px solid" borderColor="gray.800" p={3} borderRadius={5}>
      <HStack>
        {commitment.loading ? (
          <Spinner size="sm" color="gray.500" />
        ) : commitment.error ? (
          <WarningIcon color="red" />
        ) : (
          <CheckCircleIcon />
        )}
        <Heading fontSize="lg" wordBreak="break-all">
          {note?.commitment}{' '}
          {commitment.result?.height ? `(${commitment.result?.height})` : ''}
          {ownAddress && (
            <Text opacity={0.7} fontSize="small">
              {note?.note?.address === ownAddress ? 'Internal' : 'External'}
            </Text>
          )}
        </Heading>
        <Spacer />
        <Text fontSize="lg">
          ${fromBigIntToCurrency(BigInt(`0x${note?.note?.value}`))}
        </Text>
      </HStack>
      {commitment.error && <Text color="red">{commitment.error.message}</Text>}
    </Stack>
  )
}
