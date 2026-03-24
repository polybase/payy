export function detectBrowser(
  agent?: string
): 'ios' | 'android' | 'blackberry' | null {
  if (!agent) agent = navigator.userAgent

  // iPhone Version:
  if (/iPhone|iPod|iPad/i.test(agent)) {
    return 'ios'
  }

  // Android Version:
  if (agent.match(/android/i)) {
    return 'android'
  }
  // Blackberry Version:
  if (agent.match(/blackberry/i)) {
    return 'blackberry'
  }

  return null
}
