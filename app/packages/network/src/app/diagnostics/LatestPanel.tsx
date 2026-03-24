import { FC } from 'react'
import Panel from './Panel'

const LS_LATEST_KEY = 'latest'
const LS_LATEST_PANEL_WIDTH_KEY = 'diagnostics.latest.panel.width'

interface LatestPanelProps {
  latest: string
  setLatest: (latest: string) => void
  latestPanelWidth: number
  setLatestPanelWidth: (width: number) => void
  showToast: (description: string, success: boolean) => void
  clearLocalStorage: () => void
}

const LatestPanel: FC<LatestPanelProps> = ({
  latest,
  setLatest,
  latestPanelWidth,
  setLatestPanelWidth,
  showToast,
  clearLocalStorage
}: LatestPanelProps) => {
  const onLatestChange = (value: string) => {
    try {
      JSON.parse(value)
      localStorage.setItem(LS_LATEST_KEY, value)
      setLatest(value)
      showToast('Latest wallet state parsed successfully', true)
    } catch (err) {
      console.error(err)
      showToast(`While parsing latest wallet state: ${err}`, false)
    }
  }

  return (
    <Panel
      title="Latest"
      localStoragePanelWidthKey={LS_LATEST_PANEL_WIDTH_KEY}
      state={latest}
      panelWidth={latestPanelWidth}
      setPanelWidth={setLatestPanelWidth}
      onChange={onLatestChange}
      clearLocalStorage={clearLocalStorage}
    />
  )
}

export default LatestPanel
