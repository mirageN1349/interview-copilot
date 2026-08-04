<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import Button from '@/shared/ui/button/Button.vue'
import Dialog from '@/shared/ui/dialog/Dialog.vue'
import Input from '@/shared/ui/input/Input.vue'

const props = defineProps<{ open: boolean; pending?: boolean; error?: string }>()
const emit = defineEmits<{ 'update:open': [value: boolean]; create: [name: string] }>()
const { t } = useI18n()
const name = ref('')

watch(() => props.open, (open) => {
  if (open) name.value = ''
})

function submit() {
  const value = name.value.trim()
  if (value) emit('create', value)
}
</script>

<template>
  <Dialog
    :open="open"
    :title="t('profiles.create.title')"
    :description="t('profiles.create.description')"
    @update:open="emit('update:open', $event)"
  >
    <form
      class="grid gap-2"
      @submit.prevent="submit"
    >
      <label
        class="text-sm font-medium"
        for="profile-name"
      >{{ t('profiles.fields.name') }}</label>
      <Input
        id="profile-name"
        v-model="name"
        autofocus
        maxlength="100"
        :placeholder="t('profiles.create.placeholder')"
      />
      <p
        v-if="error"
        role="alert"
        class="rounded-(--radius-sm) bg-[color-mix(in_oklch,var(--danger)_9%,transparent)] px-3 py-2 text-sm text-(--danger)"
      >
        {{ error }}
      </p>
    </form>
    <template #footer>
      <Button
        variant="secondary"
        @click="emit('update:open', false)"
      >
        {{ t('common.cancel') }}
      </Button>
      <Button
        :disabled="pending || !name.trim()"
        @click="submit"
      >
        {{ pending ? t('common.saving') : t('profiles.create.action') }}
      </Button>
    </template>
  </Dialog>
</template>
