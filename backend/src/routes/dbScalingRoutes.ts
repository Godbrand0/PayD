import { Router } from 'express';
import { DbScalingController } from '../controllers/dbScalingController.js';

const router = Router();
const ctrl = new DbScalingController();

/**
 * @swagger
 * tags:
 *   name: DB Scaling
 *   description: Database connection pool and performance monitoring (Issue #260 Part 15)
 */

/**
 * @swagger
 * /api/v1/db-scaling/pool:
 *   get:
 *     summary: Get current database connection pool stats
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Pool stats returned successfully
 *       500:
 *         description: Internal server error
 */
router.get('/pool', (req, res, next) => ctrl.getPoolStats(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/health:
 *   get:
 *     summary: Database connectivity health check with latency
 *     tags: [DB Scaling]
 *     responses:
 *       200:
 *         description: Database is reachable
 *       503:
 *         description: Database is unreachable
 */
router.get('/health', (req, res, next) => ctrl.healthCheck(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/slow-queries:
 *   get:
 *     summary: List queries exceeding a mean execution time threshold
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     parameters:
 *       - in: query
 *         name: threshold
 *         schema:
 *           type: number
 *         description: Mean execution time threshold in ms (default 1000)
 *       - in: query
 *         name: limit
 *         schema:
 *           type: number
 *         description: Max rows to return (default 20, max 100)
 *     responses:
 *       200:
 *         description: Slow query list
 *       400:
 *         description: Invalid query parameters
 */
router.get('/slow-queries', (req, res, next) => ctrl.getSlowQueries(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/index-usage:
 *   get:
 *     summary: Return index usage statistics from pg_stat_user_indexes
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Index usage data
 */
router.get('/index-usage', (req, res, next) => ctrl.getIndexUsage(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/config:
 *   get:
 *     summary: Return current connection pool configuration
 *     tags: [DB Scaling]
 *     responses:
 *       200:
 *         description: Pool min/max configuration
 */
router.get('/config', (req, res) => ctrl.getPoolConfig(req, res));

// Issue #289 — table bloat
router.get('/table-bloat', (req, res, next) => ctrl.getTableBloat(req, res, next));

// Issue #290 — cache hit rate
router.get('/cache-hit-rate', (req, res, next) => ctrl.getCacheHitRate(req, res, next));

// Issue #291 — long-running transactions (?minDurationSec=10)
router.get('/long-running-transactions', (req, res, next) => ctrl.getLongRunningTransactions(req, res, next));

// Issue #292 — vacuum / analyse stats
router.get('/vacuum-stats', (req, res, next) => ctrl.getVacuumStats(req, res, next));

// ── Part 39 (#284) ──────────────────────────────────────────────────────────

/**
 * @swagger
 * /api/v1/db-scaling/lock-contention:
 *   get:
 *     summary: Active lock-wait chains between database backends (Part 39)
 *     description: >
 *       Returns rows from pg_locks + pg_stat_activity where one backend is
 *       blocking another. An empty array means no lock waits are active.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Lock contention rows (may be empty)
 *       500:
 *         description: Internal server error
 */
router.get('/lock-contention', (req, res, next) => ctrl.getLockContention(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/unused-indexes:
 *   get:
 *     summary: Indexes with zero scans since last statistics reset (Part 39)
 *     description: >
 *       Lists non-primary, non-unique indexes never used by the query planner.
 *       These are candidates for removal to reduce write overhead and storage.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Unused index list
 *       500:
 *         description: Internal server error
 */
router.get('/unused-indexes', (req, res, next) => ctrl.getUnusedIndexes(req, res, next));

// ── Part 40 (#285) ──────────────────────────────────────────────────────────

/**
 * @swagger
 * /api/v1/db-scaling/replication-lag:
 *   get:
 *     summary: Streaming replication lag per standby replica (Part 40)
 *     description: >
 *       Queries pg_stat_replication for LSN distances between primary and each
 *       connected standby. Returns an empty array when no replicas are configured.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Replication lag per replica (may be empty)
 *       500:
 *         description: Internal server error
 */
router.get('/replication-lag', (req, res, next) => ctrl.getReplicationLag(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/table-sizes:
 *   get:
 *     summary: Per-table disk usage including indexes and TOAST (Part 40)
 *     description: >
 *       Returns total on-disk size for each public-schema table broken down into
 *       heap, index, and TOAST segments. Ordered by total size descending.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     parameters:
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *           default: 30
 *           maximum: 100
 *         description: Maximum number of tables to return
 *     responses:
 *       200:
 *         description: Table size breakdown
 *       400:
 *         description: Invalid limit parameter
 *       500:
 *         description: Internal server error
 */
router.get('/table-sizes', (req, res, next) => ctrl.getTableSizes(req, res, next));

// ── Part 41 (#286) ──────────────────────────────────────────────────────────

/**
 * @swagger
 * /api/v1/db-scaling/bgwriter-stats:
 *   get:
 *     summary: Background writer and checkpoint activity statistics (Part 41)
 *     description: >
 *       Returns a single-row snapshot from pg_stat_bgwriter covering checkpoint
 *       frequency (timed vs requested), buffer write counts per writer, backend
 *       fsync calls, and checkpoint I/O durations. Useful for tuning
 *       checkpoint_completion_target and bgwriter_lru_maxpages.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: BGWriter stats snapshot
 *       500:
 *         description: Internal server error
 */
router.get('/bgwriter-stats', (req, res, next) => ctrl.getBgwriterStats(req, res, next));

/**
 * @swagger
 * /api/v1/db-scaling/database-stats:
 *   get:
 *     summary: Database-level statistics for the current database (Part 41)
 *     description: >
 *       Returns a snapshot from pg_stat_database for the active database,
 *       including transaction throughput (commits/rollbacks), overall buffer
 *       cache hit ratio, temporary file usage, deadlock count, and conflict
 *       count since the last stats reset.
 *     tags: [DB Scaling]
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Database-level stats snapshot
 *       500:
 *         description: Internal server error
 */
router.get('/database-stats', (req, res, next) => ctrl.getDatabaseStats(req, res, next));

export default router;
