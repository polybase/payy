import { FC } from 'react'
import { Box } from '@chakra-ui/react'
import CodeMirror from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { codemirrorTheme } from '../../components/codemirror.theme'
import { debounce } from 'lodash'
import { ResizableBox } from 'react-resizable'
import '../../components/resize.styles.css'

interface PanelProps {
  title: string
  localStoragePanelWidthKey: string
  state: string
  panelWidth: number
  setPanelWidth: (width: number) => void
  onChange: (value: string) => void
  clearLocalStorage: () => void
}

const Panel: FC<PanelProps> = ({
  title,
  localStoragePanelWidthKey,
  state,
  panelWidth,
  setPanelWidth,
  onChange
}: PanelProps) => {
  const onPanelChange = debounce((_: any, data: any) => {
    localStorage.setItem(localStoragePanelWidthKey, data.size.width)
    setPanelWidth(data.size.width)
  }, 400)

  return (
    <ResizableBox
      width={panelWidth}
      resizeHandles={['e']}
      axis="x"
      onResize={onPanelChange}
    >
      <Box height="100%" display="flex" flexDirection="column">
        <Box fontSize="small" p={1} bg="#191919" textTransform="uppercase">
          {title}
        </Box>
        <Box
          overflowY="auto"
          borderRight="1px solid #333"
          width="100%"
          flex={1}
        >
          <CodeMirror
            style={{ fontSize: 13 }}
            value={state ?? '[]'}
            theme={codemirrorTheme}
            extensions={[javascript({ jsx: true })]}
            onChange={onChange}
          />
        </Box>
      </Box>
    </ResizableBox>
  )
}

export default Panel
