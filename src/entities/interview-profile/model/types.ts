export type ProfileStatus = 'draft' | 'ready' | 'archived'
export type SourceKind = 'resume' | 'manual' | 'project'
export type ContentStatus = 'pending' | 'allowed' | 'redacted' | 'rejected'
export type ReviewStatus = 'pending' | 'needs_review' | 'confirmed' | 'rejected'
export type AnswerDepth = 'brief' | 'balanced' | 'detailed'

export type ExtractedFact = {
  id: string
  category: string
  text: string
  sourceRange: string
}

export type ProfileSource = {
  id: string
  kind: SourceKind
  displayName: string
  mimeType: string | null
  extractedFacts: ExtractedFact[]
  contentStatus: ContentStatus
  redactionSummary: string | null
  checksum: string | null
}

export type VacancySource = {
  id: string
  sourceKind: 'url' | 'pasted_text'
  sourceValue: string
  roleTitle: string
  companyContext: string
  responsibilities: string[]
  requirements: string[]
  reviewStatus: ReviewStatus
  provenance: {
    fixtureId: string
    extractionModelId: string
    extractedAtMs: number
  }
}

export type ModelConfiguration = {
  id: string
  responseModelId: string
  transcriptionModelId: string
  translationLanguage: string
  answerDepth: AnswerDepth
  questionConfidenceThreshold: number
  processingBoundaryId: string
}

export type ProfileSummary = {
  id: string
  name: string
  status: ProfileStatus
  revision: number
  updatedAtMs: number
}

export type ProfileDetails = ProfileSummary & {
  manualContext: string
  createdAtMs: number
  vacancy: VacancySource | null
  sources: ProfileSource[]
  modelConfiguration: ModelConfiguration | null
}

export type ProfileSaveInput = {
  id?: string
  expectedRevision?: number
  name: string
  manualContext: string
  vacancy: Omit<VacancySource, 'id'> | null
  modelConfiguration: Omit<ModelConfiguration, 'id'> | null
}

export type ProfileSourceImportInput = {
  profileId: string
  expectedRevision: number
  fixtureId: string
  kind: Exclude<SourceKind, 'manual'>
}

export type ModelKind = 'response' | 'transcription' | 'translation'
export type ModelCatalogEntry = {
  id: string
  kind: ModelKind
  name: string
  description: string
  availability: 'available' | 'unavailable' | 'disabled'
  languages: string[]
  capabilities: string[]
}

export type VacancyExtraction = {
  title: string
  company: string
  responsibilities: string[]
  requirements: string[]
  summary: string
  confidence: number
  needsReview: true
  sourceLabel: string
  provenance: {
    fixtureId: string
    extractionModelId: string
    extractedAt: string
  }
}
