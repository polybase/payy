import { tokyoNightInit } from '@uiw/codemirror-theme-tokyo-night'
import { CreateThemeOptions } from '@uiw/codemirror-themes'

export const settings: CreateThemeOptions['settings'] = {
  background: '#161616',
  foreground: '#787c99',
  caret: '#c0caf5',
  selection: '#515c7e40',
  selectionMatch: '#16161e',
  gutterBackground: '#161616',
  gutterForeground: '#787c99',
  gutterBorder: 'transparent',
  lineHighlight: '#474b6611'
}

export const codemirrorTheme = tokyoNightInit({ settings })
