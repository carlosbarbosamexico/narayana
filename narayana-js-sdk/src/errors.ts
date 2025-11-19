// Beautiful Error Types for NarayanaDB SDK

export class NarayanaError extends Error {
  constructor(
    message: string,
    public code: string,
    public statusCode?: number,
    public details?: any
  ) {
    super(message);
    this.name = 'NarayanaError';
    Object.setPrototypeOf(this, NarayanaError.prototype);
  }
}

export class ConnectionError extends NarayanaError {
  constructor(message: string, details?: any) {
    super(message, 'CONNECTION_ERROR', 0, details);
    this.name = 'ConnectionError';
  }
}

export class AuthenticationError extends NarayanaError {
  constructor(message: string = 'Authentication failed', details?: any) {
    super(message, 'AUTH_ERROR', 401, details);
    this.name = 'AuthenticationError';
  }
}

export class PermissionError extends NarayanaError {
  constructor(
    message: string = 'Permission denied',
    public resource: string,
    public requiredPermission: string,
    details?: any
  ) {
    super(message, 'PERMISSION_ERROR', 403, details);
    this.name = 'PermissionError';
  }
}

export class NotFoundError extends NarayanaError {
  constructor(resource: string, details?: any) {
    super(`Resource not found: ${resource}`, 'NOT_FOUND', 404, details);
    this.name = 'NotFoundError';
  }
}

export class ValidationError extends NarayanaError {
  constructor(message: string, public field?: string, details?: any) {
    super(message, 'VALIDATION_ERROR', 400, details);
    this.name = 'ValidationError';
  }
}

export class QueryError extends NarayanaError {
  constructor(message: string, public query?: string, details?: any) {
    super(message, 'QUERY_ERROR', 400, details);
    this.name = 'QueryError';
  }
}

export class TimeoutError extends NarayanaError {
  constructor(message: string = 'Operation timed out', details?: any) {
    super(message, 'TIMEOUT_ERROR', 408, details);
    this.name = 'TimeoutError';
  }
}

export class RateLimitError extends NarayanaError {
  constructor(
    message: string = 'Rate limit exceeded',
    public retryAfter?: number,
    details?: any
  ) {
    super(message, 'RATE_LIMIT_ERROR', 429, details);
    this.name = 'RateLimitError';
  }
}

export class ServerError extends NarayanaError {
  constructor(message: string, details?: any) {
    super(message, 'SERVER_ERROR', 500, details);
    this.name = 'ServerError';
  }
}

// Error helper
export function createError(error: any): NarayanaError {
  if (error instanceof NarayanaError) {
    return error;
  }

  if (error.response) {
    const status = error.response.status;
    const data = error.response.data;

    switch (status) {
      case 401:
        return new AuthenticationError(data?.message || 'Authentication failed', data);
      case 403:
        return new PermissionError(
          data?.message || 'Permission denied',
          data?.resource || '',
          data?.requiredPermission || '',
          data
        );
      case 404:
        return new NotFoundError(data?.resource || 'Resource', data);
      case 408:
        return new TimeoutError(data?.message || 'Operation timed out', data);
      case 429:
        return new RateLimitError(
          data?.message || 'Rate limit exceeded',
          data?.retryAfter,
          data
        );
      case 500:
      case 502:
      case 503:
      case 504:
        return new ServerError(data?.message || 'Server error', data);
      default:
        return new NarayanaError(
          data?.message || 'Unknown error',
          'UNKNOWN_ERROR',
          status,
          data
        );
    }
  }

  if (error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND') {
    return new ConnectionError('Failed to connect to NarayanaDB', error);
  }

  return new NarayanaError(
    error.message || 'Unknown error',
    'UNKNOWN_ERROR',
    undefined,
    error
  );
}

