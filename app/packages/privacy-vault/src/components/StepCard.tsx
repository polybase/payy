import type { ReactNode } from 'react'

import { StatusIndicator, type StepStatus } from './StatusIndicator'

interface StepCardProps {
  step: number
  title: string
  description?: string
  status: StepStatus
  isOpen: boolean
  onToggle: () => void
  children: ReactNode
}

const statusLabel: Record<StepStatus, string> = {
  idle: 'Not started',
  pending: 'In progress',
  complete: 'Complete',
  error: 'Action needed'
}

export const StepCard = ({
  step,
  title,
  description,
  status,
  isOpen,
  onToggle,
  children
}: StepCardProps) => (
  <section
    className={`rounded-2xl border border-gray-800 bg-[var(--surface)] p-6 transition-shadow ${
      isOpen ? 'shadow-glow' : 'shadow-none'
    }`}
  >
    <button
      type="button"
      onClick={onToggle}
      className="flex w-full items-center justify-between text-left"
      aria-expanded={isOpen}
    >
      <div className="flex items-start gap-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-full border border-gray-800 text-sm font-semibold text-text/70">
          {step}
        </div>
        <div>
          <h2 className="text-lg font-semibold text-text">{title}</h2>
          {description && (
            <p className="mt-1 text-sm text-text/60">{description}</p>
          )}
        </div>
      </div>
      <StatusIndicator status={status} label={statusLabel[status]} />
    </button>
    <div
      className={`grid transition-all duration-300 ${
        isOpen ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'
      }`}
    >
      <div className="overflow-hidden">
        <div className="mt-6 border-t border-gray-800 pt-6">{children}</div>
      </div>
    </div>
  </section>
)
