import { ExternalLinkIcon } from '@chakra-ui/icons'
import { Box, Heading } from '@chakra-ui/react'
import { ReactNode } from 'react'

export interface PanelProps {
  title: string
  titleLink?: string
  children: ReactNode
}

export function Panel({ title, titleLink, children }: PanelProps) {
  const titleContent = titleLink ? (
    <Box
      as="a"
      href={titleLink}
      target="_blank"
      rel="noopener noreferrer"
      display="inline-flex"
      alignItems="center"
      gap={1}
      color="inherit"
      textDecoration="none"
      aria-label={`${title} (opens in a new tab)`}
      _hover={{ textDecoration: 'underline' }}
    >
      {title}
      <ExternalLinkIcon aria-hidden="true" boxSize="0.75em" />
    </Box>
  ) : (
    title
  )

  return (
    <Box bg="#181818" borderRadius={10} p={2}>
      <Box p={2}>
        <Heading
          fontWeight={600}
          fontSize="small"
          color="#fff"
          opacity={0.3}
          textTransform="uppercase"
        >
          {titleContent}
        </Heading>
      </Box>
      <Box p={2}>{children}</Box>
    </Box>
  )
}
