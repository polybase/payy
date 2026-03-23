import { useEffect } from 'react'

type ThemeMode = 'dark' | 'light'

export const useTheme = (): ThemeMode => {
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', 'dark')
    document.documentElement.style.colorScheme = 'dark'
  }, [])

  return 'dark'
}
