'use client'

import {
  Box,
  HStack,
  Heading,
  Spacer,
  DrawerOverlay,
  Drawer,
  DrawerContent
} from '@chakra-ui/react'
import { HamburgerIcon } from '@chakra-ui/icons'
import { ReactNode, useState } from 'react'
import Link from 'next/link'
import { Logo } from './Logo'
import { Menu } from './Menu'
import { useIsMobile } from './useIsMobile'

export interface LayoutProps {
  line?: boolean
  children: ReactNode
}

export function Layout({ children, line }: LayoutProps) {
  const [isOpen, setIsOpen] = useState(false)

  const sm = useIsMobile()

  return (
    <>
      <Box height="100%" display="flex" flexDirection="column">
        <Box
          p={line ? 3 : 5}
          borderBottom={line ? '1px solid #333' : undefined}
          display="flex"
        >
          <Link href="/">
            <HStack p={2}>
              <Logo fill="#fff" width={120} />
              <Heading
                as="h1"
                position="relative"
                left="-8px"
                top="-1px"
                fontSize={18}
                color="primary"
              >
                NETWORK
              </Heading>
            </HStack>
          </Link>
          <Spacer />
          <Box>
            {sm ? (
              <Box
                p={2}
                cursor="pointer"
                onClick={() => {
                  setIsOpen(true)
                }}
              >
                <HamburgerIcon w={6} h={6} />
              </Box>
            ) : (
              <Menu />
            )}
          </Box>
        </Box>
        <Box height={1} flex="1 1 auto">
          {children}
        </Box>
      </Box>
      {sm && (
        <Drawer
          isOpen={isOpen}
          onClose={() => {
            setIsOpen(false)
          }}
        >
          <DrawerOverlay />
          <DrawerContent>
            <Box p={10}>
              <Menu
                vertical
                onClick={() => {
                  setIsOpen(false)
                }}
              />
            </Box>
          </DrawerContent>
        </Drawer>
      )}
    </>
  )
}
