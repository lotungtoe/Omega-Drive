import type { ThemeColors, ThemeName } from './themes'
import { themes } from './themes'

export interface ReaderSettings {
  font: string
  fontSize: number
  lineHeight: number
  theme: ThemeName
}

export function injectReaderStyles(settings: ReaderSettings): string {
  const colors: ThemeColors = themes[settings.theme]
  return `
<style>
  :host {
    --reader-bg: ${colors.background};
    --reader-text: ${colors.text};
    --reader-link: ${colors.link};
    --reader-highlight: ${colors.highlight};
    --reader-selection: ${colors.selection};
  }
  body {
    font-family: ${settings.font}, 'Noto Serif', Georgia, serif !important;
    font-size: ${settings.fontSize}px !important;
    line-height: ${settings.lineHeight} !important;
    color: var(--reader-text) !important;
    background: var(--reader-bg) !important;
    text-align: justify;
    hyphens: auto;
    word-wrap: break-word;
    margin: 0;
    padding: 0;
  }
  p { margin-bottom: 1.5em; }
  h1, h2, h3 {
    font-family: ${settings.font}, 'Noto Serif', Georgia, serif !important;
    color: var(--reader-text) !important;
    text-align: left;
  }
  a { color: var(--reader-link) !important; }
  img { max-width: 100%; height: auto; }
  ::selection { background: var(--reader-selection); }
</style>`
}
