import React, { useCallback } from 'react';
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-react';

export interface PaginationControlsProps {
  /**
   * Current page number (1-indexed)
   */
  currentPage: number;
  /**
   * Total number of pages
   */
  totalPages: number;
  /**
   * Callback when page changes
   */
  onPageChange: (page: number) => void;
  /**
   * Number of visible page buttons (excluding prev/next)
   */
  maxVisiblePages?: number;
  /**
   * Show first/last page buttons
   */
  showFirstLast?: boolean;
  /**
   * Additional CSS classes
   */
  className?: string;
  /**
   * Disable pagination
   */
  disabled?: boolean;
}

export const PaginationControls: React.FC<PaginationControlsProps> = ({
  currentPage,
  totalPages,
  onPageChange,
  maxVisiblePages = 5,
  showFirstLast = true,
  className = '',
  disabled = false,
}) => {
  const getPageRange = useCallback(() => {
    const half = Math.floor(maxVisiblePages / 2);
    let start = Math.max(1, currentPage - half);
    let end = start + maxVisiblePages - 1;

    if (end > totalPages) {
      end = totalPages;
      start = Math.max(1, end - maxVisiblePages + 1);
    }

    return { start, end };
  }, [currentPage, maxVisiblePages, totalPages]);

  const { start, end } = getPageRange();
  const pages = Array.from({ length: end - start + 1 }, (_, i) => start + i);

  const handlePageChange = useCallback(
    (page: number) => {
      if (page >= 1 && page <= totalPages && !disabled) {
        onPageChange(page);
      }
    },
    [onPageChange, totalPages, disabled],
  );

  if (totalPages <= 1) {
    return null;
  }

  const buttonClass = (isActive: boolean) =>
    `px-3 py-2 rounded-lg border transition-colors font-medium text-sm focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 ${
      isActive
        ? 'bg-[var(--accent)] text-[var(--bg)] border-[var(--accent)]'
        : 'border-[var(--border-hi)] text-[var(--text)] hover:bg-[var(--surface)] disabled:opacity-50 disabled:cursor-not-allowed'
    }`;

  const iconButtonClass = `p-2 rounded-lg border border-[var(--border-hi)] text-[var(--text)] hover:bg-[var(--surface)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 disabled:opacity-50 disabled:cursor-not-allowed`;

  return (
    <nav
      className={`flex items-center justify-center gap-2 ${className}`}
      aria-label="Pagination"
    >
      {showFirstLast && (
        <button
          onClick={() => handlePageChange(1)}
          disabled={currentPage === 1 || disabled}
          className={iconButtonClass}
          title="Go to first page"
          aria-label="Go to first page"
          type="button"
        >
          <ChevronsLeft size={18} />
        </button>
      )}

      <button
        onClick={() => handlePageChange(currentPage - 1)}
        disabled={currentPage === 1 || disabled}
        className={iconButtonClass}
        title="Go to previous page"
        aria-label="Go to previous page"
        type="button"
      >
        <ChevronLeft size={18} />
      </button>

      <div className="flex gap-1 flex-wrap justify-center">
        {start > 1 && (
          <>
            <button
              onClick={() => handlePageChange(1)}
              className={buttonClass(false)}
              type="button"
            >
              1
            </button>
            {start > 2 && <span className="px-2 py-2 text-[var(--muted)]">…</span>}
          </>
        )}

        {pages.map((page) => (
          <button
            key={page}
            onClick={() => handlePageChange(page)}
            className={buttonClass(page === currentPage)}
            aria-label={`Go to page ${page}`}
            aria-current={page === currentPage ? 'page' : undefined}
            type="button"
          >
            {page}
          </button>
        ))}

        {end < totalPages && (
          <>
            {end < totalPages - 1 && <span className="px-2 py-2 text-[var(--muted)]">…</span>}
            <button
              onClick={() => handlePageChange(totalPages)}
              className={buttonClass(false)}
              type="button"
            >
              {totalPages}
            </button>
          </>
        )}
      </div>

      <button
        onClick={() => handlePageChange(currentPage + 1)}
        disabled={currentPage === totalPages || disabled}
        className={iconButtonClass}
        title="Go to next page"
        aria-label="Go to next page"
        type="button"
      >
        <ChevronRight size={18} />
      </button>

      {showFirstLast && (
        <button
          onClick={() => handlePageChange(totalPages)}
          disabled={currentPage === totalPages || disabled}
          className={iconButtonClass}
          title="Go to last page"
          aria-label="Go to last page"
          type="button"
        >
          <ChevronsRight size={18} />
        </button>
      )}

      <span className="ml-4 text-sm text-[var(--muted)]">
        Page <span className="font-semibold text-[var(--text)]">{currentPage}</span> of{' '}
        <span className="font-semibold text-[var(--text)]">{totalPages}</span>
      </span>
    </nav>
  );
};
