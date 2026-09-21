import failOnConsole from 'vitest-fail-on-console'
import { ALLOWED_CONSOLE_PATTERNS } from './allowed-console.ts'

failOnConsole({
  shouldFailOnWarn: true,
  shouldFailOnError: true,
  silenceMessage: (message: string) =>
    ALLOWED_CONSOLE_PATTERNS.some((pattern) => pattern.test(message)),
})
