import { Box, Button, Heading } from '@chakra-ui/react'
import Link from 'next/link'

export const MENU = [
  {
    title: 'Docs',
    href: 'https://payy.network/docs',
    target: '_blank'
  },
  {
    title: 'Explorer',
    href: '/explorer'
  }
]

export interface MenuProps {
  vertical?: boolean
  onClick?: () => void
}

export function Menu({ vertical, onClick }: MenuProps) {
  return (
    <Box display="flex" flexDir={vertical ? 'column' : 'row'} mr={5}>
      {MENU.map(({ title, href, target }) => (
        <Link href={href} target={target} onClick={onClick} key={title}>
          <Box
            py={vertical ? 4 : 2}
            px={vertical ? 2 : 6}
            _hover={{ opacity: 0.8 }}
          >
            <Heading fontSize="lg">{title}</Heading>
          </Box>
        </Link>
      ))}
      <Button
        mt={vertical ? 4 : 0}
        fontWeight="bold"
        as={Link}
        href="https://payy.link/download"
        borderRadius={20}
        px={10}
        width={vertical ? '100%' : undefined}
      >
        Get the app
      </Button>
    </Box>
  )
}
