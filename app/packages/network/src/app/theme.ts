import { extendTheme } from '@chakra-ui/react'
import { steradian } from './steradian'

const mode = (light: any, _dark: any) => ({ default: light, _dark })

export const theme = extendTheme({
  fonts: {
    heading:
      steradian.style.fontFamily
      + ', Steradian, "Open Sans", "Source Sans Pro", Arial, Helvetica, sans-serif',
    body:
      steradian.style.fontFamily
      + ', Steradian, "Open Sans", "Source Sans Pro", Arial, Helvetica, sans-serif'
  },
  styles: {
    global: () => ({
      '*': {
        margin: 0,
        padding: 0,
        boxSizing: 'border-box',
        letterSpacing: '-1px'
      },
      '.reset > *': {
        letterSpacing: 0
      },
      'html, body, #root': {
        height: '100%',
        fontSize: '16px',
        color: '#fff'
      },
      body: {
        borderTop: '2px solid #E0FF32',
        fontFamily:
          steradian.style.fontFamily
          + ', Steradian, "Open Sans", "Source Sans Pro", Arial, Helvetica, sans-serif',
        backgroundColor: '#111',
        color: '#fff'
      }
    })
  },
  colors: {
    white: '#111',
    black: '#fff',
    primary: '#E0FF32',
    brand: {
      0: '#FBFFEC',
      50: '#FBFFE5',
      100: '#F4FFB8',
      200: '#EDFF8A',
      300: '#E6FF5C',
      400: '#DFFF2E',
      500: '#E0FF32', // D8FF00
      600: '#ADCC00',
      700: '#829900',
      800: '#576600',
      900: '#2B3300',
      950: '#151b00'
    },
    gray: {
      0: '#FCFCFC',
      50: '#F5F5F5',
      100: '#E5E5E5',
      200: '#DBDBDB',
      300: '#D4D4D4',
      400: '#A3A3A3',
      500: '#8C8C8C',
      600: '#737373',
      700: '#525252',
      800: '#404040',
      850: '#2D2D2D',
      900: '#242424',
      950: '#161616'
    },
    blue: {
      50: '#E5E6FF',
      100: '#B8B9FF',
      200: '#8A8DFF',
      300: '#5C60FF',
      400: '#2E33FF',
      500: '#1D23FF',
      600: '#0005CC',
      700: '#000499',
      800: '#000366',
      900: '#000133'
    }
  },
  semanticTokens: {
    colors: {
      error: 'red.500',
      warning: mode('#ca4b03c7', '#cc630887'),
      bws: mode('rgba(255, 255, 255)', 'rgba(15, 17, 22)'),
      'bws.100': mode('rgba(240, 240, 240)', 'rgba(29, 31, 36)'),
      'bw.10': mode('rgba(0, 0, 0, 0.01)', 'rgba(255, 255, 255, 0.01)'),
      'bw.50': mode('rgba(0, 0, 0, 0.04)', 'rgba(255, 255, 255, 0.04)'),
      'bw.100': mode('rgba(0, 0, 0, 0.06)', 'rgba(255, 255, 255, 0.06)'),
      'bw.200': mode('rgba(0, 0, 0, 0.08)', 'rgba(255, 255, 255, 0.08)'),
      'bw.300': mode('rgba(0, 0, 0, 0.16)', 'rgba(255, 255, 255, 0.16)'),
      'bw.400': mode('rgba(0, 0, 0, 0.24)', 'rgba(255, 255, 255, 0.24)'),
      'bw.500': mode('rgba(0, 0, 0, 0.36)', 'rgba(255, 255, 255, 0.36)'),
      'bw.600': mode('rgba(0, 0, 0, 0.48)', 'rgba(255, 255, 255, 0.48)'),
      'bw.700': mode('rgba(0, 0, 0, 0.64)', 'rgba(255, 255, 255, 0.64)'),
      'bw.800': mode('rgba(0, 0, 0, 0.80)', 'rgba(255, 255, 255, 0.80)'),
      'bw.900': mode('rgba(0, 0, 0, 0.92)', 'rgba(255, 255, 255, 0.92)')
    }
  }
})
