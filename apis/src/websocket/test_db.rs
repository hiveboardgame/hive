// The same harness the `db` and `apis/tests` suites use, so these tests take the shared
// cross-process lock and start from a truncated database instead of racing those suites.
#![allow(dead_code)]
include!("../../../db/tests/common/test_db_support.rs");
