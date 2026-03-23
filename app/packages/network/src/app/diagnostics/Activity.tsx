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
import { useState } from 'react'
import { fromBigIntToCurrency } from './util'
import { map } from 'lodash'
import { WalletActivity, WalletActivityTxn } from '../../types'
import { resultColours } from './colors'
import { Txn } from './Txn'

export interface ActivityListProps {
  activities: Record<string, WalletActivity>
}

export function ActivityList({ activities }: ActivityListProps) {
  const activitiesArray = map(
    activities,
    (activity: WalletActivity, id: string) => ({
      id,
      activity
    })
  )

  // Parent activities
  const parentActivities = activitiesArray.filter((a) => !a.activity.parentId)

  return (
    <Stack>
      {map(parentActivities, ({ id, activity }) => {
        const children = activitiesArray.filter(
          (a) => a.activity.parentId === id
        )
        return (
          <Activity
            id={id}
            activity={activity}
            key={id}
            subActivities={children}
          />
        )
      })}
    </Stack>
  )
}

export interface ActivityProps {
  id: string
  activity: WalletActivity
  subActivities: { id: string; activity: WalletActivity }[]
}

export function Activity({ activity, subActivities }: ActivityProps) {
  const [isOpen, setIsOpen] = useState(false)
  const value =
    activity?.data?.value
    ?? activity?.data?.note?.note?.value
    ?? activity?.data?.outputs?.[0]?.note?.value
  return (
    <Stack border="1px solid" borderColor="gray.800" p={3} borderRadius={5}>
      <HStack>
        <Heading
          cursor="pointer"
          fontSize="lg"
          minWidth={50}
          onClick={() => {
            setIsOpen(!isOpen)
          }}
        >
          {activity?.kind}
        </Heading>
        <Box>
          <Tag
            size="sm"
            textAlign="center"
            variant="solid"
            colorScheme={resultColours[activity?.result ?? 'active']}
          >
            {activity?.result ?? 'active'}
          </Tag>
        </Box>
        <Text opacity={0.7}>
          {new Date(activity?.timestamp).toLocaleString()}
        </Text>
        {activity?.error && (
          <Text opacity={0.7} color="red">
            {activity?.error} ({activity?.errorCycles ?? 0})
          </Text>
        )}
        <Spacer />
        <Text fontSize="lg">
          ${value ? fromBigIntToCurrency(BigInt(`0x${value}`)) : ''}
        </Text>
      </HStack>
      {isOpen && (
        <Stack>
          {map(subActivities, ({ id, activity }) => {
            return <Txn id={id} txn={activity as WalletActivityTxn} key={id} />
          })}
        </Stack>
      )}
    </Stack>
  )
}
