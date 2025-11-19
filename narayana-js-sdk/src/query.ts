// Query Builder - Elegant Fluent API

import { QueryOptions, WhereClause, OrderBy, Join } from './types';

export class QueryBuilder {
  private options: QueryOptions = {};

  select(columns: string[]): this {
    this.options.columns = columns;
    return this;
  }

  from(table: string): this {
    // Table is set at database/table level
    return this;
  }

  where(where: WhereClause | ((builder: WhereBuilder) => WhereBuilder)): this {
    if (typeof where === 'function') {
      const builder = new WhereBuilder();
      where(builder);
      this.options.where = builder.build();
    } else {
      this.options.where = where;
    }
    return this;
  }

  orderBy(column: string, direction: 'ASC' | 'DESC' = 'ASC'): this {
    if (!this.options.orderBy) {
      this.options.orderBy = [];
    }
    this.options.orderBy.push({ column, direction });
    return this;
  }

  groupBy(columns: string[]): this {
    this.options.groupBy = columns;
    return this;
  }

  having(having: WhereClause): this {
    this.options.having = having;
    return this;
  }

  join(table: string, on: { left: string; right: string }, type: 'INNER' | 'LEFT' | 'RIGHT' | 'FULL' = 'INNER'): this {
    if (!this.options.join) {
      this.options.join = [];
    }
    this.options.join.push({ table, on, type });
    return this;
  }

  limit(count: number): this {
    this.options.limit = count;
    return this;
  }

  offset(count: number): this {
    this.options.offset = count;
    return this;
  }

  build(): QueryOptions {
    return { ...this.options };
  }
}

export class WhereBuilder {
  private conditions: WhereClause[] = [];

  equals(column: string, value: any): this {
    this.conditions.push({ column, operator: '=', value });
    return this;
  }

  notEquals(column: string, value: any): this {
    this.conditions.push({ column, operator: '!=', value });
    return this;
  }

  greaterThan(column: string, value: any): this {
    this.conditions.push({ column, operator: '>', value });
    return this;
  }

  lessThan(column: string, value: any): this {
    this.conditions.push({ column, operator: '<', value });
    return this;
  }

  greaterThanOrEqual(column: string, value: any): this {
    this.conditions.push({ column, operator: '>=', value });
    return this;
  }

  lessThanOrEqual(column: string, value: any): this {
    this.conditions.push({ column, operator: '<=', value });
    return this;
  }

  in(column: string, values: any[]): this {
    this.conditions.push({ column, operator: 'IN', value: values });
    return this;
  }

  notIn(column: string, values: any[]): this {
    this.conditions.push({ column, operator: 'NOT IN', value: values });
    return this;
  }

  like(column: string, pattern: string): this {
    this.conditions.push({ column, operator: 'LIKE', value: pattern });
    return this;
  }

  isNull(column: string): this {
    this.conditions.push({ column, operator: 'IS NULL' });
    return this;
  }

  isNotNull(column: string): this {
    this.conditions.push({ column, operator: 'IS NOT NULL' });
    return this;
  }

  and(condition: (builder: WhereBuilder) => WhereBuilder): this {
    const builder = new WhereBuilder();
    condition(builder);
    const built = builder.build();
    if (built.and) {
      this.conditions.push(...built.and);
    } else {
      this.conditions.push(built);
    }
    return this;
  }

  or(condition: (builder: WhereBuilder) => WhereBuilder): this {
    const builder = new WhereBuilder();
    condition(builder);
    const built = builder.build();
    if (built.or) {
      this.conditions.push(...built.or);
    } else {
      this.conditions.push(built);
    }
    return this;
  }

  build(): WhereClause {
    if (this.conditions.length === 0) {
      throw new Error('Where clause cannot be empty');
    }
    if (this.conditions.length === 1) {
      return this.conditions[0];
    }
    return {
      column: '',
      operator: '=',
      and: this.conditions,
    };
  }
}

