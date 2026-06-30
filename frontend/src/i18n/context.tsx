// frontend/src/i18n/context.tsx
import { createContext, useContext, useState, useEffect, useCallback, useMemo } from 'react';
import type { Lang, TranslationKeys } from './index';
import { detectLang, translations, LANG_TO_LOCALE } from './index';

interface LangContextValue {
  lang: Lang;
  t: (key: TranslationKeys, params?: Record<string, string | number>) => string;
}

const LangContext = createContext<LangContextValue>({
  lang: 'en',
  t: (key) => key,
});

export function LangProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLang] = useState<Lang>(() => {
    // 优先使用 Rust 注入的初始值，否则从系统语言检测
    const injected = (window as any).__INITIAL_LANG__ as Lang | undefined;
    const valid = Object.keys(LANG_TO_LOCALE) as Lang[];
    if (injected && valid.includes(injected)) return injected;
    return detectLang();
  });

  useEffect(() => {
    const isTauri = '__TAURI__' in window;
    if (!isTauri) return;

    let active = true;

    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<string>('language-changed', (event) => {
        const valid = Object.keys(LANG_TO_LOCALE) as Lang[];
        const newLang = event.payload as Lang;
        if (active && valid.includes(newLang)) {
          setLang(newLang);
        }
      })
    ).then((fn) => {
      if (!active) fn();
      else unlisten = fn;
    });

    let unlisten: (() => void) | undefined;

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const t = useCallback((key: TranslationKeys, params?: Record<string, string | number>): string => {
    const locale = translations[lang];
    let str: string = locale[key] ?? key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        str = str.replace(`{${k}}`, String(v));
      }
    }
    return str;
  }, [lang]);

  const value = useMemo(() => ({ lang, t }), [lang, t]);

  return (
    <LangContext.Provider value={value}>
      {children}
    </LangContext.Provider>
  );
}

export function useTranslation() {
  return useContext(LangContext);
}
