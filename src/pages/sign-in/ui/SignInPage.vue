<script setup lang="ts">
import { ArrowLeft, ArrowRight, AudioLines, Check, Command, Layers3, LockKeyhole } from '@lucide/vue'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { requestMagicLink } from '@/shared/api/auth/client'
import Button from '@/shared/ui/button/Button.vue'
import Input from '@/shared/ui/input/Input.vue'

const { t } = useI18n()
const router = useRouter()
const email = ref('')
const pending = ref(false)
const error = ref('')
const currentStep = ref(0)
const lastStep = 3
const isFinalStep = computed(() => currentStep.value === lastStep)
const steps = [
  { icon: Layers3, key: 'context', visual: 'context' },
  { icon: AudioLines, key: 'live', visual: 'live' },
  { icon: Command, key: 'keyboard', visual: 'keyboard' },
] as const

function goTo(step: number) {
  currentStep.value = Math.min(lastStep, Math.max(0, step))
  error.value = ''
}

function onKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return
  if (event.key === 'ArrowLeft') goTo(currentStep.value - 1)
  if (event.key === 'ArrowRight') goTo(currentStep.value + 1)
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))

async function submit() {
  pending.value = true
  error.value = ''
  try {
    await requestMagicLink(email.value)
    await router.push({ path: '/sign-in/check-email', query: { email: email.value.trim().toLowerCase() } })
  } catch {
    error.value = t('auth.signIn.error')
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <main class="onboarding-page relative isolate min-h-screen overflow-x-hidden bg-(--background) p-4 text-(--foreground) sm:p-7">
    <section class="onboarding-shell relative z-10 mx-auto flex min-h-[calc(100vh-2rem)] w-full max-w-6xl flex-col overflow-hidden rounded-3xl border border-(--border) sm:min-h-[calc(100vh-3.5rem)]">
      <header class="flex items-center justify-between gap-4 px-5 py-4 sm:px-8 sm:py-6">
        <div class="inline-flex items-center gap-2.5 text-sm font-semibold tracking-tight">
          <span
            class="onboarding-brand-mark"
            aria-hidden="true"
          ><span /></span>
          {{ t('auth.productName') }}
        </div>
        <p class="text-xs font-medium text-(--muted-foreground)">
          {{ t('auth.signIn.onboarding.step', { current: currentStep + 1, total: lastStep + 1 }) }}
        </p>
      </header>

      <div class="flex flex-1 items-center px-5 py-6 sm:px-8 lg:px-14">
        <Transition
          name="onboarding-step"
          mode="out-in"
        >
          <article
            v-if="!isFinalStep"
            :key="currentStep"
            class="grid w-full items-center gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:gap-20"
          >
            <div class="max-w-xl">
              <component
                :is="steps[currentStep]!.icon"
                :size="24"
                class="mb-6 text-(--accent)"
                aria-hidden="true"
              />
              <p class="mb-3 text-sm font-semibold text-(--accent)">
                {{ t(`auth.signIn.onboarding.steps.${steps[currentStep]!.key}.kicker`) }}
              </p>
              <h1 class="text-4xl font-semibold leading-[1.06] tracking-[-0.04em] sm:text-5xl">
                {{ t(`auth.signIn.onboarding.steps.${steps[currentStep]!.key}.title`) }}
              </h1>
              <p class="mt-5 max-w-[62ch] text-base leading-7 text-(--muted-foreground)">
                {{ t(`auth.signIn.onboarding.steps.${steps[currentStep]!.key}.description`) }}
              </p>
              <ul class="mt-8 grid gap-3">
                <li
                  v-for="index in 3"
                  :key="index"
                  class="flex items-start gap-3 text-sm leading-6"
                >
                  <span class="mt-1 grid size-4 shrink-0 place-items-center rounded-full bg-[color-mix(in_oklch,var(--accent)_15%,transparent)] text-(--accent)">
                    <Check
                      :size="11"
                      :stroke-width="2.5"
                      aria-hidden="true"
                    />
                  </span>
                  {{ t(`auth.signIn.onboarding.steps.${steps[currentStep]!.key}.point${index}`) }}
                </li>
              </ul>
            </div>

            <div
              class="onboarding-visual"
              aria-hidden="true"
            >
              <div
                v-if="steps[currentStep]!.visual === 'context'"
                class="visual-context"
              >
                <div class="visual-title-line" />
                <div
                  v-for="index in 3"
                  :key="index"
                  class="visual-profile-row"
                >
                  <span class="visual-profile-icon" />
                  <span><i /><i /></span>
                  <Check :size="14" />
                </div>
              </div>
              <div
                v-else-if="steps[currentStep]!.visual === 'live'"
                class="visual-live"
              >
                <div class="visual-window-bar">
                  <i /><i /><i />
                </div>
                <div class="visual-wave">
                  <i
                    v-for="index in 22"
                    :key="index"
                    :style="{ '--bar': `${18 + ((index * 17) % 58)}%` }"
                  />
                </div>
                <div class="visual-question">
                  <i /><span />
                </div>
                <div class="visual-answer">
                  <i /><i /><i />
                </div>
              </div>
              <div
                v-else
                class="visual-keyboard"
              >
                <div class="visual-key-row">
                  <kbd>⌘</kbd><kbd>⇧</kbd><kbd>Space</kbd>
                </div>
                <div class="visual-response-lines">
                  <i /><i /><i /><i />
                </div>
                <div class="visual-key-row visual-key-row--small">
                  <kbd>⌘ K</kbd><kbd>↵</kbd><kbd>Esc</kbd>
                </div>
              </div>
            </div>
          </article>

          <article
            v-else
            key="sign-in"
            class="grid w-full items-center gap-10 lg:grid-cols-[0.9fr_1.1fr] lg:gap-20"
          >
            <div class="max-w-xl">
              <LockKeyhole
                :size="24"
                class="mb-6 text-(--accent)"
                aria-hidden="true"
              />
              <p class="mb-3 text-sm font-semibold text-(--accent)">
                {{ t('auth.signIn.formKicker') }}
              </p>
              <h1 class="text-4xl font-semibold leading-[1.06] tracking-[-0.04em] sm:text-5xl">
                {{ t('auth.signIn.title') }}
              </h1>
              <p class="mt-5 max-w-[56ch] text-base leading-7 text-(--muted-foreground)">
                {{ t('auth.signIn.onboarding.finalDescription') }}
              </p>
            </div>

            <form
              class="sign-in-form grid w-full gap-5 rounded-2xl border border-(--border) p-6 shadow-(--shadow-dialog) sm:p-8"
              @submit.prevent="submit"
            >
              <p
                id="sign-in-description"
                class="text-sm leading-6 text-(--muted-foreground)"
              >
                {{ t('auth.signIn.description') }}
              </p>
              <div class="grid gap-2">
                <label
                  class="text-sm font-semibold"
                  for="email"
                >{{ t('auth.signIn.emailLabel') }}</label>
                <Input
                  id="email"
                  v-model="email"
                  class="min-h-11"
                  name="email"
                  type="email"
                  autocomplete="email"
                  required
                  aria-describedby="sign-in-description"
                  :placeholder="t('auth.signIn.emailPlaceholder')"
                />
              </div>
              <p
                v-if="error"
                role="alert"
                class="m-0 rounded-(--radius-sm) bg-[color-mix(in_oklch,var(--danger)_9%,transparent)] px-3 py-2 text-sm text-(--danger)"
              >
                {{ error }}
              </p>
              <Button
                class="min-h-11 w-full"
                type="submit"
                :disabled="pending"
              >
                {{ pending ? t('auth.signIn.sending') : t('auth.signIn.continue') }}
              </Button>
              <p class="flex items-start gap-2 text-xs leading-5 text-(--muted-foreground)">
                <LockKeyhole
                  :size="14"
                  class="mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                {{ t('auth.signIn.privacy') }}
              </p>
            </form>
          </article>
        </Transition>
      </div>

      <footer class="grid grid-cols-[1fr_auto_1fr] items-center gap-4 px-5 py-4 sm:px-8 sm:py-6">
        <Button
          variant="ghost"
          class="w-fit"
          :disabled="currentStep === 0"
          @click="goTo(currentStep - 1)"
        >
          <ArrowLeft
            :size="16"
            aria-hidden="true"
          />
          <span class="hidden sm:inline">{{ t('auth.signIn.onboarding.back') }}</span>
        </Button>

        <div
          class="flex items-center gap-2"
          :aria-label="t('auth.signIn.onboarding.progressLabel')"
        >
          <button
            v-for="step in lastStep + 1"
            :key="step"
            type="button"
            class="onboarding-dot"
            :class="{ 'onboarding-dot--active': currentStep === step - 1 }"
            :aria-label="t('auth.signIn.onboarding.goToStep', { step })"
            :aria-current="currentStep === step - 1 ? 'step' : undefined"
            @click="goTo(step - 1)"
          />
        </div>

        <Button
          v-if="!isFinalStep"
          class="justify-self-end"
          @click="goTo(currentStep + 1)"
        >
          <span class="hidden sm:inline">{{ t('auth.signIn.onboarding.next') }}</span>
          <ArrowRight
            :size="16"
            aria-hidden="true"
          />
        </Button>
        <span v-else />
      </footer>
    </section>
  </main>
</template>

<style scoped>
.onboarding-page {
  background:
    linear-gradient(125deg,
      color-mix(in oklch, var(--background) 92%, var(--accent)) 0%,
      var(--background) 32%,
      color-mix(in oklch, var(--background) 90%, oklch(0.68 0.07 180)) 62%,
      color-mix(in oklch, var(--background) 89%, var(--accent)) 100%);
  background-size: 260% 260%;
  animation: onboarding-gradient 16s var(--ease-out-expo) infinite alternate;
}

.onboarding-page::before {
  position: absolute;
  inset: -35% -15% -15%;
  z-index: -1;
  content: '';
  background-image:
    linear-gradient(color-mix(in oklch, var(--border) 52%, transparent) 1px, transparent 1px),
    linear-gradient(90deg, color-mix(in oklch, var(--border) 52%, transparent) 1px, transparent 1px);
  background-size: 2.4rem 2.4rem;
  mask-image: radial-gradient(ellipse at center, black 12%, transparent 69%);
  transform: perspective(52rem) rotateX(57deg) scale(1.2);
  transform-origin: center bottom;
  opacity: 0.62;
}

.onboarding-page::after {
  position: absolute;
  inset: 0;
  z-index: -2;
  content: '';
  background: radial-gradient(circle at 50% 45%, transparent 10%, color-mix(in oklch, var(--background) 48%, transparent) 86%);
}

.onboarding-shell,
.sign-in-form {
  background: color-mix(in oklch, var(--surface) 84%, transparent);
  backdrop-filter: blur(1.4rem) saturate(125%);
  -webkit-backdrop-filter: blur(1.4rem) saturate(125%);
}

.onboarding-brand-mark {
  position: relative;
  width: 1.5rem;
  height: 1.5rem;
  border: 1px solid color-mix(in oklch, var(--accent) 65%, var(--border));
  border-radius: 0.5rem;
  background: color-mix(in oklch, var(--accent) 13%, var(--surface));
}

.onboarding-brand-mark::before,
.onboarding-brand-mark span {
  position: absolute;
  content: '';
  border-radius: 999px;
  background: var(--accent);
}

.onboarding-brand-mark::before { inset: 0.32rem 0.62rem; }
.onboarding-brand-mark span { inset: 0.62rem 0.32rem; opacity: 0.58; }

.onboarding-visual {
  position: relative;
  min-height: 19rem;
  overflow: hidden;
  border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
  border-radius: 1.5rem;
  padding: clamp(1.5rem, 5vw, 3.5rem);
  background:
    linear-gradient(145deg, color-mix(in oklch, var(--surface-raised) 78%, transparent), color-mix(in oklch, var(--accent) 8%, transparent));
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 80%, transparent), var(--shadow-dialog);
}

.visual-context,
.visual-live,
.visual-keyboard {
  display: grid;
  height: 100%;
  min-height: 14rem;
  align-content: center;
  gap: 0.8rem;
}

.visual-title-line { width: 42%; height: 0.7rem; margin-bottom: 0.55rem; border-radius: 999px; background: var(--foreground); opacity: 0.82; }
.visual-profile-row { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 0.85rem; border-top: 1px solid var(--border); padding-block: 0.85rem; color: var(--accent); }
.visual-profile-icon { width: 2rem; height: 2rem; border-radius: 0.65rem; background: color-mix(in oklch, var(--accent) 16%, var(--surface-raised)); }
.visual-profile-row > span:nth-child(2) { display: grid; gap: 0.42rem; }
.visual-profile-row i { display: block; width: 65%; height: 0.42rem; border-radius: 999px; background: var(--foreground); opacity: 0.74; }
.visual-profile-row i + i { width: 42%; opacity: 0.24; }

.visual-window-bar { display: flex; gap: 0.35rem; border-bottom: 1px solid var(--border); padding-bottom: 0.8rem; }
.visual-window-bar i { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: var(--muted-foreground); opacity: 0.45; }
.visual-wave { display: flex; height: 4.5rem; align-items: center; gap: 0.22rem; }
.visual-wave i { width: 100%; height: var(--bar); min-height: 0.25rem; border-radius: 999px; background: var(--accent); opacity: 0.72; }
.visual-question { display: grid; grid-template-columns: auto 1fr; align-items: center; gap: 0.65rem; }
.visual-question i { width: 1.75rem; height: 1.75rem; border-radius: 50%; background: var(--muted); }
.visual-question span, .visual-answer i { display: block; height: 0.45rem; border-radius: 999px; background: var(--foreground); opacity: 0.72; }
.visual-answer { display: grid; gap: 0.55rem; border-radius: 0.8rem; background: color-mix(in oklch, var(--accent) 10%, transparent); padding: 1rem; }
.visual-answer i:nth-child(2) { width: 86%; opacity: 0.46; }
.visual-answer i:nth-child(3) { width: 58%; opacity: 0.24; }

.visual-keyboard { justify-items: center; }
.visual-key-row { display: flex; justify-content: center; gap: 0.6rem; }
.visual-key-row kbd { min-width: 3rem; border: 1px solid var(--border); border-radius: 0.7rem; padding: 0.8rem; background: var(--surface-raised); box-shadow: 0 0.25rem 0 color-mix(in oklch, var(--border) 78%, transparent); text-align: center; font: inherit; font-weight: 600; }
.visual-key-row--small kbd { min-width: 3.4rem; padding: 0.55rem; color: var(--muted-foreground); font-size: 0.75rem; }
.visual-response-lines { display: grid; width: min(100%, 23rem); gap: 0.55rem; border-block: 1px solid var(--border); padding-block: 1.4rem; }
.visual-response-lines i { height: 0.45rem; border-radius: 999px; background: var(--foreground); opacity: 0.66; }
.visual-response-lines i:nth-child(2) { width: 88%; opacity: 0.5; }
.visual-response-lines i:nth-child(3) { width: 93%; opacity: 0.38; }
.visual-response-lines i:nth-child(4) { width: 54%; opacity: 0.24; }

.onboarding-dot {
  width: 0.5rem;
  height: 0.5rem;
  border: 0;
  border-radius: 999px;
  padding: 0;
  background: var(--muted-foreground);
  opacity: 0.35;
}

.onboarding-dot--active {
  width: 1.5rem;
  background: var(--accent);
  opacity: 1;
}

.onboarding-step-enter-active,
.onboarding-step-leave-active { transition: opacity var(--motion-medium) var(--ease-out-expo), transform var(--motion-medium) var(--ease-out-expo); }
.onboarding-step-enter-from { opacity: 0; transform: translateX(0.75rem); }
.onboarding-step-leave-to { opacity: 0; transform: translateX(-0.5rem); }

@keyframes onboarding-gradient {
  0% { background-position: 0% 20%; }
  50% { background-position: 100% 60%; }
  100% { background-position: 25% 100%; }
}

@media (prefers-reduced-transparency: reduce) {
  .onboarding-shell,
  .sign-in-form { background: var(--surface); backdrop-filter: none; -webkit-backdrop-filter: none; }
}

@media (prefers-reduced-motion: reduce) {
  .onboarding-page { animation: none; background-position: 50% 50%; }
  .onboarding-step-enter-from,
  .onboarding-step-leave-to { transform: none; }
}
</style>
