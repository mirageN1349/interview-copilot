import type { ProfileDetails } from '@/entities/interview-profile'

export type ProfileReadinessReason =
  | 'name_missing'
  | 'vacancy_review'
  | 'source_review'
  | 'model_configuration_missing'
  | 'response_model_unavailable'
  | 'transcription_model_unavailable'

export type ProfileReadiness = { ready: boolean; reasons: ProfileReadinessReason[] }

export function evaluateProfileReadiness(
  profile: ProfileDetails,
  availableModelIds: ReadonlySet<string>,
): ProfileReadiness {
  const reasons: ProfileReadinessReason[] = []
  if (!profile.name.trim()) reasons.push('name_missing')
  if (profile.vacancy && profile.vacancy.reviewStatus !== 'confirmed') reasons.push('vacancy_review')
  if (profile.sources.some((source) => !['allowed', 'redacted'].includes(source.contentStatus))) reasons.push('source_review')
  if (!profile.modelConfiguration) reasons.push('model_configuration_missing')
  else {
    if (!availableModelIds.has(profile.modelConfiguration.responseModelId)) reasons.push('response_model_unavailable')
    if (!availableModelIds.has(profile.modelConfiguration.transcriptionModelId)) reasons.push('transcription_model_unavailable')
  }
  return { ready: reasons.length === 0, reasons }
}
