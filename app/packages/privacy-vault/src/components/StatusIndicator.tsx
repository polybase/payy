import type { ReactNode } from 'react'

export type StepStatus = 'idle' | 'pending' | 'complete' | 'error'

interface StatusIndicatorProps {
  status: StepStatus
  label?: string
}

const iconMap: Record<StepStatus, ReactNode> = {
  idle: null,
  pending: (
    <span className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
  ),
  complete: (
    <span className="flex h-5 w-5 items-center justify-center rounded-full bg-primary text-black">
      <svg
        viewBox="0 0 16 16"
        className="h-3 w-3"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M3.5 8.5l2.5 2.5 6.5-6.5" />
      </svg>
    </span>
  ),
  error: (
    <span className="flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white">
      <svg
        viewBox="0 0 16 16"
        className="h-3 w-3"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      >
        <path d="M4 4l8 8M12 4l-8 8" />
      </svg>
    </span>
  )
}

export const StatusIndicator = ({ status, label }: StatusIndicatorProps) => (
  <div className="flex items-center gap-2 text-xs uppercase tracking-[0.2em] text-text/60">
    {iconMap[status]}
    <span>{label}</span>
  </div>
)
