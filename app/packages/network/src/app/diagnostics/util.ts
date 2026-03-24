import { Element, StoredNote } from '../../types'

export function calculateBalanceBigInt(
  notes?: Record<Element, StoredNote>
): bigint {
  if (!notes) return BigInt(0)
  return Object.values(notes).reduce((total, note) => {
    return total + BigInt(`0x${note.note.value}`)
  }, BigInt(0))
}

export function fromElementToBigInt(element?: Element | null): bigint {
  if (!element) return BigInt(0)
  return BigInt(`0x${element}`)
}

export function parseCurrencyToBigInt(currencyString: string): bigint {
  // Remove the dollar sign and other non-numeric characters
  const numericString = currencyString.replace(/[^0-9.]/g, '')

  // Split the string into whole and decimal parts
  const [whole, decimalRaw] = numericString.split('.')

  // Ensure the decimal part has 6 digits (USDC has 6 decimal places)
  const decimal = ((decimalRaw ?? '') + '000000').slice(0, 6)

  // Combine the whole and decimal parts
  const combined = whole + decimal

  // Convert to BigInt
  return BigInt(combined)
}

export function fromBigIntToCurrency(balance: bigint): string {
  const balanceString = balance.toString().padStart(6, '0')
  const whole = balanceString.slice(0, -6) ?? '0'
  const decimal = balanceString.slice(-6).slice(0, 2).padStart(2, '0')
  const wholeWithComma = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  const wholeWithDefault = wholeWithComma === '' ? '0' : wholeWithComma
  if (decimal === '00') return wholeWithDefault
  return `${wholeWithDefault}.${decimal}`
}

export function toElement(number: number | bigint): Element {
  const hex = number.toString(16)
  if (hex.length > 64) throw new Error('Number is too large')
  return hex.padStart(64, '0')
}

export function toElementFromHex(hex: string): Element {
  if (hex.startsWith('0x')) hex = hex.slice(2)
  if (hex.length > 64) throw new Error('Hex is too large')
  return hex.padStart(64, '0')
}

export function toBigInt(number: number | bigint): bigint {
  return BigInt(number)
}
