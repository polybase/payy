export function timeSince(timestamp: number, exact?: boolean) {
  const now = +new Date()
  const past = timestamp

  // Calculate the time difference in milliseconds
  const timeDifference = now - past

  // Calculate time difference in minutes, hours, and days
  const seconds = Math.floor(timeDifference / 1000)
  const minutes = Math.floor(timeDifference / (1000 * 60))
  const hours = Math.floor(timeDifference / (1000 * 60 * 60))
  const days = Math.floor(timeDifference / (1000 * 60 * 60 * 24))

  if (minutes < 1) {
    return exact
      ? `${seconds} second${seconds === 1 ? '' : 's'} ago`
      : 'Just now'
  }

  if (minutes < 60) {
    return `${minutes} min${minutes === 1 ? '' : 's'} ago`
  }

  if (hours < 24) {
    return `${hours} hour${hours === 1 ? '' : 's'} ago`
  }

  return `${days} day${days === 1 ? '' : 's'} ago`
}
