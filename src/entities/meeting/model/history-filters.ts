export type HistoryFilters = {
  query?: string
  field?: 'any' | 'title' | 'vacancy' | 'transcript' | 'chat'
  profileQuery?: string
  fromMs?: number
  toMs?: number
  cursor?: string
  limit: number
}

export type HistoryMeetingSummary = {
  id: string
  title: string
  profileName: string
  profileId: string
  vacancyRole: string
  mode: string
  status: string
  createdAtMs: number
  endedAtMs: number | null
  retentionExpiresAtMs: number
}

export type HistoryPage = {
  items: HistoryMeetingSummary[]
  nextCursor: string | null
}

export type HistoryArtifact = {
  id: string
  kind: string
  mimeType: string
  byteLength: number
  contentStatus: string
  createdAtMs: number
  expiresAtMs: number
}

export type HistoryMessage = {
  id: string
  sequence: number
  role: 'user' | 'assistant' | 'system'
  content: string
  createdAtMs: number
}

export type HistoryTranscriptSegment = { id: string; sequence: number; speaker: string; text: string; confidence: number; isQuestion: boolean; startedAtMs: number; endedAtMs: number }
export type HistoryChat = { kind: 'live' | 'side'; messages: HistoryMessage[] }

export type HistoryMeetingDetail = HistoryMeetingSummary & {
  startedAtMs: number | null
  transcript: HistoryTranscriptSegment[]
  chats: HistoryChat[]
  artifacts: HistoryArtifact[]
}

export function normalizeHistoryFilters(input: Partial<HistoryFilters>): HistoryFilters {
  const query = input.query?.trim().replace(/\s+/g, ' ').slice(0, 200)
  const profileQuery = input.profileQuery?.trim().replace(/\s+/g, ' ').slice(0, 120)
  const from = Number.isFinite(input.fromMs) ? Math.max(0, Number(input.fromMs)) : undefined
  const to = Number.isFinite(input.toMs) ? Math.max(0, Number(input.toMs)) : undefined
  const [fromMs, toMs] = from !== undefined && to !== undefined && from > to ? [to, from] : [from, to]
  return {
    ...(query ? { query } : {}),
    field: input.field ?? 'any',
    ...(profileQuery ? { profileQuery } : {}),
    ...(fromMs !== undefined ? { fromMs } : {}),
    ...(toMs !== undefined ? { toMs } : {}),
    ...(input.cursor ? { cursor: input.cursor.slice(0, 512) } : {}),
    limit: Math.min(100, Math.max(1, Math.trunc(input.limit ?? 30))),
  }
}
