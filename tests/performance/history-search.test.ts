import { describe, expect, it } from 'vitest'

type Cursor = { createdAtMs: number; id: string }

const beforeCursor = (item: Cursor, cursor: Cursor) =>
  item.createdAtMs < cursor.createdAtMs ||
  (item.createdAtMs === cursor.createdAtMs && item.id < cursor.id)

describe('history search performance contract', () => {
  it('keeps stable cursor ordering across 10,000 synthetic rows', () => {
    const meetings = Array.from({ length: 10_000 }, (_, index) => ({
      createdAtMs: Math.floor(index / 2),
      id: `meeting-${index.toString().padStart(5, '0')}`,
    })).sort((left, right) =>
      right.createdAtMs - left.createdAtMs || right.id.localeCompare(left.id),
    )
    const cursor = meetings[99]
    const next = meetings.filter((meeting) => beforeCursor(meeting, cursor)).slice(0, 100)

    expect(next).toEqual(meetings.slice(100, 200))
  })
})
