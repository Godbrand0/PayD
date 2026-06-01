import { jest } from '@jest/globals';
import {
  clearFxRateMemoryCache,
  convertOrgUsdAmount,
  getOrgUsdRates,
} from '../services/fxRateService.js';

describe('fxRateService', () => {
  beforeEach(() => {
    clearFxRateMemoryCache();
    jest.restoreAllMocks();
  });

  it('fetches, normalizes, and caches ORGUSD fiat rates', async () => {
    const fetchMock = jest.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: async () => ({
        result: 'success',
        rates: {
          usd: 1,
          kes: 130.25,
        },
      }),
    } as unknown as Response);

    const first = await getOrgUsdRates();
    const second = await getOrgUsdRates();

    expect(first.base).toBe('ORGUSD');
    expect(first.rates.USD).toBe(1);
    expect(first.rates.ORGUSD).toBe(1);
    expect(first.rates.KES).toBe(130.25);
    expect(second).toEqual(first);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('converts between any supported cached currencies', async () => {
    jest.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: async () => ({
        result: 'success',
        rates: {
          EUR: 0.8,
          KES: 128,
        },
      }),
    } as unknown as Response);

    const result = await convertOrgUsdAmount(10, 'EUR', 'KES');

    expect(result.from).toBe('EUR');
    expect(result.to).toBe('KES');
    expect(result.rate).toBe(160);
    expect(result.convertedAmount).toBe(1600);
  });
});
