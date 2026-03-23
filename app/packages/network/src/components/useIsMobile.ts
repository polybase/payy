'use client'

import { useBreakpoint } from '@chakra-ui/react'

export function useIsMobile() {
  const breakpoint = useBreakpoint()
  return breakpoint === 'xs' || breakpoint === 'sm' || breakpoint === 'base'
}
