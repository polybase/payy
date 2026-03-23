'use client'

import * as React from 'react'
import { Box, Heading, Stack } from '@chakra-ui/react'
import { useState } from 'react'

export function Section({
  title,
  children
}: {
  title: string
  children: React.ReactNode
}) {
  const [showSection, setShowSection] = useState(
    typeof window !== 'undefined'
      ? localStorage?.getItem(`diagnostics.section.show.${title}`) !== 'false'
      : false
  )

  const onToggle = () => {
    localStorage.setItem(
      `diagnostics.section.show.${title}`,
      (!showSection).toString()
    )
    setShowSection(!showSection)
  }

  return (
    <Stack>
      <Heading cursor="pointer" onClick={onToggle} fontSize="2xl">
        {title}
      </Heading>
      {showSection && <Box>{children}</Box>}
    </Stack>
  )
}
