import { Router } from 'express';
import { BulkPaymentController } from '../controllers/bulkPaymentController.js';
import { authenticateJWT } from '../middlewares/auth.js';
import { authorizeRoles, isolateOrganization } from '../middlewares/rbac.js';
import { optionalIpWhitelist } from '../middlewares/ipWhitelist.js';

const router = Router();

/**
 * @swagger
 * tags:
 *   name: Bulk Payments
 *   description: Multi-operation Stellar transaction batching
 */

router.use(authenticateJWT);
router.use(authorizeRoles('EMPLOYER'));
router.use(isolateOrganization);
router.use(optionalIpWhitelist);

/**
 * @swagger
 * /api/v1/bulk-payments:
 *   post:
 *     summary: Submit a bulk payment batch
 *     description: >
 *       Batches up to 100 payment operations per Stellar transaction envelope.
 *       Handles fee bumping for high-traffic scenarios, manages sequence numbers
 *       across envelopes, and provides per-envelope rollback on partial failure.
 *     tags: [Bulk Payments]
 *     security:
 *       - bearerAuth: []
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required:
 *               - sourceSecret
 *               - payments
 *             properties:
 *               sourceSecret:
 *                 type: string
 *                 description: Stellar secret key of the source account
 *               payments:
 *                 type: array
 *                 items:
 *                   type: object
 *                   required:
 *                     - destination
 *                     - amount
 *                   properties:
 *                     destination:
 *                       type: string
 *                     amount:
 *                       type: string
 *                     referenceId:
 *                       type: string
 *               assetCode:
 *                 type: string
 *                 default: XLM
 *               assetIssuer:
 *                 type: string
 *               feeBumpFeePerOp:
 *                 type: integer
 *     responses:
 *       200:
 *         description: All envelopes submitted successfully
 *       207:
 *         description: Partial success - some envelopes failed
 *       400:
 *         description: Validation error
 *       502:
 *         description: All envelopes failed
 */
router.post('/', BulkPaymentController.submitBatch);

/**
 * @swagger
 * /api/v1/bulk-payments/{batchId}:
 *   get:
 *     summary: Get bulk payment batch status
 *     tags: [Bulk Payments]
 *     security:
 *       - bearerAuth: []
 *     parameters:
 *       - in: path
 *         name: batchId
 *         required: true
 *         schema:
 *           type: string
 *           format: uuid
 *     responses:
 *       200:
 *         description: Batch status and item details
 *       404:
 *         description: Batch not found
 */
router.get('/:batchId', BulkPaymentController.getBatchStatus);

export default router;
