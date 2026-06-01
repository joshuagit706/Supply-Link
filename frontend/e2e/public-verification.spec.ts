import { test, expect } from '@playwright/test';
import { encodeQrProof } from '../lib/services/offlineVerify';

const TEST_PRODUCT_ID = 'CBYQQ3ZYMRV4EWCV235ZIJV5NBHM4QMCVPVTKN4CIMYCTBIHAHPNIXS';

test.describe('E2E: Public Verification without Wallet', () => {
  test('visit /verify/ directly without wallet → see product journey', async ({ page }) => {
    await page.goto(`/en/verify/${TEST_PRODUCT_ID}`);
    await expect(page.locator('h1')).toBeVisible({ timeout: 10000 });
    // No wallet prompt required
    const walletPrompt = page.locator('text=Connect Wallet to').first();
    expect(await walletPrompt.isVisible().catch(() => false)).toBe(false);
  });

  test('visit /verify/ with invalid product ID → see error message', async ({ page }) => {
    await page.goto('/en/verify/INVALID_PRODUCT_ID');
    await expect(page.locator('text=Product not found')).toBeVisible({ timeout: 5000 });
  });

  test('offline mode: valid QR proof token shows verified banner', async ({ page }) => {
    const token = await encodeQrProof({
      id: TEST_PRODUCT_ID,
      name: 'Test Coffee',
      origin: 'Ethiopia',
      owner: 'GTEST',
      ts: 1700000000000,
    });

    // Simulate offline by blocking all network requests after navigation
    await page.goto(`/en/verify/${TEST_PRODUCT_ID}?proof=${token}`);
    await page.evaluate(() => window.dispatchEvent(new Event('offline')));

    await expect(page.locator('[data-testid="offline-mode-banner"]')).toBeVisible({
      timeout: 5000,
    });
    await expect(page.locator('[data-testid="qr-proof-result"]')).toContainText(
      'QR proof verified',
      { timeout: 5000 },
    );
  });

  test('offline mode: tampered QR proof token shows failure', async ({ page }) => {
    await page.goto(`/en/verify/${TEST_PRODUCT_ID}?proof=invalid.token`);
    await page.evaluate(() => window.dispatchEvent(new Event('offline')));

    await expect(page.locator('[data-testid="qr-proof-result"]')).toContainText('invalid', {
      timeout: 5000,
    });
  });
});
