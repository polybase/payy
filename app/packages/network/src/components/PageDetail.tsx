import { ReactNode } from 'react'
import { Layout } from './Layout'
import { Stack, Box, Heading } from '@chakra-ui/react'

export interface PageDetailProps {
  title: string
  children: ReactNode
  loading?: boolean
  notFound?: boolean
}

export function PageDetail({
  title,
  children,
  loading,
  notFound
}: PageDetailProps) {
  const status = notFound ? 'Not found' : loading ? 'Loading...' : undefined
  return (
    <Layout>
      <Box p={8} maxW={1200} margin="0 auto" pb={20}>
        <Heading>{title}</Heading>
        <Stack mt={4} spacing={12}>
          {status ? (
            <Heading fontSize="md" opacity={0.6}>
              {status}
            </Heading>
          ) : (
            children
          )}
        </Stack>
      </Box>
    </Layout>
  )
}
