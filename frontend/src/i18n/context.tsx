// frontend/src/i18n/context.tsx
import { createContext, useContext, useState, useEffect } from 'react';
import type { Lang, TranslationKeys } from './index';
import { detectLang, translations } from './index';

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
    const valid: Lang[] = ['en', 'zh-CN', 'zh-TW', 'ja'];
    if (injected && valid.includes(injected)) return injected;
    return detectLang();
  });

  useEffect(() => {
    const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
    if (!isTauri) return;

    let unlisten: (() => void) | undefined;

    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<string>('language-changed', (event) => {
        const valid: Lang[] = ['en', 'zh-CN', 'zh-TW', 'ja'];
        const newLang = event.payload as Lang;
        if (valid.includes(newLang)) {
          setLang(newLang);
        }
      }).then((fn) => {
        unlisten = fn;
      });
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const t = (key: TranslationKeys, params?: Record<string, string | number>): string => {
    const locale = translations[lang];
    let str: string = locale[key] ?? key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        str = str.replace(`{${k}}`, String(v));
      }
    }
    return str;
  };

  return (
    <LangContext.Provider value={{ lang, t }}>
      {children}
    </LangContext.Provider>
  );
}

export function useTranslation() {
  return useContext(LangContext);
}
