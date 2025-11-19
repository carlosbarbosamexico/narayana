// NarayanaDB JavaScript SDK - The Most Advanced SDK Ever
// Works directly from the browser with full type safety
// Plus advanced server-side Node.js package

export * from './client';
export * from './database';
export * from './server';

// Default export for browser usage
export { NarayanaClient as default } from './client';

// Server-side default export
export { NarayanaServerClient as ServerClient } from './server';
export * from './table';
export * from './query';
export * from './types';
export * from './auth';
export * from './permissions';
export * from './realtime';
export * from './streaming';
export * from './batch';
export * from './transaction';
export * from './search';
export * from './webhooks';
export * from './errors';
export * from './session';
export * from './cache';

// Default export
export { NarayanaClient as default } from './client';

