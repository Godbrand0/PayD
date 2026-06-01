import { Router } from 'express';
import authenticateJWT from '../middlewares/auth.js';
import { authorizeRoles, isolateOrganization } from '../middlewares/rbac.js';
import { OrganizationController } from '../controllers/organizationController.js';

const router = Router();

// All org routes require authentication and EMPLOYER role
router.use(authenticateJWT);
router.use(authorizeRoles('EMPLOYER'));
router.use(isolateOrganization);

/**
 * @openapi
 * /api/v1/organizations/me:
 *   get:
 *     tags: [Organizations]
 *     summary: Get current organization profile
 *     security:
 *       - bearerAuth: []
 *     responses:
 *       200:
 *         description: Organization profile
 */
router.get('/me', OrganizationController.getMe);

/**
 * @openapi
 * /api/v1/organizations/me/name:
 *   patch:
 *     tags: [Organizations]
 *     summary: Update organization name
 *     security:
 *       - bearerAuth: []
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               name:
 *                 type: string
 *     responses:
 *       200:
 *         description: Updated organization name
 */
router.patch('/me/name', OrganizationController.updateName);

/**
 * @openapi
 * /api/v1/organizations/me/issuer:
 *   patch:
 *     tags: [Organizations]
 *     summary: Update organization Stellar issuer account
 *     security:
 *       - bearerAuth: []
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               issuerAccount:
 *                 type: string
 *                 description: Stellar public key (G...)
 *     responses:
 *       200:
 *         description: Updated issuer account
 */
router.patch('/me/issuer', OrganizationController.updateIssuer);

export default router;
