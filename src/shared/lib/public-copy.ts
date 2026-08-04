const restrictedTerms = [
  /\badversarial\b/i,
  /\baudit\b/i,
  /\bcapture[- ]?matrix\b/i,
  /\bkill[- ]?switch\b/i,
  /\blaunch[- ]?policy\b/i,
  /\bresearcher\b/i,
  /\bsandbox\b/i,
  /\bsecurity policy\b/i,
  /\bstealth\b/i,
  /\bэкспериментатор\b/i,
  /\bисследователь\b/i,
  /\bпесочниц\w*\b/i,
]

export function assertPublicCopy(value: string): string {
  if (restrictedTerms.some((term) => term.test(value))) {
    throw new Error('RESTRICTED_COPY')
  }
  return value
}

export function publicMeetingUnavailable(reasonCode?: string) {
  void reasonCode
  return {
    titleKey: 'meeting.unavailable.title',
    detailKey: 'meeting.unavailable.detail',
    actionKey: 'meeting.unavailable.action',
  } as const
}
