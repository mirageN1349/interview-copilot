import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

import { highlightCode, sanitizeShikiHtml } from '@/shared/lib/highlight-code'
import CodeBlock from '@/shared/ui/code/CodeBlock.vue'

describe('safe code rendering', () => {
  it('keeps unknown languages as inert plain text without loading Shiki', async () => {
    const loadShiki = vi.fn()
    const code = '<img src=x onerror="globalThis.pwned = true"><script>globalThis.pwned = true</script>'

    await expect(highlightCode(code, 'not-a-language', { loadShiki })).resolves.toEqual({
      kind: 'plain',
      text: code,
    })
    expect(loadShiki).not.toHaveBeenCalled()

    const wrapper = mount(CodeBlock, { props: { code, language: 'not-a-language' } })
    await flushPromises()

    expect(wrapper.find('script').exists()).toBe(false)
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.text()).toContain(code)
  })

  it('highlights only allowlisted languages through lazily loaded Shiki', async () => {
    const codeToHtml = vi.fn().mockResolvedValue(
      '<pre class="shiki" style="background-color:#fff;color:#111"><code><span style="color:#111">const answer = 42</span></code></pre>',
    )
    const loadShiki = vi.fn().mockResolvedValue({ codeToHtml })

    await expect(highlightCode('const answer = 42', 'ts', { loadShiki })).resolves.toMatchObject({
      kind: 'highlighted',
    })
    expect(loadShiki).toHaveBeenCalledOnce()
    expect(codeToHtml).toHaveBeenCalledWith('const answer = 42', {
      lang: 'typescript',
      theme: 'github-dark',
    })
  })

  it('renders an allowlisted language with the installed Shiki bundle', async () => {
    const result = await highlightCode('const answer: number = 42', 'typescript')

    expect(result.kind).toBe('highlighted')
    if (result.kind === 'highlighted') {
      expect(result.html).toMatch(/<pre[^>]*class="shiki(?:\s[^"]*)?"/)
      expect(result.html).toContain('answer')
    }
  })

  it('removes executable markup and unsafe CSS from Shiki-shaped HTML', () => {
    const sanitized = sanitizeShikiHtml(
      '<pre class="shiki" onclick="alert(1)" style="background-color:#111;background-image:url(https://bad.test)"><code><span style="color:#fff" onmouseover="alert(2)">safe</span><img src=x onerror="alert(3)"></code></pre>',
    )

    expect(sanitized).toContain('safe')
    expect(sanitized).not.toMatch(/onclick|onmouseover|onerror|<img|background-image|url\(/i)
  })
})
