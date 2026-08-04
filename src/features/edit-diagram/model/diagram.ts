export const DIAGRAM_LIMITS = {
  nodes: 100,
  edges: 300,
  labelLength: 80,
  coordinate: 100_000,
} as const

const identifier = /^[a-zA-Z0-9][a-zA-Z0-9._:-]{0,63}$/

export type DiagramNode = {
  id: string
  label: string
  x: number
  y: number
}

export type DiagramEdge = {
  id: string
  source: string
  target: string
  label?: string
}

export type Diagram = {
  revision: number
  nodes: DiagramNode[]
  edges: DiagramEdge[]
}

export class DiagramError extends Error {
  constructor(public readonly code: string) {
    super(code)
    this.name = 'DiagramError'
  }
}

function validateId(id: string, kind: 'node' | 'edge') {
  if (!identifier.test(id)) throw new DiagramError(`diagram_${kind}_id_invalid`)
}

export function validateLabel(label: string) {
  const normalized = label.trim()
  if (!normalized || normalized.length > DIAGRAM_LIMITS.labelLength) {
    throw new DiagramError('diagram_label_invalid')
  }
  return normalized
}

function validateCoordinate(value: number) {
  if (!Number.isFinite(value) || Math.abs(value) > DIAGRAM_LIMITS.coordinate) {
    throw new DiagramError('diagram_coordinate_invalid')
  }
}

export function validateNode(node: DiagramNode): DiagramNode {
  validateId(node.id, 'node')
  validateCoordinate(node.x)
  validateCoordinate(node.y)
  return { ...node, label: validateLabel(node.label) }
}

export function validateEdge(edge: DiagramEdge, nodeIds: ReadonlySet<string>): DiagramEdge {
  validateId(edge.id, 'edge')
  if (!nodeIds.has(edge.source)) throw new DiagramError('diagram_edge_source_missing')
  if (!nodeIds.has(edge.target)) throw new DiagramError('diagram_edge_target_missing')
  if (edge.source === edge.target) throw new DiagramError('diagram_edge_self_reference')
  return edge.label === undefined ? { ...edge } : { ...edge, label: validateLabel(edge.label) }
}

export function createDiagram(input: Diagram): Diagram {
  if (!Number.isSafeInteger(input.revision) || input.revision < 0) {
    throw new DiagramError('diagram_revision_invalid')
  }
  if (input.nodes.length > DIAGRAM_LIMITS.nodes) throw new DiagramError('diagram_node_limit')
  if (input.edges.length > DIAGRAM_LIMITS.edges) throw new DiagramError('diagram_edge_limit')

  const nodes = input.nodes.map(validateNode)
  const nodeIds = new Set(nodes.map(({ id }) => id))
  if (nodeIds.size !== nodes.length) throw new DiagramError('diagram_node_duplicate')
  const edges = input.edges.map((edge) => validateEdge(edge, nodeIds))
  if (new Set(edges.map(({ id }) => id)).size !== edges.length) {
    throw new DiagramError('diagram_edge_duplicate')
  }
  const relationships = new Set<string>()
  for (const edge of edges) {
    const relationship = `${edge.source}\0${edge.target}`
    if (relationships.has(relationship)) throw new DiagramError('diagram_edge_duplicate_relationship')
    relationships.add(relationship)
  }
  return { revision: input.revision, nodes, edges }
}

export function emptyDiagram(): Diagram {
  return { revision: 0, nodes: [], edges: [] }
}
