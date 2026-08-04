import { isRecord, readBoundedString } from '@/shared/api/contracts/common'

import { DiagramError, validateLabel, validateNode } from './diagram'
import type { DiagramOperation } from './operations'
import { commitDiagramChange, type DiagramHistory } from './operations'

export type DiagramProposal = {
  id: string
  baseRevision: number
  operations: DiagramOperation[]
  status: 'pending' | 'accepted' | 'rejected'
}

function readCoordinate(value: unknown) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw new DiagramError('diagram_proposal_coordinate_invalid')
  }
  return value
}

function parseOperation(value: unknown): DiagramOperation {
  if (!isRecord(value) || typeof value.type !== 'string') {
    throw new DiagramError('diagram_proposal_operation_invalid')
  }
  switch (value.type) {
    case 'node.add': {
      if (!isRecord(value.node)) throw new DiagramError('diagram_proposal_operation_invalid')
      return { type: 'node.add', node: validateNode({
        id: readBoundedString(value.node.id, 'node.id', 64),
        label: readBoundedString(value.node.label, 'node.label', 80),
        x: readCoordinate(value.node.x),
        y: readCoordinate(value.node.y),
      }) }
    }
    case 'node.move':
      return {
        type: 'node.move',
        nodeId: readBoundedString(value.nodeId, 'nodeId', 64),
        x: readCoordinate(value.x),
        y: readCoordinate(value.y),
      }
    case 'node.rename':
      return {
        type: 'node.rename',
        nodeId: readBoundedString(value.nodeId, 'nodeId', 64),
        label: validateLabel(readBoundedString(value.label, 'label', 80)),
      }
    case 'node.delete':
      return { type: 'node.delete', nodeId: readBoundedString(value.nodeId, 'nodeId', 64) }
    case 'edge.add': {
      if (!isRecord(value.edge)) throw new DiagramError('diagram_proposal_operation_invalid')
      return { type: 'edge.add', edge: {
        id: readBoundedString(value.edge.id, 'edge.id', 64),
        source: readBoundedString(value.edge.source, 'edge.source', 64),
        target: readBoundedString(value.edge.target, 'edge.target', 64),
        ...(typeof value.edge.label === 'string' ? { label: validateLabel(value.edge.label) } : {}),
      } }
    }
    case 'edge.delete':
      return { type: 'edge.delete', edgeId: readBoundedString(value.edgeId, 'edgeId', 64) }
    default:
      throw new DiagramError('diagram_proposal_operation_invalid')
  }
}

export function parseDiagramProposal(value: unknown): DiagramProposal {
  if (!isRecord(value) || !Number.isSafeInteger(value.baseRevision) || Number(value.baseRevision) < 0) {
    throw new DiagramError('diagram_proposal_invalid')
  }
  if (!Array.isArray(value.operations) || value.operations.length === 0 || value.operations.length > 50) {
    throw new DiagramError('diagram_proposal_operations_invalid')
  }
  return {
    id: readBoundedString(value.id, 'proposal.id', 128),
    baseRevision: Number(value.baseRevision),
    operations: value.operations.map(parseOperation),
    status: 'pending',
  }
}

export function acceptDiagramProposal(history: DiagramHistory, proposal: DiagramProposal) {
  if (proposal.status !== 'pending') throw new DiagramError('diagram_proposal_resolved')
  if (history.diagram.revision !== proposal.baseRevision) throw new DiagramError('diagram_revision_stale')
  return {
    history: commitDiagramChange(history, proposal.operations),
    proposal: { ...proposal, status: 'accepted' as const },
  }
}

export function rejectDiagramProposal(history: DiagramHistory, proposal: DiagramProposal) {
  if (proposal.status !== 'pending') throw new DiagramError('diagram_proposal_resolved')
  return {
    history,
    proposal: { ...proposal, status: 'rejected' as const },
  }
}
