import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'happy-dom',
    include: ['src/lib/**/*.test.ts'],
    exclude: ['src/lib/marp-roundtrip.test.ts'],
  },
});