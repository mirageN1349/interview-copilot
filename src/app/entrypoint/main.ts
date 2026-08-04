import { createApp } from 'vue'

import App from '@/App.vue'
import { installProviders } from '@/app/providers'
import { initializeMeetingSocket } from '@/app/providers/meeting-socket'
import { setProductTransport } from '@/entities/interview-profile'
import { setAuthTransport } from '@/shared/api/auth/client'
import { setSubscriptionTransport } from '@/entities/subscription'
import { mockTransportTarget } from '@/mocks/packaged'
import '@/app/styles/index.css'

async function bootstrap(): Promise<void> {
  document.documentElement.classList.toggle('overlay-window', location.hash.startsWith('#/overlay'))
  const mockTarget = mockTransportTarget('__TAURI_INTERNALS__' in window, import.meta.env.DEV)
  if (mockTarget === 'packaged') {
    const [{ createPackagedFetch }, { createScenarioRuntime }, { createInMemoryMeetingSocket }] = await Promise.all([
      import('@/mocks/packaged'),
      import('@/mocks/scenarios/runtime'),
      import('@/mocks/scenarios/ws'),
    ])
    const [authScenario, productScenario, subscriptionScenario] = import.meta.env.DEV
      ? await Promise.all([
          import('@/mocks/scenarios/auth').then(({ authScenario }) => authScenario),
          import('@/mocks/scenarios/product').then(({ createProductScenario }) => createProductScenario()),
          import('@/mocks/scenarios/subscription').then(({ createSubscriptionScenario }) => createSubscriptionScenario()),
        ])
      : await Promise.all([
          import('@/mocks/scenarios/auth').then(({ createAuthScenario }) => createAuthScenario()),
          import('@/mocks/scenarios/product').then(({ createProductScenario }) => createProductScenario()),
          import('@/mocks/scenarios/subscription').then(({ createSubscriptionScenario }) => createSubscriptionScenario()),
        ])
    const fetcher = createPackagedFetch(createScenarioRuntime([
      ...authScenario.definitions,
      ...productScenario.definitions,
      ...subscriptionScenario.definitions,
    ]))
    setAuthTransport(fetcher)
    setProductTransport(fetcher)
    setSubscriptionTransport(fetcher)
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    if (getCurrentWindow().label === 'main') initializeMeetingSocket(() => createInMemoryMeetingSocket())
  } else if (mockTarget === 'browser') {
    const [{ createBrowserMock }, { createScenarioRuntime }, { authScenario }, { productScenario }, { subscriptionScenario }, { meetingWebSocketHandler }] = await Promise.all([
      import('@/mocks/browser'),
      import('@/mocks/scenarios/runtime'),
      import('@/mocks/handlers/auth'),
      import('@/mocks/handlers/product'),
      import('@/mocks/handlers/subscription'),
      import('@/mocks/handlers/ws'),
    ])
    await createBrowserMock(createScenarioRuntime([
      ...authScenario.definitions,
      ...productScenario.definitions,
      ...subscriptionScenario.definitions,
    ]), [meetingWebSocketHandler]).start()
    initializeMeetingSocket()
  }

  const app = createApp(App)
  installProviders(app)
  app.mount('#app')
}

void bootstrap().catch((error: unknown) => {
  console.error(error)
  const root = document.querySelector('#app')
  if (!root) return
  const message = error instanceof Error ? error.stack ?? error.message : String(error)
  root.textContent = import.meta.env.DEV ? `Startup failed\n\n${message}` : 'The application could not start.'
  root.setAttribute('role', 'alert')
  root.setAttribute('style', 'padding: 24px; white-space: pre-wrap; color: #ffb4ab')
})
