import { ReactNode } from 'react'
import { Box } from '@chakra-ui/react'
import Link from 'next/link'
import { Url } from 'next/dist/shared/lib/router/router'

export interface LinkBoxProps {
  children: ReactNode
  href: Url
}

export function LinkBox({ children, href }: LinkBoxProps) {
  return (
    <Link href={href}>
      <Box
        borderRadius="sm"
        _hover={{ bg: 'gray.900', cursor: 'pointer' }}
        px={2}
      >
        {children}
      </Box>
    </Link>
  )
}
