import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

const mockChangeLanguage = vi.fn().mockResolvedValue(undefined);
const mockToggleTheme = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'settings.title': 'Settings',
        'settings.languageLabel': 'Language',
        'settings.languageDescription': 'Choose your preferred language',
        'settings.languageEnglish': 'English',
        'settings.languageSpanish': 'Español',
      };
      return translations[key] ?? key;
    },
    i18n: {
      language: 'en',
      changeLanguage: mockChangeLanguage,
    },
  }),
}));

vi.mock('../hooks/useTheme', () => ({
  useTheme: () => ({
    theme: 'dark',
    toggleTheme: mockToggleTheme,
  }),
}));

import Settings from '../pages/Settings';

describe('Settings', () => {
  test('renders settings title', () => {
    render(<Settings />);

    expect(screen.getByRole('heading', { name: 'Settings', level: 1 })).toBeTruthy();
  });

  test('renders language section with select', () => {
    render(<Settings />);

    expect(screen.getByRole('heading', { name: 'Language', level: 2 })).toBeTruthy();
    expect(screen.getByRole('combobox', { name: 'Language' })).toBeTruthy();
  });

  test('renders language options', () => {
    render(<Settings />);

    expect(screen.getByRole('option', { name: 'English' })).toBeTruthy();
    expect(screen.getByRole('option', { name: 'Español' })).toBeTruthy();
  });

  test('changes language on select change', () => {
    render(<Settings />);

    const select = screen.getByRole('combobox', { name: 'Language' });
    fireEvent.change(select, { target: { value: 'es' } });

    expect(mockChangeLanguage).toHaveBeenCalledWith('es');
  });

  test('renders appearance section with theme toggle', () => {
    render(<Settings />);

    expect(screen.getByRole('heading', { name: 'Appearance', level: 2 })).toBeTruthy();
    expect(screen.getByRole('button', { name: /switch to light mode/i })).toBeTruthy();
  });

  test('theme toggle button calls toggleTheme on click', () => {
    render(<Settings />);

    const toggleButton = screen.getByRole('button', { name: /switch to light mode/i });
    fireEvent.click(toggleButton);

    expect(mockToggleTheme).toHaveBeenCalled();
  });

  test('theme toggle shows pressed state for dark mode', () => {
    render(<Settings />);

    const toggleButton = screen.getByRole('button', { name: /switch to light mode/i });
    expect(toggleButton.getAttribute('aria-pressed')).toBe('true');
  });

  test('renders notifications section', () => {
    render(<Settings />);

    expect(screen.getByRole('heading', { name: 'Notifications', level: 2 })).toBeTruthy();
  });

  test('all sections are properly labelled', () => {
    render(<Settings />);

    expect(screen.getByRole('region', { name: 'Language' })).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Appearance' })).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Notifications' })).toBeTruthy();
  });

  test('language select is enabled by default', () => {
    render(<Settings />);

    const select = screen.getByRole('combobox', { name: 'Language' });
    expect((select as HTMLSelectElement).disabled).toBe(false);
  });
});
