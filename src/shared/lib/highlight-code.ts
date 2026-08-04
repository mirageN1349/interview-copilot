type ShikiModule = Pick<typeof import('shiki/bundle/web'), 'codeToHtml'>

export type CodeTheme = 'github-dark' | 'github-light'

export type HighlightResult =
  | { kind: 'highlighted'; html: string }
  | { kind: 'plain'; text: string }

const languages = {
  bash: 'bash',
  c: 'c',
  cpp: 'cpp',
  'c++': 'cpp',
  cs: 'csharp',
  csharp: 'csharp',
  css: 'css',
  go: 'go',
  html: 'html',
  java: 'java',
  js: 'javascript',
  javascript: 'javascript',
  json: 'json',
  jsx: 'jsx',
  py: 'python',
  python: 'python',
  rs: 'rust',
  rust: 'rust',
  sh: 'bash',
  shell: 'bash',
  sql: 'sql',
  ts: 'typescript',
  tsx: 'tsx',
  typescript: 'typescript',
  vue: 'vue',
  zsh: 'bash',
} as const

type HighlightOptions = {
  theme?: CodeTheme
  loadShiki?: () => Promise<ShikiModule>
}

const allowedTags = new Set(['PRE', 'CODE', 'SPAN'])
const allowedStyles = new Set(['background-color', 'color', 'font-style', 'font-weight', 'text-decoration'])

export function sanitizeShikiHtml(html: string): string {
  const document = new DOMParser().parseFromString(html, 'text/html')

  for (const element of [...document.body.querySelectorAll('*')]) {
    if (!allowedTags.has(element.tagName)) {
      element.replaceWith(document.createTextNode(element.textContent ?? ''))
      continue
    }

    for (const attribute of [...element.attributes]) {
      if (attribute.name !== 'class' && attribute.name !== 'style' && attribute.name !== 'tabindex') {
        element.removeAttribute(attribute.name)
      }
    }

    const styleProperties = Array.from(
      { length: element.style.length },
      (_, index) => element.style.item(index),
    )
    for (const property of styleProperties) {
      const value = element.style.getPropertyValue(property)
      if (!allowedStyles.has(property) || /url\s*\(|expression\s*\(|@import|javascript:/i.test(value)) {
        element.style.removeProperty(property)
      }
    }

    if (element.hasAttribute('tabindex') && !/^-?\d+$/.test(element.getAttribute('tabindex') ?? '')) {
      element.removeAttribute('tabindex')
    }
  }

  return document.body.innerHTML
}

export async function highlightCode(
  code: string,
  language: string,
  options: HighlightOptions = {},
): Promise<HighlightResult> {
  const normalizedLanguage = languages[language.trim().toLowerCase() as keyof typeof languages]
  if (!normalizedLanguage) return { kind: 'plain', text: code }

  try {
    const shiki = await (options.loadShiki?.() ?? import('shiki/bundle/web'))
    const html = await shiki.codeToHtml(code, {
      lang: normalizedLanguage,
      theme: options.theme ?? 'github-dark',
    })
    return { kind: 'highlighted', html: sanitizeShikiHtml(html) }
  } catch {
    return { kind: 'plain', text: code }
  }
}
