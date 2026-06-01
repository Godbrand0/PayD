import React from 'react';
import {
  ArrowUpRight,
  RefreshCw,
  Wallet,
  TrendingUp,
  Clock,
  CheckCircle2,
  Search,
  ExternalLink,
  DollarSign,
  Award,
  Receipt,
  AlertCircle,
  Gift,
  Info,
} from 'lucide-react';
import { useEmployeePortal, EmployeeTransaction } from '../hooks/useEmployeePortal';
import {
  formatCurrency,
  getSupportedCurrencies,
  getCurrencySymbol,
  getStellarExpertAccountLink,
} from '../services/currencyConversion';
import { claimService, ClaimableBalance } from '../services/claimableBalance';
import styles from './EmployeePortal.module.css';
import { useWallet } from '../hooks/useWallet';
import { useAuth } from '../providers/useAuth';

/* ── Pending Claims Section ──────── */
function PendingClaimsSection() {
  const { user } = useAuth();
  const [pendingClaims, setPendingClaims] = React.useState<ClaimableBalance[]>([]);
  const [isLoading, setIsLoading] = React.useState(true);
  const [showInstructions, setShowInstructions] = React.useState<Record<number, string>>({});

  React.useEffect(() => {
    const employeeId = Number(user?.id);
    if (!Number.isFinite(employeeId) || employeeId <= 0) {
      setIsLoading(false);
      return;
    }

    const loadClaims = async () => {
      try {
        const result = await claimService.getEmployeeClaims(employeeId, { limit: 100 });
        const pending = result.data.filter((c) => c.status === 'pending');
        setPendingClaims(pending);
      } catch (err) {
        console.error('Failed to load pending claims:', err);
      } finally {
        setIsLoading(false);
      }
    };

    void loadClaims();
  }, [user?.id]);

  const handleShowInstructions = async (claim: ClaimableBalance) => {
    const instructions = await claimService.generateClaimInstructions(
      claim.asset_code,
      claim.asset_issuer || undefined,
      claim.amount
    );
    setShowInstructions((prev) => ({ ...prev, [claim.id]: instructions }));
  };

  if (isLoading) {
    return null;
  }

  if (pendingClaims.length === 0) {
    return null;
  }

  return (
    <div className="p-4 rounded-xl bg-[rgba(255,213,0,0.08)] border border-[rgba(255,213,0,0.3)]">
      <div className="flex items-center gap-2 mb-4">
        <Gift className="w-5 h-5 text-[#ffd500]" />
        <h3 className="font-semibold text-[#ffd500]">Pending Claimable Balances</h3>
        <span className="px-2 py-0.5 text-xs rounded-full bg-[rgba(255,213,0,0.2)] text-[#ffd500]">
          {pendingClaims.length}
        </span>
      </div>

      <div className="space-y-3">
        {pendingClaims.map((claim) => (
          <div
            key={claim.id}
            className="p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-hi)]"
          >
            <div className="flex items-center justify-between mb-2">
              <div>
                <span className="text-lg font-semibold">
                  {claimService.formatClaimAmount(claim.amount, claim.asset_code)}
                </span>
                {claim.expires_at && (
                  <span className="ml-2 text-xs text-[var(--muted)]">
                    Expires: {new Date(claim.expires_at).toLocaleDateString()}
                  </span>
                )}
              </div>
              <span className="px-2 py-1 text-xs font-medium rounded-full bg-[rgba(255,213,0,0.2)] text-[#ffd500]">
                Pending Claim
              </span>
            </div>

            {showInstructions[claim.id] ? (
              <div className="mt-3 p-3 rounded bg-[var(--bg-base)] border border-[var(--border-hi)]">
                <pre className="text-xs whitespace-pre-wrap font-mono text-[var(--text-secondary)]">
                  {showInstructions[claim.id]}
                </pre>
              </div>
            ) : (
              <button
                onClick={() => void handleShowInstructions(claim)}
                className="mt-2 flex items-center gap-1.5 text-xs text-[var(--accent)] hover:underline"
              >
                <Info className="w-3.5 h-3.5" />
                How to claim this payment
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

/* ── Helper: status badge ────────── */
function StatusBadge({ status }: { status: EmployeeTransaction['status'] }) {
  const map = {
    completed: { cls: styles.statusCompleted, dot: styles.statusDotCompleted },
    pending: { cls: styles.statusPending, dot: styles.statusDotPending },
    failed: { cls: styles.statusFailed, dot: styles.statusDotFailed },
  };
  const { cls, dot } = map[status];
  return (
    <span className={`${styles.statusBadge} ${cls}`}>
      <span className={`${styles.statusDot} ${dot}`} />
      {status}
    </span>
  );
}

/* ── Helper: type badge ──────────── */
function TypeBadge({ type }: { type: EmployeeTransaction['type'] }) {
  const map = {
    salary: { cls: styles.txMemoTypeSalary, icon: <DollarSign className="w-3 h-3" /> },
    bonus: { cls: styles.txMemoTypeBonus, icon: <Award className="w-3 h-3" /> },
    reimbursement: { cls: styles.txMemoTypeReimbursement, icon: <Receipt className="w-3 h-3" /> },
  };
  const { cls } = map[type];
  return <span className={`${styles.txMemoType} ${cls}`}>{type}</span>;
}

/* ── Loading skeleton ────────────── */
function LoadingSkeleton() {
  return (
    <div>
      {['s1', 's2', 's3', 's4', 's5'].map((id) => (
        <div key={id} className={`${styles.skeleton} ${styles.skeletonRow}`} />
      ))}
    </div>
  );
}

/* ── Main Page Component ─────────── */
const EmployeePortal: React.FC = () => {
  const { address } = useWallet();
  const {
    transactions,
    balance,
    isLoading,
    error,
    selectedCurrency,
    setSelectedCurrency,
    refreshData,
    currentPage,
    setCurrentPage,
    totalPages,
    filterStatus,
    setFilterStatus,
    filterType,
    setFilterType,
    searchQuery,
    setSearchQuery,
  } = useEmployeePortal();

  const currencies = getSupportedCurrencies();

  // Calculate stats
  const totalReceived = balance?.orgUsd?.value || 0;
  const totalTransactions = transactions.length;
  const pendingCount = transactions.filter((t) => t.status === 'pending').length;
  const lastPayment = transactions.find((t) => t.status === 'completed');

  return (
    <div className="page-fade flex flex-col gap-6 max-w-[1200px] mx-auto w-full">
      {/* ── Page Header ─────────────── */}
      <div className={styles.pageHeader}>
        <div>
          <h1 className={styles.pageTitle}>
            My <span className="text-[var(--accent)]">Portal</span>
          </h1>
          <p className={styles.pageSubtitle}>
            View your salary payments, balances, and transaction history
          </p>
        </div>
        {address && (
          <a
            href={getStellarExpertAccountLink(address)}
            target="_blank"
            rel="noopener noreferrer"
            className={styles.walletBadge}
          >
            <span className={styles.walletDot} />
            {address.substring(0, 6)}...{address.substring(address.length - 4)}
            <ExternalLink className="w-3 h-3 opacity-60" />
          </a>
        )}
      </div>

      {/* ── Balance Card ─────────────── */}
      <div className={styles.balanceCard}>
        <div className="relative z-10">
          <p className={styles.balanceLabel}>Total Balance</p>
          <div className={styles.balanceAmountRow}>
            {isLoading ? (
              <div className={`${styles.skeleton}`} style={{ width: 200, height: 48 }} />
            ) : (
              <>
                <span className={styles.balanceAmount}>
                  {formatCurrency(balance?.orgUsd?.value || 0, 'USD')}
                </span>
                <span className={styles.localAmount}>
                  ≈ {formatCurrency(balance?.localAmount || 0, selectedCurrency)}
                </span>
              </>
            )}
          </div>

          <p className={styles.rateInfo}>
            1 ORGUSD ≈ {getCurrencySymbol(selectedCurrency)}
            {balance?.exchangeRate?.toLocaleString()} {selectedCurrency}
            {balance?.lastUpdated && <> · Updated {balance.lastUpdated.toLocaleTimeString()}</>}
          </p>

          <div className={styles.currencySelector}>
            <span className="text-[11px] text-[var(--muted)] uppercase tracking-widest font-semibold">
              Local Currency:
            </span>
            <select
              className={styles.currencySelect}
              value={selectedCurrency}
              onChange={(e) => setSelectedCurrency(e.target.value)}
            >
              {Object.entries(currencies).map(([code, name]) => (
                <option key={code} value={code}>
                  {code} — {name}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* ── Pending Claims Section ─────── */}
      <PendingClaimsSection />

      {/* ── Stats Cards ──────────────── */}
      <div className={styles.statsRow}>
        <div className={styles.statCard}>
          <div
            className={styles.statIcon}
            style={{
              background: 'rgba(74, 240, 184, 0.1)',
              border: '1px solid rgba(74, 240, 184, 0.2)',
            }}
          >
            <Wallet className="w-4 h-4 text-[var(--accent)]" />
          </div>
          <div className={styles.statValue}>{formatCurrency(totalReceived, 'USD')}</div>
          <div className={styles.statLabel}>Total Received</div>
        </div>

        <div className={styles.statCard}>
          <div
            className={styles.statIcon}
            style={{
              background: 'rgba(124, 111, 247, 0.1)',
              border: '1px solid rgba(124, 111, 247, 0.2)',
            }}
          >
            <TrendingUp className="w-4 h-4 text-[var(--accent2)]" />
          </div>
          <div className={styles.statValue}>{totalTransactions}</div>
          <div className={styles.statLabel}>Transactions</div>
        </div>

        <div className={styles.statCard}>
          <div
            className={styles.statIcon}
            style={{
              background: 'rgba(255, 213, 0, 0.1)',
              border: '1px solid rgba(255, 213, 0, 0.2)',
            }}
          >
            <Clock className="w-4 h-4 text-[#ffd500]" />
          </div>
          <div className={styles.statValue}>{pendingCount}</div>
          <div className={styles.statLabel}>Pending</div>
        </div>

        <div className={styles.statCard}>
          <div
            className={styles.statIcon}
            style={{
              background: 'rgba(63, 185, 80, 0.1)',
              border: '1px solid rgba(63, 185, 80, 0.2)',
            }}
          >
            <CheckCircle2 className="w-4 h-4 text-[var(--success)]" />
          </div>
          <div className={styles.statValue}>
            {lastPayment
              ? new Date(lastPayment.date).toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                })
              : '—'}
          </div>
          <div className={styles.statLabel}>Last Payment</div>
        </div>
      </div>

      {/* ── Error Banner ─────────────── */}
      {error && (
        <div className="flex items-center gap-3 p-4 rounded-xl bg-[rgba(255,123,114,0.08)] border border-[rgba(255,123,114,0.2)]">
          <AlertCircle className="w-5 h-5 text-[var(--danger)]" />
          <span className="text-sm text-[var(--danger)]">{error}</span>
        </div>
      )}

      {/* ── Transactions Table ────────── */}
      <div className={styles.txSection}>
        <div className={styles.txHeader}>
          <h2 className={styles.txTitle}>Payment History</h2>

          <div className={styles.txFilters}>
            <div className="relative">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted)]" />
              <input
                type="text"
                placeholder="Search tx hash, memo…"
                aria-label="Search payment history"
                className={styles.searchInput}
                style={{ paddingLeft: 28 }}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>

            <select
              className={styles.filterSelect}
              aria-label="Filter payment history by status"
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
            >
              <option value="all">All Status</option>
              <option value="completed">Completed</option>
              <option value="pending">Pending</option>
              <option value="failed">Failed</option>
            </select>

            <select
              className={styles.filterSelect}
              aria-label="Filter payment history by type"
              value={filterType}
              onChange={(e) => setFilterType(e.target.value)}
            >
              <option value="all">All Types</option>
              <option value="salary">Salary</option>
              <option value="bonus">Bonus</option>
              <option value="reimbursement">Reimbursement</option>
            </select>

            <button
              className={styles.refreshBtn}
              aria-label="Refresh payment history"
              onClick={() => void refreshData()}
              disabled={isLoading}
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? styles.refreshSpin : ''}`} />
              Refresh
            </button>
          </div>
        </div>

        {/* Table Header */}
        <div
          className={styles.txRow}
          style={{
            borderBottom: '1px solid var(--border-hi)',
            padding: '10px 24px',
          }}
        >
          <span className={styles.statLabel}>Date</span>
          <span className={styles.statLabel}>Description</span>
          <span className={styles.statLabel}>Amount</span>
          <span className={`${styles.statLabel} hidden md:block`}>Status</span>
          <span className={`${styles.statLabel} hidden md:block`}>Hash</span>
          <span className={styles.statLabel}>Verify</span>
        </div>

        {/* Rows */}
        {isLoading ? (
          <LoadingSkeleton />
        ) : transactions.length === 0 ? (
          <div className={styles.emptyState}>
            <Wallet className={styles.emptyIcon} />
            <p className={styles.emptyTitle}>No transactions found</p>
            <p className={styles.emptyDesc}>Try adjusting your filters or check back later.</p>
          </div>
        ) : (
          transactions.map((tx) => (
            <div key={tx.id} className={styles.txRow}>
              {/* Date */}
              <div>
                <div className={styles.txDate}>
                  {new Date(tx.date).toLocaleDateString('en-US', {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  })}
                </div>
                <div className={styles.txDateSub}>
                  {new Date(tx.date).toLocaleTimeString('en-US', {
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </div>
              </div>

              {/* Memo + Type */}
              <div>
                <span className={styles.txMemo}>{tx.memo}</span>
                <TypeBadge type={tx.type} />
              </div>

              {/* Amount */}
              <div>
                <div className={styles.txAmount}>+{formatCurrency(tx.amount, 'USD')}</div>
                <div className={styles.txAmountLocal}>
                  ≈ {formatCurrency(tx.amount * (balance?.exchangeRate || 1), selectedCurrency)}
                </div>
              </div>

              {/* Status */}
              <div className="hidden md:block">
                <StatusBadge status={tx.status} />
              </div>

              {/* Hash */}
              <div className="hidden md:block">
                <div className={styles.txHash}>
                  {tx.txHash.substring(0, 8)}…{tx.txHash.substring(tx.txHash.length - 6)}
                </div>
              </div>

              {/* Stellar Expert Link */}
              <div>
                <a
                  href={tx.stellarExpertUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={styles.stellarLink}
                  title="View on Stellar Expert"
                >
                  <ArrowUpRight className={styles.stellarLinkIcon} />
                  <span className="hidden sm:inline">Explorer</span>
                </a>
              </div>
            </div>
          ))
        )}

        {/* Pagination */}
        {totalPages > 1 && (
          <div className={styles.pagination}>
            <button
              className={styles.pageBtn}
              onClick={() => setCurrentPage(currentPage - 1)}
              disabled={currentPage <= 1}
              aria-label="Previous page"
            >
              ‹
            </button>
            {Array.from({ length: totalPages }, (_, i) => i + 1).map((p) => (
              <button
                key={p}
                className={`${styles.pageBtn} ${p === currentPage ? styles.pageBtnActive : ''}`}
                onClick={() => setCurrentPage(p)}
                aria-label={`Page ${p}`}
                aria-current={p === currentPage ? 'page' : undefined}
              >
                {p}
              </button>
            ))}
            <button
              className={styles.pageBtn}
              onClick={() => setCurrentPage(currentPage + 1)}
              disabled={currentPage >= totalPages}
              aria-label="Next page"
            >
              ›
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default EmployeePortal;
