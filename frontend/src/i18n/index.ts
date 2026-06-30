// frontend/src/i18n/index.ts
import en, { type TranslationKeys } from './locales/en';
import zhCN from './locales/zh-CN';
import zhTW from './locales/zh-TW';
import ja from './locales/ja';

export type Lang = 'en' | 'zh-CN' | 'zh-TW' | 'ja';

export const LANGS: { value: Lang; label: string }[] = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'zh-TW', label: '繁體中文' },
  { value: 'ja',    label: '日本語' },
  { value: 'en',    label: 'English' },
];

export const LANG_TO_LOCALE: Record<Lang, string> = {
  'zh-CN': 'zh-CN',
  'zh-TW': 'zh-TW',
  'ja':    'ja-JP',
  'en':    'en-US',
};

export const translations: Record<Lang, Record<TranslationKeys, string>> = {
  en,
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  ja,
};

/** 从 navigator.language 推断语言，fallback 到 'en' */
export function detectLang(): Lang {
  const nav = typeof navigator !== 'undefined' ? navigator.language : '';
  if (!nav) return 'en';
  if (nav === 'zh' || nav.startsWith('zh-CN') || nav.startsWith('zh-SG')) return 'zh-CN';
  if (nav.startsWith('zh-TW') || nav.startsWith('zh-HK') || nav.startsWith('zh-MO')) return 'zh-TW';
  if (nav.startsWith('ja')) return 'ja';
  if (nav.startsWith('en')) return 'en';
  return 'en';
}

export type { TranslationKeys };
