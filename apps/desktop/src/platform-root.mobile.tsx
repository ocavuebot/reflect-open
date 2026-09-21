import { parseEnvPlatform } from '@/lib/env.ts'
import { warmMobileStorage } from '@/lib/mobile-boot-warm.ts'
import { MobileRoot } from '@/mobile/mobile-root.tsx'
import type { ReactElement } from 'react'

const platform = parseEnvPlatform() === 'ios' ? 'ios' : 'android'

/** Starts the slow iCloud-container resolve ahead of the first render. */
export function warmPlatformRoot(): void {
  warmMobileStorage()
}

export function PlatformRoot(): ReactElement {
  return <MobileRoot platform={platform} />
}
