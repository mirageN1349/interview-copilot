import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { describe, expect, it } from 'vitest'

import type { AssistantMessage, RecognizedQuestion } from '@/features/overlay-chat/model/meeting-reducer'
import LiveChat from '@/features/overlay-chat/ui/LiveChat.vue'
import SideChat from '@/features/overlay-chat/ui/SideChat.vue'
import { en } from '@/shared/config/i18n/en'

const i18n = () => createI18n({ legacy: false, locale: 'en', messages: { en } })
const answer = (id: string, content: string): AssistantMessage => ({
  id, content, thread: 'side', status: 'complete', profileSourceIds: [], contextGeneration: 0,
})
const question = (segmentId: string, text: string, requiresConfirmation = false): RecognizedQuestion => ({
  segmentId, text, requiresConfirmation, confidence: requiresConfirmation ? 0.5 : 0.95,
})

describe('overlay message chronology', () => {
  it('keeps side questions and answers in arrival order', async () => {
    const wrapper = mount(SideChat, { props: { messages: [], timeline: [] }, global: { plugins: [i18n()] } })
    await wrapper.setProps({
      messages: [answer('a1', 'First answer'), answer('a2', 'Second answer')],
      timeline: [
        { id: 'u1', role: 'user', content: 'First question' },
        { id: 'a1', role: 'assistant' },
        { id: 'u2', role: 'user', content: 'Second question' },
        { id: 'a2', role: 'assistant' },
      ],
    })
    await flushPromises()

    expect(wrapper.findAll('[data-timeline-role]').map((node) => node.attributes('data-timeline-role')))
      .toEqual(['user', 'assistant', 'user', 'assistant'])
  })

  it('keeps live interviewer questions and assistant answers in arrival order and confirms low confidence', async () => {
    const wrapper = mount(LiveChat, {
      props: { messages: [], question: null, timeline: [] },
      global: { plugins: [i18n()] },
    })

    const firstQuestion = question('q1', 'First live question')
    const secondQuestion = question('q2', 'Second live question', true)
    await wrapper.setProps({
      question: secondQuestion,
      messages: [answer('a1', 'First live answer'), answer('a2', 'Second live answer')],
      timeline: [
        { id: 'q1', role: 'interviewer', question: firstQuestion },
        { id: 'a1', role: 'assistant' },
        { id: 'q2', role: 'interviewer', question: secondQuestion },
        { id: 'a2', role: 'assistant' },
      ],
    })
    await flushPromises()

    expect(wrapper.findAll('[data-timeline-role]').map((node) => node.attributes('data-timeline-role')))
      .toEqual(['interviewer', 'assistant', 'interviewer', 'assistant'])
    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('confirmQuestion')).toEqual([['q2']])
    expect(wrapper.text()).toContain(en.overlay.live.interviewer)
    expect(wrapper.text()).toContain(en.overlay.live.assistant)
  })
})
