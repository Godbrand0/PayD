import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type React from 'react';

const notifySuccessMock = vi.fn();
const notifyMock = vi.fn();
const clearSavedDataMock = vi.fn();

vi.mock('@stellar/design-system', () => ({
  Alert: ({ children, title }: { children: React.ReactNode; title?: string }) => (
    <div role="alert">
      {title ? <strong>{title}</strong> : null}
      {children}
    </div>
  ),
  Button: ({ children, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement>) => (
    <button {...props}>{children}</button>
  ),
  Card: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  Input: (props: React.InputHTMLAttributes<HTMLInputElement> & { fieldSize?: string }) => {
    const sanitizedProps = { ...props };
    delete sanitizedProps.fieldSize;
    return <input {...sanitizedProps} />;
  },
  Select: (
    props: React.SelectHTMLAttributes<HTMLSelectElement> & {
      children: React.ReactNode;
      label?: string;
      note?: string;
      fieldSize?: string;
    }
  ) => {
    const { children, label, note } = props;
    const sanitizedProps = { ...props };
    delete sanitizedProps.fieldSize;
    delete sanitizedProps.children;
    delete sanitizedProps.label;
    delete sanitizedProps.note;

    return (
      <label>
        {label}
        <select {...sanitizedProps}>{children}</select>
        {note ? <span>{note}</span> : null}
      </label>
    );
  },
}));

vi.mock('../components/AutosaveIndicator', () => ({
  AutosaveIndicator: () => <div data-testid="autosave-indicator" />,
}));

vi.mock('../components/FormField', () => ({
  FormField: ({
    children,
    id,
    label,
    error,
    helpText,
  }: {
    children: React.ReactNode;
    id: string;
    label: string;
    error?: string;
    helpText?: string;
  }) => (
    <div>
      <label htmlFor={id}>{label}</label>
      {children}
      {helpText ? <p>{helpText}</p> : null}
      {error ? <span>{error}</span> : null}
    </div>
  ),
}));

vi.mock('../components/HelpLink', () => ({
  HelpLink: () => <span data-testid="help-link" />,
}));

vi.mock('../components/WalletQRCode', () => ({
  WalletQRCode: ({ walletAddress }: { walletAddress: string }) => (
    <div data-testid="wallet-qr">{walletAddress}</div>
  ),
}));

vi.mock('../components/EmployeeList', () => ({
  EmployeeList: ({ employees }: { employees: Array<{ id: string; name: string }> }) => (
    <div data-testid="employee-list">
      {employees.map((employee) => (
        <span key={employee.id}>{employee.name}</span>
      ))}
    </div>
  ),
}));

vi.mock('../hooks/useAutosave', () => ({
  useAutosave: () => ({
    saving: false,
    lastSaved: null,
    loadSavedData: () => null,
    clearSavedData: clearSavedDataMock,
  }),
}));

vi.mock('../hooks/useNotification', () => ({
  useNotification: () => ({
    notifySuccess: notifySuccessMock,
    notify: notifyMock,
  }),
}));

vi.mock('../services/stellar', () => ({
  generateWallet: () => ({
    publicKey: 'GATESTWALLET1234567890123456789012345678901234567890123',
    secretKey: 'SATESTSECRET1234567890123456789012345678901234567890123',
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOptions?: string | Record<string, unknown>) =>
      typeof fallbackOrOptions === 'string' ? fallbackOrOptions : key,
  }),
}));

import EmployeeEntry from '../pages/EmployeeEntry';

describe('EmployeeEntry', () => {
  it('creates an employee, generates a wallet, and returns the employee to the directory', () => {
    render(<EmployeeEntry />);

    fireEvent.click(screen.getByRole('button', { name: /add employee/i }));

    fireEvent.change(screen.getByLabelText('Full Name'), {
      target: { value: 'Jane Smith' },
    });
    fireEvent.change(screen.getByLabelText('Work Email'), {
      target: { value: 'jane.smith@example.com' },
    });
    fireEvent.change(screen.getByLabelText('Role / Team'), {
      target: { value: 'Payroll Analyst' },
    });
    fireEvent.change(screen.getByLabelText('Monthly Salary'), {
      target: { value: '2600' },
    });

    fireEvent.click(screen.getByRole('button', { name: /create employee record/i }));

    expect(notifySuccessMock).toHaveBeenCalledWith(
      'Jane Smith added successfully',
      'A Stellar wallet was generated and is ready to share securely.'
    );
    expect(clearSavedDataMock).toHaveBeenCalled();
    expect(screen.getByTestId('wallet-qr')).toHaveTextContent(
      'GATESTWALLET1234567890123456789012345678901234567890123'
    );

    fireEvent.click(screen.getByRole('button', { name: /view employee directory/i }));

    expect(screen.getByTestId('employee-list')).toHaveTextContent('Jane Smith');
  });
});
