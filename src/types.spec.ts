import { describe, it, expect } from 'vitest'
import type { LogLevel } from './types'

describe('types', () => {
  it('LogLevel allows valid values', () => {
    const levels: LogLevel[] = ['Off', 'Error', 'Warn', 'Info', 'Debug']
    expect(levels).toHaveLength(5)
    expect(levels).toContain('Info')
  })
})
