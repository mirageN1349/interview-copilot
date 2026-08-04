export type PolicySnapshot = {
  policyVersion: string
  expiresAtMs: number
  verified: boolean
  killSwitch: 'clear' | 'stop_new' | 'stop_all'
}

type PolicyControllerDependencies = {
  stopAll: (reason: 'policy_lost' | 'kill_switch') => void | Promise<void>
  cancelPending: () => void
  emitAudit: (reason: 'POLICY_LOST' | 'POLICY_STALE' | 'KILL_SWITCH_ACTIVE') => void
  now?: () => number
}

export function createPolicyController(dependencies: PolicyControllerDependencies) {
  const now = dependencies.now ?? Date.now
  let snapshot: PolicySnapshot | undefined
  let stopped = false

  async function stopOnce(reason: 'policy_lost' | 'kill_switch', auditReason: 'POLICY_LOST' | 'POLICY_STALE' | 'KILL_SWITCH_ACTIVE') {
    if (stopped) return false
    stopped = true
    dependencies.cancelPending()
    try {
      await dependencies.stopAll(reason)
    } finally {
      dependencies.emitAudit(auditReason)
    }
    return true
  }

  return {
    subscribe(deviceId: string) {
      return { type: 'policy.subscribe' as const, payload: { deviceId } }
    },
    async apply(next: PolicySnapshot) {
      snapshot = next
      if (!next.verified || next.expiresAtMs <= now()) return stopOnce('policy_lost', 'POLICY_STALE')
      if (next.killSwitch !== 'clear') return stopOnce('kill_switch', 'KILL_SWITCH_ACTIVE')
      return false
    },
    async tick() {
      if (!snapshot || !snapshot.verified || snapshot.expiresAtMs <= now()) {
        return stopOnce('policy_lost', snapshot ? 'POLICY_STALE' : 'POLICY_LOST')
      }
      return false
    },
    transportLost() {
      snapshot = undefined
      return stopOnce('policy_lost', 'POLICY_LOST')
    },
    blocked: () => stopped,
  }
}
