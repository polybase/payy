'use client'

import { Layout } from '../../components/Layout'
import { Box, useToast } from '@chakra-ui/react'
import { Dispatch, SetStateAction, useState } from 'react'
import '../../components/resize.styles.css'
import Wallet from './Wallet'
import { WalletState } from '@/types'
import { reconstructWalletsFromDiffs } from './reconstruct'
import LatestPanel from './LatestPanel'
import DiffsPanel from './DiffsPanel'

const LS_LATEST_KEY = 'latest'
const LS_LATEST_PANEL_WIDTH_KEY = 'diagnostics.latest.panel.width'

const LS_DIFFS_KEY = 'diffs'
const LS_DIFFS_PANEL_WIDTH_KEY = 'diagnostics.diffs.panel.width'

export default function Home() {
  const [latest, setLatest] = useState(
    typeof window !== 'undefined'
      ? (localStorage.getItem(LS_LATEST_KEY) ?? '{}')
      : '{}'
  )
  const storedLatestPanelWidth =
    typeof window !== 'undefined'
      ? localStorage.getItem(LS_LATEST_PANEL_WIDTH_KEY)
      : undefined
  const [latestPanelWidth, setLatestPanelWidth] = useState(
    storedLatestPanelWidth ? parseInt(storedLatestPanelWidth, 10) : 300
  )
  const parsedLatest = JSON.parse(latest)
  const wallet = parsedLatest?.wallet ?? (parsedLatest as WalletState)

  const [diffs, setDiffs] = useState(
    typeof window !== 'undefined'
      ? (localStorage.getItem('diffs') ?? '[]')
      : '[]'
  )
  const storedDiffsPanelWidth =
    typeof window !== 'undefined'
      ? localStorage.getItem(LS_DIFFS_PANEL_WIDTH_KEY)
      : undefined
  const [diffsPanelWidth, setDiffsPanelWidth] = useState(
    storedDiffsPanelWidth ? parseInt(storedDiffsPanelWidth, 10) : 300
  )
  const parsedDiffs = JSON.parse(diffs)

  // the first entry in `walletStates` is the latest wallet state
  const walletWithDiff = parsedDiffs.length
    ? reconstructWalletsFromDiffs(wallet, parsedDiffs)
    : {}

  const toast = useToast()

  const clearLocalStorage = (
    storageKey: string,
    setState: Dispatch<SetStateAction<string>>,
    defaultValue: string
  ) => {
    localStorage.removeItem(storageKey)
    setState(defaultValue)
  }

  const showToast = (description: string, success: boolean) => {
    toast({
      title: success ? 'Parsed JSON' : 'Error parsing JSON',
      description,
      status: success ? 'success' : 'error',
      duration: 3000,
      isClosable: true
    })
  }

  return (
    <Layout line>
      <Box height="100%" display="flex">
        <LatestPanel
          latest={latest}
          setLatest={setLatest}
          latestPanelWidth={latestPanelWidth}
          setLatestPanelWidth={setLatestPanelWidth}
          clearLocalStorage={() =>
            clearLocalStorage(LS_LATEST_KEY, setLatest, '{}')
          }
          showToast={showToast}
        />
        <DiffsPanel
          diffs={diffs}
          setDiffs={setDiffs}
          diffsPanelWidth={diffsPanelWidth}
          setDiffsPanelWidth={setDiffsPanelWidth}
          clearLocalStorage={() =>
            clearLocalStorage(LS_DIFFS_KEY, setDiffs, '[]')
          }
          showToast={showToast}
        />
        <Wallet wallet={parsedDiffs.length ? walletWithDiff : wallet} />
      </Box>
    </Layout>
  )
}
