import type { ButtonHTMLAttributes, ReactNode } from 'react'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost'
  children: ReactNode
}

const baseClasses =
  'inline-flex items-center justify-center rounded-full px-5 py-2 text-sm font-semibold transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:cursor-not-allowed disabled:opacity-60'

const variantClasses: Record<NonNullable<ButtonProps['variant']>, string> = {
  primary: 'bg-primary text-black hover:brightness-105',
  secondary:
    'border border-gray-800 bg-transparent text-text hover:border-primary hover:text-primary',
  ghost: 'text-text/70 hover:text-text'
}

export const Button = ({
  variant = 'primary',
  className,
  children,
  ...props
}: ButtonProps) => (
  <button
    className={`${baseClasses} ${variantClasses[variant]} ${className || ''}`.trim()}
    type={props.type || 'button'}
    {...props}
  >
    {children}
  </button>
)
