import React, { useEffect, useState } from 'react';
import { AutosaveIndicator } from '../components/AutosaveIndicator';
import { useAutosave } from '../hooks/useAutosave';
import { useTransactionSimulation } from '../hooks/useTransactionSimulation';
import { TransactionSimulationPanel } from '../components/TransactionSimulationPanel';
import { useNotification } from '../hooks/useNotification';
import { useSocket } from '../hooks/useSocket';
import { createClaimableBalanceTransaction, generateWallet } from '../services/stellar';
import { useTranslation } from 'react-i18next';
import { Card, Heading, Text, Button, Input, Select } from '@stellar/design-system';
import { SchedulingWizard } from '../components/SchedulingWizard';
import { CountdownTimer } from '../components/CountdownTimer';
import { BulkPaymentStatusTracker } from '../components/BulkPaymentStatusTracker';
import { TransactionPendingOverlay } from '../components/TransactionPendingOverlay';

import { ContractErrorPanel } from '../components/ContractErrorPanel';
import { parseContractError, type ContractErrorDetail } from '../utils/contractErrorParser';
import { HelpLink } from '../components/HelpLink';

interface PayrollFormState {
  employeeName: string;
  amount: string;
  frequency: 'weekly' | 'monthly';
  startDate: string;
  memo?: string;
}

type SchedulingFrequency = 'weekly' | 'biweekly' | 'monthly';

interface EmployeePreference {
  id: string;
  name: string;
  amount: string;
  currency: string;
}

interface SchedulingConfig {
  frequency: SchedulingFrequency;
  dayOfWeek?: number;
  dayOfMonth?: number;
  timeOfDay: string; // HH:mm
  preferences: EmployeePreference[];
}

function parseTimeOfDay(time: string) {
  const [hhRaw, mmRaw] = time.split(':');
  const hh = Number.parseInt(hhRaw ?? '0', 10);
  const mm = Number.parseInt(mmRaw ?? '0', 10);
  return {
    hours: Number.isFinite(hh) ? hh : 0,
    minutes: Number.isFinite(mm) ? mm : 0,
  };
}

function clampDayOfMonth(year: number, monthIndex: number, desired: number) {
  const lastDay = new Date(year, monthIndex + 1, 0).getDate();
  return Math.max(1, Math.min(desired, lastDay));
}

function computeNextRunDate(config: SchedulingConfig, from: Date = new Date()): Date {
  const { hours, minutes } = parseTimeOfDay(config.timeOfDay);

  if (config.frequency === 'monthly') {
    const desiredDay = config.dayOfMonth || 1;

    const year = from.getFullYear();
    const monthIndex = from.getMonth();

    let candidate = new Date(
      year,
      monthIndex,
      clampDayOfMonth(year, monthIndex, desiredDay),
      hours,
      minutes,
      0,
      0
    );

    if (candidate.getTime() <= from.getTime()) {
      const nextMonthIndex = monthIndex + 1;
      candidate = new Date(
        year,
        nextMonthIndex,
        clampDayOfMonth(year, nextMonthIndex, desiredDay),
        hours,
        minutes,
        0,
        0
      );
    }

    return candidate;
  }

  // weekly / biweekly
  const dayOfWeek = config.dayOfWeek ?? 1; // default Monday
  const diffDays = (dayOfWeek - from.getDay() + 7) % 7;

  const first = new Date(from);
  first.setDate(from.getDate() + diffDays);
  first.setHours(hours, minutes, 0, 0);

  if (diffDays === 0 && first.getTime() <= from.getTime()) {
    first.setDate(first.getDate() + 7);
  }

  return first;
}

const formatDate = (dateString: string) => {
  if (!dateString) return 'N/A';
  const date = new Date(dateString);
  if (isNaN(date.getTime())) return dateString;
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
};

interface PendingClaim {
  id: string;
  employeeName: string;
  amount: string;
  dateScheduled: string;
  claimantPublicKey: string;
  status: string;
}

type OverlayStatus = 'pending' | 'success' | 'error';

// Mock employer secret key for simulation purposes
const MOCK_EMPLOYER_SECRET = 'SD3X5K7G7XV4K5V3M2G5QXH434M3VX6O5P3QVQO3L2PQSQQQQQQQQQQQ';

const initialFormState: PayrollFormState = {
  employeeName: '',
  amount: '',
  frequency: 'monthly',
  startDate: '',
  memo: '',
};

export default function PayrollScheduler() {
  const { t } = useTranslation();
  const { notifySuccess, notify, notifyPaymentSuccess, notifyPaymentFailure, notifyApiError } =
    useNotification();
  const { socket, subscribeToTransaction, unsubscribeFromTransaction } = useSocket();
  const [formData, setFormData] = useState<PayrollFormState>(initialFormState);
  const [isBroadcasting, setIsBroadcasting] = useState(false);
  const [isWizardOpen, setIsWizardOpen] = useState(false);

  const [overlayVisible, setOverlayVisible] = useState(false);
  const [overlayStatus, setOverlayStatus] = useState<OverlayStatus>('pending');
  const [overlayTxHash, setOverlayTxHash] = useState<string | undefined>();
  const [activeSchedule, setActiveSchedule] = useState<SchedulingConfig | null>(null);
  const [nextRunDate, setNextRunDate] = useState<Date | null>(null);
  const [contractError, setContractError] = useState<ContractErrorDetail | null>(null);

  const scheduleStorageKey = 'payd-scheduler-config';

  const [pendingClaims, setPendingClaims] = useState<PendingClaim[]>(() => {
    const saved = localStorage.getItem('pending-claims');
    if (saved) {
      try {
        return JSON.parse(saved) as PendingClaim[];
      } catch {
        return [];
      }
    }
    return [];
  });

  const { saving, lastSaved, loadSavedData } = useAutosave<PayrollFormState>(
    'payroll-scheduler-draft',
    formData
  );

  const {
    simulate,
    resetSimulation,
    isSimulating,
    result: simulationResult,
    error: simulationProcessError,
    isSuccess: simulationPassed,
  } = useTransactionSimulation();

  useEffect(() => {
    const saved = loadSavedData();
    if (saved) {
      setFormData(saved);
      notify('Recovered unsaved payroll draft');
    }
  }, [loadSavedData, notify]);

  // Restore confirmed schedule (persisted locally after wizard confirmation).
  useEffect(() => {
    try {
      const raw = localStorage.getItem(scheduleStorageKey);
      if (!raw) return;
      const parsed = JSON.parse(raw) as SchedulingConfig;

      if (!parsed?.frequency || !parsed?.timeOfDay) return;
      if (!['weekly', 'biweekly', 'monthly'].includes(parsed.frequency)) return;
      if (!Array.isArray(parsed.preferences)) return;

      const next = computeNextRunDate(parsed, new Date());
      setActiveSchedule(parsed);
      setNextRunDate(next);
    } catch {
      // Ignore invalid local storage payloads.
    }
  }, []);

  const handleScheduleComplete = (config: SchedulingConfig) => {
    // SchedulingWizard calls back with the full SchedulingConfig, but the current type in
    // this file is intentionally loose to avoid coupling to the component's internal type.
    setActiveSchedule(config);
    setIsWizardOpen(false);
    notifySuccess(
      'Payroll schedule configured!',
      `Frequency: ${config.frequency}, time: ${config.timeOfDay}`
    );

    // Persist config so the countdown survives refresh.
    localStorage.setItem(scheduleStorageKey, JSON.stringify(config));

    setNextRunDate(computeNextRunDate(config, new Date()));
  };

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
    if (simulationResult) {
      resetSimulation();
      setContractError(null);
    }
  };

  useEffect(() => {
    if (!socket) return;

    const handleTransactionUpdate = (data: { transactionId: string; status: string }) => {
      console.log('Received transaction update:', data);
      setPendingClaims((prev) =>
        prev.map((claim) =>
          claim.id === data.transactionId ? { ...claim, status: data.status } : claim
        )
      );

      if (data.status === 'confirmed') {
        notifyPaymentSuccess(data.transactionId, 'Payment confirmed!');
      }
    };

    socket.on('transaction:update', handleTransactionUpdate);

    return () => {
      socket.off('transaction:update', handleTransactionUpdate);
    };
  }, [socket, notifyPaymentSuccess]);

  const handleInitialize = async () => {
    if (!formData.employeeName || !formData.amount) {
      setContractError({
        code: 'MISSING_FIELDS',
        message: 'Missing required fields',
        suggestedAction: 'Please provide employee name and amount.',
      });
      return;
    }

    setContractError(null);

    // Mock XDR for simulation demonstration
    const mockXdr =
      'AAAAAgAAAABmF8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAA';

    const result = await simulate({ envelopeXdr: mockXdr });
    if (result && !result.success) {
      const parsed = parseContractError(result.envelopeXdr, result.description);
      setContractError(parsed);
    }
  };

  const handleBroadcast = async () => {
    setIsBroadcasting(true);
    setContractError(null);
    setOverlayVisible(true);
    setOverlayStatus('pending');
    setOverlayTxHash(undefined);

    try {
      const mockRecipientPublicKey = generateWallet().publicKey;

      // Integrate claimable balance logic from Issue #44
      const result = createClaimableBalanceTransaction(
        MOCK_EMPLOYER_SECRET,
        mockRecipientPublicKey,
        String(formData.amount),
        'USDC'
      );

      if (!result.success) {
        throw new Error('Failed to create claimable balance');
      }

      // Simulate a brief delay for network broadcast
      await new Promise((resolve) => setTimeout(resolve, 1500));

      // Add to pending claims
      const newClaim: PendingClaim = {
        id: Math.random().toString(36).substr(2, 9),
        employeeName: formData.employeeName,
        amount: formData.amount,
        dateScheduled: formData.startDate || new Date().toISOString().split('T')[0],
        claimantPublicKey: mockRecipientPublicKey,
        status: 'Pending Claim',
      };

      const updatedClaims = [...pendingClaims, newClaim];
      setPendingClaims(updatedClaims);
      localStorage.setItem('pending-claims', JSON.stringify(updatedClaims));

      // Subscribe to updates for this new claim
      subscribeToTransaction(newClaim.id);

      // Show success overlay
      setOverlayStatus('success');
      const mockTxHash = `broadcast_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
      setOverlayTxHash(mockTxHash);

      // Auto-dismiss after 3 seconds
      setTimeout(() => {
        setOverlayVisible(false);
      }, 3000);

      notifySuccess(
        'Broadcast successful!',
        `Claimable balance created for ${formData.employeeName}`
      );

      // Trigger Webhook Event (Internal simulation)
      try {
        await fetch('http://localhost:3001/api/webhooks/test-trigger', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            event: 'payment.completed',
            payload: {
              id: newClaim.id,
              employeeName: newClaim.employeeName,
              amount: newClaim.amount,
              status: 'created',
            },
          }),
        });
      } catch {
        notifyApiError(
          'Webhook trigger failed',
          'Payment was created, but webhook test trigger failed.'
        );
        console.warn('Webhook test-trigger skipped (Backend might not be running)');
      }

      resetSimulation();
      setFormData(initialFormState);
    } catch (err) {
      console.error(err);
      const parsed = parseContractError(
        undefined,
        err instanceof Error ? err.message : 'Broadcast failed'
      );
      setContractError(parsed);
      notifyPaymentFailure(parsed.message);
      setOverlayStatus('error');
    } finally {
      setIsBroadcasting(false);
    }
  };

  const handleRemoveClaim = (id: string) => {
    unsubscribeFromTransaction(id);
    const updatedClaims = pendingClaims.filter((c) => c.id !== id);
    setPendingClaims(updatedClaims);
    localStorage.setItem('pending-claims', JSON.stringify(updatedClaims));
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-start p-12 max-w-6xl mx-auto w-full">
      <div className="w-full mb-12 flex items-end justify-between border-b border-hi pb-8">
        <div>
          <Heading
            as="h1"
            size="lg"
            weight="bold"
            addlClassName="mb-2 tracking-tight flex items-center gap-3"
          >
            {t('payroll.title', 'Workforce')}{' '}
            <span className="text-accent">{t('payroll.titleHighlight', 'Scheduler')}</span>
            <HelpLink topic="schedule payroll" variant="icon" size="sm" />
          </Heading>
          <Text
            as="p"
            size="sm"
            weight="regular"
            addlClassName="text-muted font-mono tracking-wider uppercase"
          >
            {t('payroll.subtitle', 'Automated distribution engine')}
          </Text>
        </div>
        <div className="flex flex-col items-end gap-2">
          <AutosaveIndicator saving={saving} lastSaved={lastSaved} />
          <button onClick={() => setIsWizardOpen(true)}>
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </svg>
          </button>
        </div>
      </div>

      {activeSchedule && (
        <div className="w-full mb-12 bg-black/20 border border-success/30 rounded-2xl p-6 flex flex-col md:flex-row items-center justify-between gap-6 relative overflow-hidden">
          <div className="absolute top-0 left-0 w-1 h-full bg-success"></div>
          <div>
            <h3 className="text-success font-black text-lg mb-1 flex items-center gap-2">
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="3"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
              Automation Active
            </h3>
            <p className="text-muted text-sm">
              Scheduled to run{' '}
              <span className="font-bold text-text capitalize">{activeSchedule.frequency}</span> at{' '}
              <span className="font-mono text-text">{activeSchedule.timeOfDay}</span>
            </p>
          </div>
          <div className="bg-bg border border-hi rounded-xl p-4 shadow-inner">
            <span className="block text-[10px] uppercase font-bold text-muted mb-2 tracking-widest text-center">
              Next Scheduled Run
            </span>
            <CountdownTimer targetDate={nextRunDate} />
          </div>
        </div>
      )}

      {isWizardOpen ? (
        <SchedulingWizard
          onComplete={handleScheduleComplete}
          onCancel={() => setIsWizardOpen(false)}
        />
      ) : (
        <div className="w-full grid grid-cols-1 lg:grid-cols-5 gap-8 mb-12">
          <div className="lg:col-span-3">
            <form
              onSubmit={(e: React.FormEvent) => {
                e.preventDefault();
                void handleInitialize();
              }}
              className="w-full grid grid-cols-1 md:grid-cols-2 gap-6 card glass noise"
            >
              <div className="md:col-span-2">
                <Input
                  id="employeeName"
                  fieldSize="md"
                  label={t('payroll.employeeName', 'Employee Name')}
                  name="employeeName"
                  value={formData.employeeName}
                  onChange={handleChange}
                  placeholder="e.g. Satoshi Nakamoto"
                />
              </div>

              <div>
                <Input
                  id="amount"
                  fieldSize="md"
                  label={t('payroll.amountLabel', 'Amount (USD equivalent)')}
                  name="amount"
                  value={formData.amount}
                  onChange={handleChange}
                  placeholder="0.00"
                />
              </div>

              <div>
                <Select
                  id="frequency"
                  fieldSize="md"
                  label={t('payroll.distributionFrequency', 'Distribution Frequency')}
                  name="frequency"
                  value={formData.frequency}
                  onChange={handleChange}
                >
                  <option value="weekly">{t('payroll.frequencyWeekly', 'Weekly')}</option>
                  <option value="monthly">{t('payroll.frequencyMonthly', 'Monthly')}</option>
                </Select>
              </div>

              <div className="md:col-span-2">
                <Input
                  id="startDate"
                  fieldSize="md"
                  label={t('payroll.commencementDate', 'Commencement Date')}
                  name="startDate"
                  type="date"
                  value={formData.startDate}
                  onChange={handleChange}
                />
              </div>

              <div className="md:col-span-2 pt-4">
                {!simulationPassed ? (
                  <Button
                    type="submit"
                    disabled={isSimulating}
                    variant="primary"
                    size="md"
                    isFullWidth
                  >
                    {isSimulating
                      ? 'Simulating...'
                      : t('payroll.submit', 'Initialize and Validate')}
                  </Button>
                ) : (
                  <Button
                    type="button"
                    onClick={() => {
                      void handleBroadcast();
                    }}
                    disabled={isBroadcasting}
                    variant="primary"
                    size="md"
                    isFullWidth
                  >
                    {isBroadcasting ? 'Broadcasting...' : 'Confirm & Broadcast to Network'}
                  </Button>
                )}
              </div>
            </form>
          </div>

          <div className="lg:col-span-2 flex flex-col gap-6">
            <ContractErrorPanel error={contractError} onClear={() => setContractError(null)} />

            <TransactionSimulationPanel
              result={simulationResult}
              isSimulating={isSimulating}
              processError={simulationProcessError}
              onReset={() => {
                resetSimulation();
                setContractError(null);
              }}
            />

            <div className="card glass noise h-fit">
              <Heading as="h3" size="xs" weight="bold" addlClassName="mb-4 flex items-center gap-2">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <circle cx="12" cy="12" r="10" />
                  <line x1="12" y1="16" x2="12" y2="12" />
                  <line x1="12" y1="8" x2="12.01" y2="8" />
                </svg>
                Pre-flight Validation
                <HelpLink topic="transaction simulation" variant="icon" size="sm" />
              </Heading>
              <Text
                as="p"
                size="xs"
                weight="regular"
                addlClassName="text-muted leading-relaxed mb-4"
              >
                All transactions are simulated via Stellar Horizon before submission. This catches
                common errors like:
              </Text>
              <ul className="text-xs text-muted space-y-2 list-disc pl-4 font-medium">
                <li>Insufficient XLM balance for fees</li>
                <li>Invalid sequence numbers</li>
                <li>Missing trustlines for tokens</li>
                <li>Account eligibility status</li>
              </ul>
            </div>
          </div>
        </div>
      )}

      <div className="w-full">
        <Heading as="h2" size="sm" weight="bold" addlClassName="mb-4">
          Pending Claims
        </Heading>
        <Card>
          {pendingClaims.length === 0 ? (
            <Text as="p" size="sm" weight="regular" addlClassName="text-muted">
              No pending claimable balances.
            </Text>
          ) : (
            <ul className="flex flex-col gap-4">
              {pendingClaims.map((claim: PendingClaim) => (
                <li key={claim.id} className="border border-hi p-4 rounded-lg">
                  <div className="flex justify-between mb-2">
                    <Heading as="h3" size="xs" weight="bold">
                      {claim.employeeName}
                    </Heading>
                    <span className="bg-accent/20 text-accent px-2 py-1 rounded-full text-xs">
                      {claim.status}
                    </span>
                  </div>
                  <div className="text-sm text-muted flex justify-between items-center">
                    <div>
                      <Text as="p" size="xs" weight="regular">
                        Amount: {claim.amount} USDC
                      </Text>
                      <Text as="p" size="xs" weight="regular">
                        Scheduled: {formatDate(claim.dateScheduled)}
                      </Text>
                      <Text
                        as="p"
                        size="xs"
                        weight="regular"
                        addlClassName="font-mono truncate max-w-[200px]"
                        title={claim.claimantPublicKey}
                      >
                        To: {claim.claimantPublicKey}
                      </Text>
                    </div>
                    <button
                      onClick={() => handleRemoveClaim(claim.id)}
                      className="text-danger hover:text-danger/80 text-sm font-medium"
                    >
                      Cancel
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      <div className="w-full">
        <BulkPaymentStatusTracker organizationId={1} />
      </div>

      <TransactionPendingOverlay
        isVisible={overlayVisible}
        status={overlayStatus}
        txHash={overlayTxHash}
        onDismiss={() => setOverlayVisible(false)}
      />
    </div>
  );
}
