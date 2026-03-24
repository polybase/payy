import { Box, Stack, Text } from '@chakra-ui/react'
import { Section } from './Section'
import { ActivityList } from './Activity'
import { calculateBalanceBigInt, fromBigIntToCurrency } from './util'
import { map, size } from 'lodash'
import { StoredNote, WalletState } from '@/types'
import { Note } from './Note'

interface WalletProps {
  wallet: WalletState
}

const Wallet = ({ wallet }: WalletProps) => {
  return (
    <Box p={5} overflowY="auto" flexGrow={1}>
      <Stack spacing={10}>
        <Section title="Balance">
          <Text textAlign="left" fontSize="xl">
            $
            {fromBigIntToCurrency(
              calculateBalanceBigInt(wallet?.unspent_notes)
            )}
          </Text>
        </Section>
        <Section title={`Unspent Notes (${size(wallet?.unspent_notes)})`}>
          <Stack>
            {map(wallet?.unspent_notes, (note: StoredNote) => {
              return <Note note={note} key={note.commitment} />
            })}
          </Stack>
        </Section>

        <Section title={`Spent Notes (${size(wallet?.spent_notes)})`}>
          <Stack>
            {map(wallet?.spent_notes, (note: StoredNote) => {
              return <Note note={note} key={note.commitment} />
            })}
          </Stack>
        </Section>

        <Section title={`Invalid Notes (${size(wallet?.invalid_notes)})`}>
          <Stack>
            {map(wallet?.invalid_notes, (note: StoredNote) => {
              return <Note note={note} key={note.commitment} />
            })}
          </Stack>
        </Section>

        <Section title={`Activity (${size(wallet?.activity)})`}>
          <ActivityList activities={wallet?.activity} />
        </Section>
      </Stack>
    </Box>
  )
}

export default Wallet
