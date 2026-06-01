import { payrollWorker } from './payrollWorker.js';
import { notificationWorker } from './notificationWorker.js';
import { schedulerWorker } from './schedulerWorker.js';
import { transactionVerificationWorker } from './transactionVerificationWorker.js';
import { webhookNotificationService } from '../services/webhookNotificationService.js';
import logger from '../utils/logger.js';

export const startWorkers = () => {
  logger.info('Starting BullMQ workers...');

  // Workers are started when imported
  if (payrollWorker.isRunning()) {
    logger.info('Payroll worker is running');
  }

  logger.info('Notification worker initialized');
  logger.info('Transaction verification worker initialized');

  // Start polling for pending webhook retries
  setInterval(async () => {
    try {
      await webhookNotificationService.processPendingRetries();
    } catch (error) {
      logger.error('Error processing pending webhook retries', { error });
    }
  }, 60000); // Check every minute
};
