'use client'

import styles from './component.module.css'
import { ReactNode } from 'react'
import { steradian } from '../../app/steradian'

export interface ButtonProps {
  url?: string
  children: ReactNode
  outline?: boolean
  onClick?: () => void
}

export function Button({ url, children, outline, onClick }: ButtonProps) {
  const className = [
    steradian.className,
    styles.btn,
    outline ? styles.btn_outline : styles.btn_solid
  ].join(' ')

  if (url && !onClick) {
    return (
      <div className={styles.btn_container}>
        <a href={url}>
          <button className={className}>{children}</button>
        </a>
      </div>
    )
  }

  return (
    <div className={styles.btn_container}>
      <button className={className} onClick={onClick}>
        {children}
      </button>
    </div>
  )
}
