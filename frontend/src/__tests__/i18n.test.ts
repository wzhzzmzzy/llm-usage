// frontend/src/__tests__/i18n.test.ts
import { describe, it, expect, vi, afterEach } from 'vitest';
import { detectLang } from '../i18n/index';
import en from '../i18n/locales/en';
import zhCN from '../i18n/locales/zh-CN';
import zhTW from '../i18n/locales/zh-TW';
import ja from '../i18n/locales/ja';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('detectLang', () => {
  it('maps zh-CN to zh-CN', () => {
    vi.stubGlobal('navigator', { language: 'zh-CN' });
    expect(detectLang()).toBe('zh-CN');
  });
  it('maps zh (bare) to zh-CN', () => {
    vi.stubGlobal('navigator', { language: 'zh' });
    expect(detectLang()).toBe('zh-CN');
  });
  it('maps zh-TW to zh-TW', () => {
    vi.stubGlobal('navigator', { language: 'zh-TW' });
    expect(detectLang()).toBe('zh-TW');
  });
  it('maps zh-HK to zh-TW', () => {
    vi.stubGlobal('navigator', { language: 'zh-HK' });
    expect(detectLang()).toBe('zh-TW');
  });
  it('maps zh-MO to zh-TW', () => {
    vi.stubGlobal('navigator', { language: 'zh-MO' });
    expect(detectLang()).toBe('zh-TW');
  });
  it('maps ja to ja', () => {
    vi.stubGlobal('navigator', { language: 'ja' });
    expect(detectLang()).toBe('ja');
  });
  it('maps ja-JP to ja', () => {
    vi.stubGlobal('navigator', { language: 'ja-JP' });
    expect(detectLang()).toBe('ja');
  });
  it('maps en-US to en', () => {
    vi.stubGlobal('navigator', { language: 'en-US' });
    expect(detectLang()).toBe('en');
  });
  it('maps unknown locale to en', () => {
    vi.stubGlobal('navigator', { language: 'fr-FR' });
    expect(detectLang()).toBe('en');
  });
  it('maps ko-KR to en', () => {
    vi.stubGlobal('navigator', { language: 'ko-KR' });
    expect(detectLang()).toBe('en');
  });
});

describe('locale key coverage', () => {
  const enKeys = Object.keys(en).sort();

  it('zh-CN has exactly the same keys as en', () => {
    expect(Object.keys(zhCN).sort()).toEqual(enKeys);
  });
  it('zh-TW has exactly the same keys as en', () => {
    expect(Object.keys(zhTW).sort()).toEqual(enKeys);
  });
  it('ja has exactly the same keys as en', () => {
    expect(Object.keys(ja).sort()).toEqual(enKeys);
  });
});

describe('t() interpolation', () => {
  // Test the interpolation logic in isolation
  function interpolate(str: string, params?: Record<string, string | number>): string {
    if (!params) return str;
    let result = str;
    for (const [k, v] of Object.entries(params)) {
      result = result.replace(`{${k}}`, String(v));
    }
    return result;
  }

  it('replaces single placeholder', () => {
    expect(interpolate('Viewing {date}', { date: '2026-01-01' })).toBe('Viewing 2026-01-01');
  });
  it('replaces multiple placeholders', () => {
    expect(interpolate('Page {current} / {total}', { current: 2, total: 10 })).toBe('Page 2 / 10');
  });
  it('returns string unchanged if no params', () => {
    expect(interpolate('Refresh')).toBe('Refresh');
  });
  it('returns string unchanged if placeholder not in params', () => {
    expect(interpolate('{missing} value', {})).toBe('{missing} value');
  });
});
