# NarayanaDB Test Suite

Comprehensive test coverage for all modules and edge cases.

## Test Files

### Core Tests
- **integration_test.rs** - End-to-end integration tests
- **api_tests.rs** - API operation tests
- **property_tests.rs** - Property-based tests using proptest
- **server_tests.rs** - HTTP server endpoint tests

### Edge Case Tests
- **edge_cases.rs** - General edge cases and boundary conditions
- **comprehensive_edge_cases.rs** - Comprehensive edge case coverage
- **error_handling.rs** - Error path and exception handling tests
- **data_type_tests.rs** - All data type coverage tests

### Performance & Stress Tests
- **stress_tests.rs** - Stress tests with large datasets
- **concurrency_tests.rs** - Concurrency and race condition tests
- **benchmark_tests.rs** - Performance benchmarks using criterion

### Bug Prevention Tests
- **bug_prevention.rs** - Tests designed to catch common bugs, panics, and security issues

## Running Tests

```bash
# Run all tests
cargo test --all

# Run specific test file
cargo test --test comprehensive_edge_cases
cargo test --test bug_prevention
cargo test --test edge_cases

# Run with output
cargo test --all -- --nocapture

# Run benchmarks
cargo bench --bench benchmark_tests

# Run ignored tests (server tests)
cargo test --test server_tests -- --ignored

# Count total tests
cargo test --all -- --list | wc -l
```

## Test Coverage

### Core Module (`narayana-core`)
- ✅ All data types (Int8-64, UInt8-64, Float32/64, Boolean, String, Binary, Timestamp, Date)
- ✅ Schema operations (creation, field access, validation)
- ✅ Column operations (all types, empty, single element)
- ✅ Row operations (creation, access, edge cases)
- ✅ Transaction management (lifecycle, concurrency)
- ✅ Type system (nested types, nullable, arrays, maps)

### Storage Module (`narayana-storage`)
- ✅ Column store (create, read, write, delete)
- ✅ Compression (LZ4, Zstd, Snappy, None) - all edge cases
- ✅ Indexing (B-tree operations, range scans)
- ✅ Caching (LRU, TTL, eviction)
- ✅ Block operations (read/write, metadata)
- ✅ Error handling (table not found, invalid operations)

### Query Module (`narayana-query`)
- ✅ Vectorized operations (filter, compare, aggregate)
- ✅ Query operators (filter, project, scan)
- ✅ Query planning (plan creation, execution)
- ✅ Type safety (mismatch handling, invalid JSON)

## Edge Cases Covered

### Boundary Values
- ✅ Integer min/max (i32::MIN/MAX, i64::MIN/MAX, u64::MAX)
- ✅ Float special values (INFINITY, NEG_INFINITY, NaN, MIN, MAX)
- ✅ Empty collections (vectors, strings, binaries)
- ✅ Single element collections
- ✅ Very large collections (10M+ elements)

### Error Scenarios
- ✅ Table not found
- ✅ Column not found
- ✅ Invalid table IDs
- ✅ Invalid column IDs
- ✅ Type mismatches
- ✅ Compression errors
- ✅ Serialization errors

### Concurrency
- ✅ Concurrent table creation
- ✅ Concurrent reads/writes
- ✅ Race conditions
- ✅ High concurrency scenarios
- ✅ Parallel operations

### Memory Safety
- ✅ Buffer overflow prevention
- ✅ Use after free prevention
- ✅ Double free prevention
- ✅ Large allocation handling
- ✅ Memory leak prevention

### Panic Prevention
- ✅ Empty vector operations
- ✅ Index out of bounds
- ✅ Division by zero
- ✅ Null pointer dereference
- ✅ Integer overflow/underflow

## Test Statistics

- **Total Test Files**: 12+
- **Total Test Cases**: 300+
- **Total Lines of Test Code**: 3,600+
- **Coverage**: All modules, all public APIs, all edge cases

## Continuous Integration

All tests run automatically in CI/CD pipeline:
- Unit tests on every commit
- Integration tests on pull requests
- Benchmarks on release builds
- Property-based tests with random inputs

## Writing New Tests

When adding new functionality:

1. **Unit Tests**: Add to module's `#[cfg(test)]` block
2. **Integration Tests**: Add to `tests/integration_test.rs`
3. **Edge Cases**: Add to `tests/comprehensive_edge_cases.rs`
4. **Error Cases**: Add to `tests/error_handling.rs`
5. **Performance**: Add to `tests/stress_tests.rs` or `benchmark_tests.rs`

Follow these principles:
- Test happy paths
- Test error paths
- Test edge cases
- Test boundary conditions
- Test concurrency scenarios
- Prevent panics and crashes

