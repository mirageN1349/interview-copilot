import { createI18n } from 'vue-i18n'
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import {
  createDiagram,
  DiagramError,
  type Diagram,
} from '@/features/edit-diagram/model/diagram'
import {
  applyDiagramOperations,
  createDiagramHistory,
  undoDiagramChange,
} from '@/features/edit-diagram/model/operations'
import {
  acceptDiagramProposal,
  parseDiagramProposal,
  rejectDiagramProposal,
  type DiagramProposal,
} from '@/features/edit-diagram/model/proposals'
import DiagramEditor from '@/features/edit-diagram/ui/DiagramEditor.vue'

const baseDiagram = (): Diagram => createDiagram({
  revision: 2,
  nodes: [
    { id: 'api', label: 'API', x: 20, y: 30 },
    { id: 'db', label: 'Database', x: 240, y: 30 },
  ],
  edges: [{ id: 'api-db', source: 'api', target: 'db' }],
})

describe('system-design diagram', () => {
  it('validates graph relationships and rejects stale optimistic revisions atomically', () => {
    expect(() => createDiagram({
      revision: 0,
      nodes: [{ id: 'api', label: 'API', x: 0, y: 0 }],
      edges: [{ id: 'missing', source: 'api', target: 'db' }],
    })).toThrowError(DiagramError)

    expect(() => applyDiagramOperations(baseDiagram(), [{
      type: 'node.rename', nodeId: 'api', label: 'Gateway',
    }], 1)).toThrowError('diagram_revision_stale')

    const original = baseDiagram()
    expect(() => applyDiagramOperations(original, [
      { type: 'node.rename', nodeId: 'api', label: 'Gateway' },
      { type: 'edge.add', edge: { id: 'bad', source: 'api', target: 'missing' } },
    ], 2)).toThrowError('diagram_edge_target_missing')
    expect(original.nodes[0]?.label).toBe('API')
  })

  it('creates inverse operations and restores a deleted node with its relationships on undo', () => {
    const history = createDiagramHistory(baseDiagram())
    const changed = applyDiagramOperations(history.diagram, [{
      type: 'node.delete', nodeId: 'db',
    }], 2)
    const next = { diagram: changed.diagram, undoStack: [changed.inverse] }

    expect(next.diagram).toMatchObject({ revision: 3, nodes: [{ id: 'api' }], edges: [] })
    const undone = undoDiagramChange(next)
    expect(undone.diagram.revision).toBe(4)
    expect(undone.diagram.nodes.map(({ id }) => id)).toEqual(['api', 'db'])
    expect(undone.diagram.edges).toEqual([{ id: 'api-db', source: 'api', target: 'db' }])
  })

  it('accepts a proposal only at its base revision and rejects without changing the graph', () => {
    const proposal: DiagramProposal = {
      id: 'proposal-1',
      baseRevision: 2,
      operations: [{ type: 'node.rename', nodeId: 'api', label: 'Gateway' }],
      status: 'pending',
    }
    const accepted = acceptDiagramProposal(createDiagramHistory(baseDiagram()), proposal)
    expect(accepted.proposal.status).toBe('accepted')
    expect(accepted.history.diagram.nodes[0]?.label).toBe('Gateway')

    const rejected = rejectDiagramProposal(createDiagramHistory(baseDiagram()), proposal)
    expect(rejected.proposal.status).toBe('rejected')
    expect(rejected.history.diagram).toEqual(baseDiagram())
  })

  it('decodes bounded WebSocket proposals and excludes internal inverse operations', () => {
    expect(parseDiagramProposal({
      id: 'proposal-1', baseRevision: 2,
      operations: [{ type: 'node.move', nodeId: 'api', x: 40, y: 60 }],
    })).toEqual({
      id: 'proposal-1', baseRevision: 2, status: 'pending',
      operations: [{ type: 'node.move', nodeId: 'api', x: 40, y: 60 }],
    })
    expect(() => parseDiagramProposal({
      id: 'proposal-2', baseRevision: 2,
      operations: [{ type: 'node.restore', node: { id: 'x', label: 'X', x: 0, y: 0 }, edges: [] }],
    })).toThrowError('diagram_proposal_operation_invalid')
  })

  it('supports keyboard create, move, connect, rename, delete, undo and proposal decisions', async () => {
    const i18n = createI18n({
      legacy: false,
      locale: 'en',
      messages: { en: { diagram: {
        title: 'Diagram', canvas: 'Canvas', node: 'Node {label}', relationships: '{label}: {relationships}',
        noRelationships: 'No connections', connectedTo: 'connected to {label}', connectedFrom: 'connected from {label}',
        newNode: 'New node', undo: 'Undo', rename: 'Rename node', proposal: 'Suggested change', accept: 'Accept', reject: 'Reject',
        status: { connect: 'Choose a target', connected: 'Nodes connected', accepted: 'Accepted', rejected: 'Rejected', undone: 'Undone', stale: 'Suggestion is stale' },
      } } },
    })
    const proposal: DiagramProposal = {
      id: 'proposal-1', baseRevision: 2, status: 'pending',
      operations: [{ type: 'node.rename', nodeId: 'api', label: 'Gateway' }],
    }
    const wrapper = mount(DiagramEditor, {
      props: { initialDiagram: baseDiagram(), proposal },
      global: { plugins: [i18n] },
      attachTo: document.body,
    })
    const editor = wrapper.get('[data-testid="diagram-editor"]')

    await editor.trigger('keydown', { key: 'Enter', metaKey: true })
    expect(wrapper.emitted('proposal:accepted')).toHaveLength(1)

    await editor.trigger('keydown', { key: 'n' })
    expect(wrapper.findAll('[data-diagram-node]')).toHaveLength(3)

    const api = wrapper.get('[data-node-id="api"]')
    await api.trigger('focus')
    await editor.trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.emitted('update:diagram')?.at(-1)?.[0]).toMatchObject({ revision: 5 })

    await editor.trigger('keydown', { key: 'c' })
    await wrapper.get('[data-node-id="node-3"]').trigger('focus')
    await editor.trigger('keydown', { key: 'Enter' })
    expect(wrapper.text()).toContain('connected')

    await wrapper.get('[data-node-id="db"]').trigger('focus')
    await editor.trigger('keydown', { key: 'Enter' })
    const input = wrapper.get('[data-testid="diagram-rename"]')
    await input.setValue('Primary database')
    await input.trigger('keydown', { key: 'Enter' })
    expect(wrapper.get('[data-node-id="db"]').text()).toContain('Primary database')

    await editor.trigger('keydown', { key: 'Delete' })
    expect(wrapper.find('[data-node-id="db"]').exists()).toBe(false)
    await editor.trigger('keydown', { key: 'z', metaKey: true })
    expect(wrapper.find('[data-node-id="db"]').exists()).toBe(true)

    await wrapper.setProps({ proposal: { ...proposal, id: 'proposal-2', baseRevision: 8 } })
    await editor.trigger('keydown', { key: 'Backspace', metaKey: true })
    expect(wrapper.emitted('proposal:rejected')).toHaveLength(1)
    wrapper.unmount()
  })
})
