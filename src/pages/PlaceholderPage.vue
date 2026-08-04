<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const title = computed(() => String(route.meta.title ?? 'Interview Copilot'))
</script>

<template>
  <main class="page-shell">
    <h1>{{ title }}</h1>
  </main>
</template>
