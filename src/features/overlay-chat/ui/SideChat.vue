<script setup lang="ts">
import { ArrowUp, Check, ChevronsUpDown, Mic, Plus } from '@lucide/vue'
import { computed, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { AnswerDepth } from '@/entities/interview-profile'
import type { AssistantMessage } from '../model/meeting-reducer'
import Button from '@/shared/ui/button/Button.vue'
import MessageContent from '@/shared/ui/code/MessageContent.vue'
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover'

const props = defineProps<{
  messages: AssistantMessage[]
  pending?: boolean
  timeline: Array<{ id: string; role: 'user'; content: string } | { id: string; role: 'assistant' }>
}>()
type SendRequest = { content: string; depth: AnswerDepth }
const emit = defineEmits<{ send: [request: SendRequest]; attach: [] }>()
const { t } = useI18n()
const draft = ref('')
const messagesById = computed(() => new Map(props.messages.map((message) => [message.id, message])))
const depth = ref<AnswerDepth>('balanced')
const depthOpen = ref(false)
const depthList = ref<HTMLElement | null>(null)
const depthValues: AnswerDepth[] = ['brief', 'balanced', 'detailed']
const depthLabel = computed(() => t(`profiles.models.depthOption.${depth.value}`))

function submit() {
  const content = draft.value.trim()
  if (!content) return
  emit('send', { content, depth: depth.value })
  draft.value = ''
}

function selectDepth(value: AnswerDepth) {
  depth.value = value
  depthOpen.value = false
}

function moveDepthFocus(event: KeyboardEvent, step: 1 | -1) {
  const options = [...(event.currentTarget as HTMLElement).closest('[role="listbox"]')!.querySelectorAll<HTMLButtonElement>('[role="option"]')]
  options[(options.indexOf(event.currentTarget as HTMLButtonElement) + step + options.length) % options.length]?.focus()
}

function focusDepthBoundary(event: KeyboardEvent, index: 0 | -1) {
  const options = (event.currentTarget as HTMLElement).closest('[role="listbox"]')!.querySelectorAll<HTMLButtonElement>('[role="option"]')
  options[index === 0 ? 0 : options.length - 1]?.focus()
}

function focusSelectedDepth(event: Event) {
  event.preventDefault()
  void nextTick(() => depthList.value?.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus())
}
</script>

<template>
  <section
    aria-labelledby="overlay-side-tab"
    class="relative min-h-0 overflow-hidden"
  >
    <div class="h-full overflow-y-auto px-4 pb-36 pt-[6.25rem]">
      <div
        v-if="timeline.length"
        class="grid gap-2.5"
      >
        <template
          v-for="entry in timeline"
          :key="`${entry.role}-${entry.id}`"
        >
          <p
            v-if="entry.role === 'user'"
            data-timeline-role="user"
            class="user-bubble ml-auto max-w-[88%] rounded-[1.15rem] rounded-br-md px-3.5 py-2.5 text-sm"
          >
            {{ entry.content }}
          </p>
          <article
            v-else-if="messagesById.get(entry.id)"
            data-timeline-role="assistant"
            class="assistant-bubble max-w-[88%] rounded-[1.15rem] rounded-bl-md px-3.5 py-2.5 text-sm"
            :data-status="messagesById.get(entry.id)!.status"
          >
            <MessageContent :content="messagesById.get(entry.id)!.content" />
          </article>
        </template>
      </div>
      <p
        v-else
        class="text-sm text-(--muted-foreground)"
      >
        {{ t('overlay.side.empty') }}
      </p>
    </div>
    <form
      class="composer absolute inset-x-3 bottom-3 grid gap-2 rounded-[1.65rem] border p-2.5"
      @submit.prevent="submit"
    >
      <textarea
        id="overlay-side-input"
        v-model="draft"
        rows="2"
        class="max-h-24 min-h-12 w-full resize-none border-0 bg-transparent px-2 py-1 text-[0.9375rem] leading-6 outline-none [field-sizing:content]"
        :placeholder="t('overlay.side.placeholder')"
        @keydown.enter.exact.prevent="submit"
      />
      <div class="flex items-center justify-between gap-3">
        <div class="flex min-w-0 items-center gap-2">
          <button
            class="glass-action inline-flex size-9 shrink-0 items-center justify-center rounded-full"
            type="button"
            :aria-label="t('overlay.side.attach')"
            :title="t('overlay.side.attach')"
            @click="$emit('attach')"
          >
            <Plus
              :size="18"
              aria-hidden="true"
            />
          </button>
          <Popover v-model:open="depthOpen">
            <PopoverTrigger as-child>
              <button
                class="depth-trigger inline-flex h-9 items-center gap-1.5 rounded-full border px-3 text-xs font-medium"
                type="button"
                :aria-label="t('profiles.models.depth')"
              >
                <span>AI · {{ depthLabel }}</span>
                <ChevronsUpDown
                  :size="13"
                  aria-hidden="true"
                  class="text-(--muted-foreground)"
                />
              </button>
            </PopoverTrigger>
            <PopoverContent
              class="depth-menu w-48 rounded-[1.1rem] border p-1.5 text-(--foreground) shadow-xl"
              align="start"
              side="top"
              :side-offset="8"
              :aria-label="t('profiles.models.depth')"
              @open-auto-focus="focusSelectedDepth"
            >
              <div
                ref="depthList"
                role="listbox"
                :aria-label="t('profiles.models.depth')"
              >
                <button
                  v-for="value in depthValues"
                  :key="value"
                  class="depth-option flex min-h-9 w-full items-center justify-between rounded-xl px-3 text-left text-sm"
                  type="button"
                  role="option"
                  :aria-selected="depth === value"
                  @click="selectDepth(value)"
                  @keydown.down.prevent="moveDepthFocus($event, 1)"
                  @keydown.up.prevent="moveDepthFocus($event, -1)"
                  @keydown.home.prevent="focusDepthBoundary($event, 0)"
                  @keydown.end.prevent="focusDepthBoundary($event, -1)"
                >
                  {{ t(`profiles.models.depthOption.${value}`) }}
                  <Check
                    v-if="depth === value"
                    :size="15"
                    aria-hidden="true"
                    class="text-(--accent)"
                  />
                </button>
              </div>
            </PopoverContent>
          </Popover>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <span
            class="glass-action inline-flex size-9 items-center justify-center rounded-full text-(--muted-foreground)"
            :aria-label="t('overlay.side.listening')"
            role="status"
          >
            <Mic
              :size="17"
              aria-hidden="true"
            />
          </span>
          <Button
            size="icon"
            :disabled="pending || !draft.trim()"
            :aria-label="t('overlay.side.send')"
            type="submit"
          >
            <ArrowUp
              :size="18"
              :stroke-width="2.5"
              aria-hidden="true"
            />
          </Button>
        </div>
      </div>
    </form>
  </section>
</template>

<style scoped>
.assistant-bubble,
.glass-action,
.depth-trigger {
  border-color: color-mix(in oklch, var(--foreground) 10%, transparent);
  background: color-mix(in oklch, var(--surface-raised) 48%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 30%, transparent);
}

:global(.depth-menu) {
  border-color: color-mix(in oklch, var(--foreground) 12%, transparent);
  background: color-mix(in oklch, var(--surface-raised) 92%, transparent);
  box-shadow: 0 1rem 2.5rem oklch(0.12 0.02 250 / 0.28), inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 50%, transparent);
}

:global(.depth-option:hover),
:global(.depth-option:focus-visible),
:global(.depth-option[aria-selected="true"]) {
  background: color-mix(in oklch, var(--surface-hover) 76%, transparent);
  outline: none;
}

.composer {
  border-color: color-mix(in oklch, var(--foreground) 10%, transparent);
  background: color-mix(in oklch, var(--surface-raised) 10%, transparent);
  backdrop-filter: blur(1.5rem) saturate(140%);
  -webkit-backdrop-filter: blur(1.5rem) saturate(140%);
  box-shadow:
    0 1rem 2.5rem oklch(0.12 0.02 250 / 0.24),
    0 0.2rem 0.8rem oklch(0.12 0.02 250 / 0.14),
    inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 42%, transparent);
  transition: box-shadow var(--motion-fast) var(--ease-out-expo);
}

.composer:focus-within {
  box-shadow:
    0 1rem 2.5rem oklch(0.12 0.02 250 / 0.24),
    0 0 0 3px color-mix(in oklch, var(--accent) 13%, transparent),
    inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 50%, transparent);
}

.composer :deep(:is(textarea, button):focus-visible) {
  outline: none;
  box-shadow: none;
}

.glass-action {
  border: 1px solid color-mix(in oklch, var(--foreground) 9%, transparent);
}

.user-bubble {
  background: color-mix(in oklch, var(--accent) 78%, transparent);
  color: var(--accent-foreground);
}

.assistant-bubble[data-status="streaming"] {
  opacity: 0.86;
}
</style>
