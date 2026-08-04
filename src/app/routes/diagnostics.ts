import type { RouteRecordRaw } from 'vue-router'

export const restrictedDiagnosticRoute: RouteRecordRaw = {
  path: '/_diagnostics',
  name: 'restricted-diagnostics',
  component: () => import('@/pages/PlaceholderPage.vue'),
  beforeEnter: () => ({ path: '/profiles' }),
  meta: {
    restricted: true,
    navigation: false,
  },
}
