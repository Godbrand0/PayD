import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { EmployeeList } from '../EmployeeList';

vi.mock('../Avatar', () => ({
  Avatar: ({ name }: { name: string }) => <div data-testid="avatar">{name}</div>,
}));

vi.mock('../AvatarUpload', () => ({
  AvatarUpload: () => null,
}));

vi.mock('../../hooks/useNotification', () => ({
  useNotification: () => ({
    notifySuccess: vi.fn(),
  }),
}));

vi.mock('../CSVUploader', () => ({
  CSVUploader: () => null,
}));

vi.mock('../EmployeeRemovalConfirmModal', () => ({
  EmployeeRemovalConfirmModal: () => null,
}));

const employee = {
  id: 'emp-1',
  name: 'Alexandria Catherine Johnson-Smith With A Very Long Name',
  email: 'alexandria.catherine.johnson-smith.with.a.very.long.email@example.com',
  position: 'Senior Finance Operations Specialist',
  wallet: 'GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ12',
  salary: 1500,
  status: 'Active' as const,
};

describe('EmployeeList', () => {
  test('applies employee search after debounce delay', () => {
    vi.useFakeTimers();
    const secondEmployee = {
      ...employee,
      id: 'emp-2',
      name: 'Bob Martin',
      email: 'bob@example.com',
    };

    render(<EmployeeList employees={[employee, secondEmployee]} onAddEmployee={vi.fn()} />);

    const searchInput = screen.getByLabelText('Search employees');
    fireEvent.change(searchInput, { target: { value: 'Bob' } });

    expect(screen.getByLabelText(`Employee name: ${employee.name}`)).toBeTruthy();
    act(() => {
      vi.advanceTimersByTime(350);
    });
    expect(screen.queryByLabelText(`Employee name: ${employee.name}`)).toBeNull();
    expect(screen.getByLabelText(`Employee name: ${secondEmployee.name}`)).toBeTruthy();

    vi.useRealTimers();
  });

  test('renders employee name and email with truncation metadata for narrow columns', () => {
    render(<EmployeeList employees={[employee]} onAddEmployee={vi.fn()} />);

    const name = screen.getByLabelText(`Employee name: ${employee.name}`);
    const email = screen.getByLabelText(`Employee email: ${employee.email}`);

    expect(name).toHaveAttribute('title', employee.name);
    expect(name.className).toContain('truncate');
    expect(email).toHaveAttribute('title', employee.email);
    expect(email.className).toContain('truncate');
  });

  test('renders skeleton rows and hides employee data while loading', () => {
    render(<EmployeeList employees={[employee]} isLoading onAddEmployee={vi.fn()} />);

    // Employee data must not be visible during loading
    expect(screen.queryByLabelText(`Employee name: ${employee.name}`)).toBeNull();
    expect(screen.queryByLabelText(`Employee email: ${employee.email}`)).toBeNull();

    // Skeleton rows are rendered with pulse animation
    const rows = document.querySelectorAll('tbody tr');
    expect(rows.length).toBe(5);
    rows.forEach((row) => {
      expect(row.className).toContain('animate-pulse');
    });
  });

  test('renders empty state message when not loading and no employees exist', () => {
    render(<EmployeeList employees={[]} isLoading={false} onAddEmployee={vi.fn()} />);
    expect(screen.getByText('No employees found')).toBeTruthy();
  });
});
