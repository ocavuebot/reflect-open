import { hasBridge, setBridge, type AppPlatform } from '@reflect/core'

import { createDevBridge } from '@/dev/dev-bridge.ts'
import { createDevFileStore } from '@/dev/dev-file-store.ts'
import { createDevIndexDb } from '@/dev/dev-index-db.ts'
import { seedGraphFiles } from '@/dev/seed-graph.ts'
import { parseEnvPlatform } from '@/lib/env.ts'

const appPlatform: AppPlatform = (() => {
  if (import.meta.env.DEV && typeof window !== 'undefined') {
    const query = new URLSearchParams(window.location.search).get('platform')
    switch (query) {
      case 'ios':
        return 'ios'
      case 'android':
        return 'android'
      case 'desktop':
        return 'desktop'
    }
    if (query) {
      console.warn('[reflect-open] Unknown platform override in query string:', query)
    }
  }

  return parseEnvPlatform()
})()

if (!hasBridge()) {
  const index = await createDevIndexDb()
  const files = createDevFileStore(seedGraphFiles())
  const devBridge = createDevBridge({ files, index })
  setBridge(devBridge)
}

const { PlatformRoot, warmPlatformRoot } = await (async () => {
  if (appPlatform === 'desktop') {
    const { warmPlatformRoot, PlatformRoot } = await import('@/platform-root.desktop.tsx')
    return { PlatformRoot, warmPlatformRoot }
  } else {
    const { warmPlatformRoot, PlatformRoot } = await import('@/platform-root.mobile.tsx')
    return { PlatformRoot, warmPlatformRoot }
  }
})()

export { PlatformRoot, warmPlatformRoot }
