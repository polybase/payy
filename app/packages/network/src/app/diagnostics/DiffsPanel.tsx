import { FC } from 'react'
import Panel from './Panel'

const LS_DIFFS_KEY = 'diffs'
const LS_DIFFS_PANEL_WIDTH_KEY = 'diagnostics.diffs.panel.width'

interface DiffsPanelProps {
  diffs: string
  setDiffs: (diff: string) => void
  diffsPanelWidth: number
  setDiffsPanelWidth: (width: number) => void
  showToast: (description: string, success: boolean) => void
  clearLocalStorage: () => void
}

const DiffsPanel: FC<DiffsPanelProps> = ({
  diffs,
  setDiffs,
  diffsPanelWidth,
  setDiffsPanelWidth,
  showToast,
  clearLocalStorage
}: DiffsPanelProps) => {
  const onDiffsChange = (value: string) => {
    try {
      JSON.parse(value)
      localStorage.setItem(LS_DIFFS_KEY, value)
      setDiffs(value)
      showToast('Wallet backup diffs parsed successfully', true)
    } catch (err) {
      console.error(err)
      showToast(`While parsing wallet state backup diffs: ${err}`, false)
    }
  }

  return (
    <Panel
      title="Diff"
      localStoragePanelWidthKey={LS_DIFFS_PANEL_WIDTH_KEY}
      state={diffs}
      panelWidth={diffsPanelWidth}
      setPanelWidth={setDiffsPanelWidth}
      onChange={onDiffsChange}
      clearLocalStorage={clearLocalStorage}
    />
  )
}

export default DiffsPanel
