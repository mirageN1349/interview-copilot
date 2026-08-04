import {
  createDiagram,
  DiagramError,
  type Diagram,
  type DiagramEdge,
  type DiagramNode,
} from './diagram'

export type DiagramOperation =
  | { type: 'node.add'; node: DiagramNode }
  | { type: 'node.move'; nodeId: string; x: number; y: number }
  | { type: 'node.rename'; nodeId: string; label: string }
  | { type: 'node.delete'; nodeId: string }
  | { type: 'node.restore'; node: DiagramNode; edges: DiagramEdge[] }
  | { type: 'edge.add'; edge: DiagramEdge }
  | { type: 'edge.delete'; edgeId: string }

export type DiagramChange = {
  diagram: Diagram
  inverse: DiagramOperation[]
}

export type DiagramHistory = {
  diagram: Diagram
  undoStack: DiagramOperation[][]
}

function requireNode(diagram: Diagram, nodeId: string) {
  const node = diagram.nodes.find(({ id }) => id === nodeId)
  if (!node) throw new DiagramError('diagram_node_missing')
  return node
}

function requireEdge(diagram: Diagram, edgeId: string) {
  const edge = diagram.edges.find(({ id }) => id === edgeId)
  if (!edge) throw new DiagramError('diagram_edge_missing')
  return edge
}

function applyOne(diagram: Diagram, operation: DiagramOperation): { diagram: Diagram; inverse: DiagramOperation } {
  switch (operation.type) {
    case 'node.add':
      return {
        diagram: createDiagram({ ...diagram, nodes: [...diagram.nodes, operation.node] }),
        inverse: { type: 'node.delete', nodeId: operation.node.id },
      }
    case 'node.move': { const node = requireNode(diagram, operation.nodeId); return {
      diagram: createDiagram({ ...diagram, nodes: diagram.nodes.map((item) => item.id === node.id ? { ...item, x: operation.x, y: operation.y } : item) }),
      inverse: { type: 'node.move', nodeId: node.id, x: node.x, y: node.y },
    } }
    case 'node.rename': { const node = requireNode(diagram, operation.nodeId); return {
      diagram: createDiagram({ ...diagram, nodes: diagram.nodes.map((item) => item.id === node.id ? { ...item, label: operation.label } : item) }),
      inverse: { type: 'node.rename', nodeId: node.id, label: node.label },
    } }
    case 'node.delete': { const node = requireNode(diagram, operation.nodeId); const edges = diagram.edges.filter((edge) => edge.source === node.id || edge.target === node.id); return {
      diagram: createDiagram({ ...diagram, nodes: diagram.nodes.filter(({ id }) => id !== node.id), edges: diagram.edges.filter((edge) => edge.source !== node.id && edge.target !== node.id) }),
      inverse: { type: 'node.restore', node, edges },
    } }
    case 'node.restore':
      return {
        diagram: createDiagram({ ...diagram, nodes: [...diagram.nodes, operation.node], edges: [...diagram.edges, ...operation.edges] }),
        inverse: { type: 'node.delete', nodeId: operation.node.id },
      }
    case 'edge.add':
      return {
        diagram: createDiagram({ ...diagram, edges: [...diagram.edges, operation.edge] }),
        inverse: { type: 'edge.delete', edgeId: operation.edge.id },
      }
    case 'edge.delete': { const edge = requireEdge(diagram, operation.edgeId); return {
      diagram: createDiagram({ ...diagram, edges: diagram.edges.filter(({ id }) => id !== edge.id) }),
      inverse: { type: 'edge.add', edge },
    } }
  }
}

export function applyDiagramOperations(
  diagram: Diagram,
  operations: readonly DiagramOperation[],
  expectedRevision: number,
): DiagramChange {
  if (diagram.revision !== expectedRevision) throw new DiagramError('diagram_revision_stale')
  if (operations.length === 0) throw new DiagramError('diagram_operations_empty')

  let draft = createDiagram(diagram)
  const inverse: DiagramOperation[] = []
  for (const operation of operations) {
    const change = applyOne(draft, operation)
    draft = change.diagram
    inverse.unshift(change.inverse)
  }
  return { diagram: { ...draft, revision: expectedRevision + 1 }, inverse }
}

export function createDiagramHistory(diagram: Diagram): DiagramHistory {
  return { diagram: createDiagram(diagram), undoStack: [] }
}

export function commitDiagramChange(history: DiagramHistory, operations: readonly DiagramOperation[]): DiagramHistory {
  const change = applyDiagramOperations(history.diagram, operations, history.diagram.revision)
  return { diagram: change.diagram, undoStack: [...history.undoStack, change.inverse] }
}

export function undoDiagramChange(history: DiagramHistory): DiagramHistory {
  const inverse = history.undoStack.at(-1)
  if (!inverse) return history
  const change = applyDiagramOperations(history.diagram, inverse, history.diagram.revision)
  return { diagram: change.diagram, undoStack: history.undoStack.slice(0, -1) }
}
