import type { ReactNode } from 'react'

import { Logo } from './Logo'

interface LayoutProps {
  children: ReactNode
}

export const Layout = ({ children }: LayoutProps) => (
  <div className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(224,255,50,0.15),_transparent_45%),radial-gradient(circle_at_20%_20%,_rgba(255,255,255,0.08),_transparent_40%)]">
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-8 px-6 pb-20 pt-10">
      <header className="flex flex-col gap-4">
        <div className="flex items-center gap-3 text-text">
          <Logo width={120} height={20} fill="currentColor" />
        </div>
        <div>
          <h1 className="mt-8 text-3xl font-semibold text-text sm:text-4xl">
            Setup your Privacy Vault
          </h1>
          <p className="mt-3 max-w-2xl text-base text-text/60">
            The privacy vault protects your privacy, its authenticated so that
            only you can see your private balances.
          </p>
        </div>
      </header>
      <main className="flex flex-col gap-4">{children}</main>
    </div>
  </div>
)
