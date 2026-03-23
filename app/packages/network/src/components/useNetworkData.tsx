'use client'

import axios from 'axios'
import { useInterval } from '@chakra-ui/react'
import { useMemo, useRef, useState } from 'react'

export function useNetworkData() {
  const startRollup = useRef(0)
  const startDate = useRef(Date.now())
  const [rollupHeight, setRollupHeight] = useState<null | number>(null)
  const [contractHeight, setContractHeight] = useState<null | number>(null)
  const [avgTime, setAvgTime] = useState(1)

  useInterval(async () => {
    const [contract, rollup] = await Promise.all([
      axios.get(`${process.env.NEXT_PUBLIC_GUILD_URL}/rollup/contract/status`),
      axios.get(`${process.env.NEXT_PUBLIC_ROLLUP_URL}/health`)
    ])
    setContractHeight(contract.data.height)
    setRollupHeight(rollup.data.height)

    const timeDiff = Date.now() - startDate.current
    const blockDiff = startRollup.current
      ? rollup.data.height - startRollup.current
      : 1

    if (!startRollup.current) {
      startRollup.current = rollup.data.height
      startDate.current = Date.now()
    }

    setAvgTime(Math.floor(timeDiff / (blockDiff > 0 ? blockDiff : 1)) / 1000)
  }, 1000)

  return useMemo(
    () => ({ rollupHeight, contractHeight, avgTime }),
    [rollupHeight, contractHeight, avgTime]
  )
}
