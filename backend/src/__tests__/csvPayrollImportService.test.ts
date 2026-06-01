import { CsvPayrollImportService } from '../services/csvPayrollImportService.js';

describe('CsvPayrollImportService', () => {
  const service = new CsvPayrollImportService();

  it('normalizes headers, validates rows, and reports duplicate emails before insert', async () => {
    const csv = [
      ' first_name , last_name , email , base_salary , base_currency , notes ',
      ' Ada , Lovelace , ada@example.com , 1000 , USDC , unsupported ',
      ' Ada , Byron , ADA@example.com , 1200 , USDC , unsupported ',
    ].join('\n');

    const result = await service.processCsv(1, csv);

    expect(result.totalRows).toBe(2);
    expect(result.successCount).toBe(0);
    expect(result.errorCount).toBe(2);
    expect(result.errors[1]?.row).toBe(3);
    expect(result.errors[1]?.email).toBe('ada@example.com');
    expect(result.errors[1]?.errors.join(' ')).toContain('Duplicate email');
  });

  it('returns row-level validation errors for malformed payroll fields', async () => {
    const csv = [
      'first_name,last_name,email,wallet_address,base_salary,base_currency,notes',
      'Grace,Hopper,grace@example.com,not-a-stellar-key,-1,XYZ,unsupported',
    ].join('\n');

    const result = await service.processCsv(1, csv);

    expect(result.totalRows).toBe(1);
    expect(result.successCount).toBe(0);
    expect(result.errorCount).toBe(1);
    expect(result.errors[0]?.errors).toEqual(
      expect.arrayContaining([
        'Unsupported columns: notes',
        'Invalid Stellar wallet address',
        'Salary cannot be negative',
        "Unsupported base_currency 'XYZ'. Allowed: USDC, USD, EUR, GBP, KES, NGN",
      ])
    );
  });
});
