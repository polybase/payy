'use client'

import { use } from 'react'
import { SimpleGrid, Stack } from '@chakra-ui/react'
import { useQuery } from '@tanstack/react-query'
import { getElement } from '../../../../api'
import { HashPanel } from '../../../../components/HashPanel'
import { PageDetail } from '../../../../components/PageDetail'
import { StatusPanel } from '../../../../components/StatusPanel'

type ElementPageProps = { params: Promise<{ hash: string }> }

export default function Element({ params }: ElementPageProps) {
  const { hash } = use(params)

  const query = useQuery({
    queryKey: ['elements', hash],
    queryFn: async () => {
      const data = await getElement(hash)
      return data
    }
  })

  return (
    <PageDetail
      title="Element"
      loading={query.isLoading}
      notFound={(query.error as any)?.response?.status === 404}
    >
      <Stack spacing="40px">
        <SimpleGrid columns={[1, 1, 3]} spacing="40px">
          <Stack>
            <HashPanel hash={hash} base="elements" />
          </Stack>
          <Stack spacing={4}>
            <StatusPanel height={query.data?.height} type="Element" />
          </Stack>
        </SimpleGrid>
      </Stack>
    </PageDetail>
  )
}
