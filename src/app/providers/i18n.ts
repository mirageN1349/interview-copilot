import { createI18n } from 'vue-i18n'

import { en } from '@/shared/config/i18n/en'
import { ru } from '@/shared/config/i18n/ru'

const savedLocale = localStorage.getItem('locale')
const browserLocale = navigator.language.toLowerCase().startsWith('ru') ? 'ru' : 'en'
const initialLocale = savedLocale === 'ru' || savedLocale === 'en' ? savedLocale : browserLocale
document.documentElement.lang = initialLocale

export const i18n = createI18n({
  legacy: false,
  locale: initialLocale,
  fallbackLocale: 'en',
  messages: { en, ru },
  datetimeFormats: {
    en: { short: { year: 'numeric', month: 'short', day: 'numeric' } },
    ru: { short: { year: 'numeric', month: 'short', day: 'numeric' } },
  },
  numberFormats: {
    en: { percent: { style: 'percent', maximumFractionDigits: 0 } },
    ru: { percent: { style: 'percent', maximumFractionDigits: 0 } },
  },
})
