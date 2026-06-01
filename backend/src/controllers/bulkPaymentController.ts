import { Request, Response } from 'express';
import { z } from 'zod';
import { Keypair } from '@stellar/stellar-sdk';
import { BulkPaymentService, MAX_OPS_PER_ENVELOPE } from '../services/bulkPaymentService.js';

const paymentItemSchema = z.object({
  destination: z.string().length(56, 'Stellar public key must be 56 characters'),
  amount: z
    .string()
    .regex(/^\d+(\.\d{1,7})?$/, 'Amount must be a positive decimal with up to 7 decimal places'),
  referenceId: z.string().max(255).optional(),
});

const bulkPaymentSchema = z.object({
  sourceSecret: z.string().min(56, 'Stellar secret key required'),
  payments: z
    .array(paymentItemSchema)
    .min(1, 'At least one payment is required')
    .max(MAX_OPS_PER_ENVELOPE * 10, `Maximum ${MAX_OPS_PER_ENVELOPE * 10} payments per request`),
  assetCode: z.string().max(12).optional().default('XLM'),
  assetIssuer: z.string().length(56).optional(),
  feeBumpFeePerOp: z.number().int().positive().optional(),
});

export class BulkPaymentController {
  /**
   * POST /api/v1/bulk-payments
   * Submit a batch of up to 1000 payments split across multiple Stellar
   * transaction envelopes (max 100 operations each).
   */
  static async submitBatch(req: Request, res: Response) {
    try {
      const parsed = bulkPaymentSchema.safeParse(req.body);
      if (!parsed.success) {
        return res.status(400).json({ error: 'Validation Error', details: parsed.error.issues });
      }

      const { sourceSecret, payments, assetCode, assetIssuer, feeBumpFeePerOp } = parsed.data;
      const organizationId = req.organizationId ?? req.user?.organizationId ?? null;

      if (!organizationId) {
        return res.status(403).json({ error: 'Organization context is required.' });
      }

      let sourceKeypair: Keypair;
      try {
        sourceKeypair = Keypair.fromSecret(sourceSecret);
      } catch {
        return res
          .status(400)
          .json({ error: 'Invalid sourceSecret: not a valid Stellar secret key.' });
      }

      if (assetCode !== 'XLM' && !assetIssuer) {
        return res.status(400).json({ error: 'assetIssuer is required for non-native assets.' });
      }

      const result = await BulkPaymentService.submitBatch(sourceKeypair, payments, {
        organizationId,
        assetCode,
        assetIssuer,
        feeBumpFeePerOp,
      });

      const statusCode = result.failedItems === 0 ? 200 : result.successfulItems === 0 ? 502 : 207;
      res.status(statusCode).json(result);
    } catch (error: any) {
      if (error instanceof z.ZodError) {
        return res.status(400).json({ error: 'Validation Error', details: error.issues });
      }
      res.status(500).json({ error: 'Bulk payment failed', message: error.message });
    }
  }

  /**
   * GET /api/v1/bulk-payments/:batchId
   * Retrieve the status and item-level details for a submitted batch.
   */
  static async getBatchStatus(req: Request, res: Response) {
    try {
      const { batchId } = req.params;
      const organizationId = req.organizationId ?? req.user?.organizationId ?? null;
      const result = await BulkPaymentService.getBatchStatus(batchId as string, organizationId ?? undefined);

      if (!result) {
        if (organizationId) {
          const existing = await BulkPaymentService.getBatchStatus(batchId as string);
          if (existing) {
            return res
              .status(403)
              .json({ error: 'Access denied: batch does not belong to your organization.' });
          }
        }

        return res.status(404).json({ error: 'Batch not found.' });
      }

      res.json(result);
    } catch (error: any) {
      res.status(500).json({ error: 'Failed to retrieve batch status', message: error.message });
    }
  }
}
